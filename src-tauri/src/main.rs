//! Zatlas: a Windows app that joins DayZ servers with their mods installed, and shows which servers your Steam friends
//! are on. The window (src/) calls the commands in this file; a join reports its progress back through `join` and
//! `launch` events.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod a2s;
mod api;
mod dayz;
mod friends;
mod join;
mod launch;
mod mods;
mod ping;
mod settings;
mod steam;
mod vdf;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager, State};

use settings::Settings;
use steam::Steam;

/// What the commands share.
struct Core {
    steam: Arc<Steam>,
    status: Mutex<dayz::StatusReader>,
    joins: Arc<join::Joins>,
    settings: Mutex<Settings>,
    settings_path: PathBuf,
}

type Shared<'a> = State<'a, Arc<Core>>;

/// Runs blocking work (network, files, Steam) on a worker thread, so the window stays responsive.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusOut {
    #[serde(flatten)]
    local: dayz::Status,
    /// whether the app is connected to Steam as DayZ right now
    steam_connected: bool,
}

#[tauri::command]
async fn status(core: Shared<'_>) -> Result<StatusOut, String> {
    let core = core.inner().clone();
    blocking(move || {
        let local = core.status.lock().unwrap().read();
        Ok(StatusOut {
            local,
            steam_connected: core.steam.connected(),
        })
    })
    .await
}

/// The server list. `query` goes to the servers API as its query string (q, map, person, modded, minPlayers, ids, farm,
/// limit, offset), so only characters that are safe in a URL are accepted.
#[tauri::command]
async fn servers(query: String) -> Result<Value, String> {
    if !query
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "=&%._,:+-".contains(c))
    {
        return Err("bad query".into());
    }
    blocking(move || api::get(&format!("/servers?{query}"))).await
}

/// The maps and versions in the server list, and its totals.
#[tauri::command]
async fn facets() -> Result<Value, String> {
    blocking(|| api::get("/facets")).await
}

/// One server, with its mods and its player count over the last `hours` (1 to 336; the API keeps two weeks).
#[tauri::command]
async fn server(id: String, hours: Option<u32>) -> Result<Value, String> {
    if !api::valid_id(&id) {
        return Err(format!("{id} isn't a server address"));
    }
    let hours = hours.unwrap_or(24).clamp(1, 336);
    blocking(move || api::get(&format!("/servers/{id}?hours={hours}"))).await
}

/// Which of these mods are downloaded and up to date (no Steam connection needed).
#[tauri::command]
async fn mod_states(ids: Vec<u64>) -> Result<Vec<mods::ModState>, String> {
    blocking(move || Ok(mods::states(&ids))).await
}

/// Each server's ping from this PC.
#[tauri::command]
async fn ping(servers: Vec<ping::Target>) -> Result<Vec<ping::Ping>, String> {
    blocking(move || ping::ping(&servers)).await
}

#[tauri::command]
fn join(app: AppHandle, core: Shared<'_>, req: join::Request) -> Result<(), String> {
    let settings = core.settings.lock().unwrap().clone();
    core.joins.start(app, core.steam.clone(), settings, req)
}

#[tauri::command]
fn cancel_join(core: Shared<'_>) {
    core.joins.cancel();
}

/// The Mods tab's list (no Steam connection needed).
#[tauri::command]
async fn workshop_mods() -> Result<Vec<mods::WorkshopMod>, String> {
    blocking(mods::list).await
}

/// Each mod's state and download progress, from Steam.
#[tauri::command]
async fn mod_progress(core: Shared<'_>, ids: Vec<u64>) -> Result<Vec<steam::Item>, String> {
    let core = core.inner().clone();
    blocking(move || core.steam.call(&steam::Request::Items { ids })).await
}

/// Asks Steam to download the newest version of these mods now.
#[tauri::command]
async fn update_mods(core: Shared<'_>, ids: Vec<u64>) -> Result<(), String> {
    let core = core.inner().clone();
    blocking(move || {
        for id in ids {
            core.steam.call::<bool>(&steam::Request::Download { id })?;
        }
        Ok(())
    })
    .await
}

/// Subscribes to these mods again (the Mods tab's Undo).
#[tauri::command]
async fn subscribe_mods(core: Shared<'_>, ids: Vec<u64>) -> Result<(), String> {
    let core = core.inner().clone();
    blocking(move || {
        for id in ids {
            core.steam.call::<()>(&steam::Request::Subscribe { id })?;
        }
        Ok(())
    })
    .await
}

/// Unsubscribes from these mods (Steam then deletes their files) and removes the `!zatlas` links to them.
#[tauri::command]
async fn unsubscribe_mods(core: Shared<'_>, ids: Vec<u64>) -> Result<(), String> {
    let core = core.inner().clone();
    blocking(move || {
        for id in &ids {
            core.steam.call::<()>(&steam::Request::Unsubscribe { id: *id })?;
        }
        mods::unlink(&ids);
        Ok(())
    })
    .await
}

/// Opens DayZ's log folder (%LOCALAPPDATA%\DayZ) in Explorer.
#[tauri::command]
fn open_logs() -> Result<(), String> {
    let dir = launch::log_dir()
        .filter(|d| d.is_dir())
        .ok_or("DayZ hasn't written any logs yet.")?;
    std::process::Command::new("explorer")
        .arg(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn close_game() {
    dayz::close_game();
}

#[tauri::command]
async fn friends(core: Shared<'_>) -> Result<Value, String> {
    let core = core.inner().clone();
    blocking(move || core.steam.call(&steam::Request::Friends)).await
}

#[tauri::command]
fn get_settings(core: Shared<'_>) -> Settings {
    core.settings.lock().unwrap().clone()
}

#[tauri::command]
fn set_settings(core: Shared<'_>, settings: Settings) -> Result<(), String> {
    settings::save(&core.settings_path, &settings)?;
    *core.settings.lock().unwrap() = settings;
    Ok(())
}

fn main() {
    // The Steam helper is this same exe, started by the app with an argument (see steam.rs).
    if std::env::args().nth(1).as_deref() == Some(steam::SESSION_ARG) {
        steam::serve();
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let settings_path = settings::path(&app.path().app_config_dir()?);
            app.manage(Arc::new(Core {
                steam: Steam::start(),
                status: Mutex::default(),
                joins: Arc::default(),
                settings: Mutex::new(settings::load(&settings_path)),
                settings_path,
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            status,
            servers,
            facets,
            server,
            mod_states,
            ping,
            join,
            cancel_join,
            open_logs,
            close_game,
            friends,
            workshop_mods,
            mod_progress,
            update_mods,
            subscribe_mods,
            unsubscribe_mods,
            get_settings,
            set_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running Zatlas");
}
