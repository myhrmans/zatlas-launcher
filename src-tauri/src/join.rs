//! Joining a server: subscribe to its mods through Steam and wait for the downloads, link each mod's folder under
//! `<DayZ>\!zatlas`, and start DayZ straight into the server, the same way DZSA Launcher does:
//!
//! `DayZ_BE.exe 0 1 1 -exe DayZ_x64.exe "-mod=<links>" -connect=<ip>:<game port>:<query port>`
//!
//! A join to a full server can wait for a free slot first. A join runs on its own thread and reports its progress to
//! the window as `join` events.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use steamworks::ItemState;
use tauri::{AppHandle, Emitter, Manager, UserAttentionType};

use crate::a2s::{self, Slots};
use crate::api::{self, ModRef};
use crate::dayz::{self, BE_EXE, GAME_EXE};
use crate::mods;
use crate::settings::Settings;
use crate::steam::{Item, Request as SteamRequest, Steam};

/// How often the downloads are checked.
const TICK: Duration = Duration::from_millis(500);
/// A download that Steam hasn't started after this long is asked for again.
const ASK_AGAIN: Duration = Duration::from_secs(15);
/// How often a full server is asked for its player count while a join waits for a free slot.
const ROOM_CHECK: Duration = Duration::from_secs(5);
/// The folder inside DayZ's folder that holds the links to the mods.
pub const LINKS: &str = "!zatlas";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    /// the server's IP and game port
    pub id: String,
    pub password: Option<String>,
    /// close DayZ first if it's running
    pub close_running: bool,
    /// when the server is full, wait for a free slot before starting DayZ
    #[serde(default)]
    pub wait_for_room: bool,
}

/// A join's progress, as the window gets it.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub id: String,
    pub name: String,
    /// "mods", "waiting" (for a free slot), "starting", "started", "failed" or "cancelled"
    pub phase: &'static str,
    /// why it failed
    pub message: Option<String>,
    pub mods: Vec<ModProgress>,
    /// while waiting: the server's players and slots, or none when it didn't answer
    pub slots: Option<Slots>,
}

#[derive(Serialize, Clone)]
pub struct ModProgress {
    pub id: u64,
    pub name: String,
    /// "ready", "subscribing", "waiting", "downloading", or "skipped" (not on the Workshop: removed, private, or meant
    /// only for the server)
    pub state: &'static str,
    /// bytes downloaded, and in all
    pub done: u64,
    pub total: u64,
}

/// Why a join stopped before DayZ started.
enum Stop {
    Cancelled,
    Failed(String),
}

impl From<String> for Stop {
    fn from(e: String) -> Stop {
        Stop::Failed(e)
    }
}

impl From<&str> for Stop {
    fn from(e: &str) -> Stop {
        Stop::Failed(e.to_string())
    }
}

/// The join that is running, if any: only one runs at a time.
#[derive(Default)]
pub struct Joins {
    busy: AtomicBool,
    cancel: AtomicBool,
}

impl Joins {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Starts a join on its own thread. Fails at once only when another join is running.
    pub fn start(
        self: &Arc<Self>,
        app: AppHandle,
        steam: Arc<Steam>,
        settings: Settings,
        req: Request,
    ) -> Result<(), String> {
        if self.busy.swap(true, Ordering::SeqCst) {
            return Err("Another join is still running.".into());
        }
        self.cancel.store(false, Ordering::SeqCst);
        let joins = self.clone();
        thread::spawn(move || {
            let mut p = Progress {
                id: req.id.clone(),
                name: req.id.clone(),
                phase: "mods",
                message: None,
                mods: vec![],
                slots: None,
            };
            match run(&app, &steam, &settings, &req, &joins.cancel, &mut p) {
                Ok(()) => p.phase = "started",
                Err(Stop::Cancelled) => p.phase = "cancelled",
                Err(Stop::Failed(e)) => {
                    p.phase = "failed";
                    p.message = Some(e);
                }
            }
            emit(&app, &p);
            joins.busy.store(false, Ordering::SeqCst);
        });
        Ok(())
    }
}

fn emit(app: &AppHandle, p: &Progress) {
    let _ = app.emit("join", p);
}

fn run(
    app: &AppHandle,
    steam: &Arc<Steam>,
    settings: &Settings,
    req: &Request,
    cancel: &AtomicBool,
    p: &mut Progress,
) -> Result<(), Stop> {
    let detail = api::detail(&req.id)?;
    let server = &detail.server;
    p.name = server.name.clone();
    let query_port = server
        .query_port
        .ok_or("zatlas.app doesn't know this server's query port yet.")?;
    let dayz_dir = dayz::dayz_dir().ok_or("DayZ isn't installed (Steam's library has no DayZ_x64.exe).")?;

    // Servers often list mods the Workshop doesn't have (removed, private, or meant only for the server). They can't be
    // subscribed to and the game doesn't need them, so they're skipped. If the Workshop can't be reached, every mod is
    // tried.
    let ids: Vec<u64> = detail.mods.iter().map(|m| m.id).collect();
    let public = api::workshop(&ids).ok();
    let skip = |id: u64| public.as_ref().is_some_and(|w| !w.contains_key(&id));
    p.mods = detail
        .mods
        .iter()
        .map(|m| ModProgress {
            id: m.id,
            name: m.name.clone(),
            state: if skip(m.id) { "skipped" } else { "waiting" },
            done: 0,
            total: 0,
        })
        .collect();
    emit(app, p);
    let mods: Vec<&ModRef> = detail.mods.iter().filter(|m| !skip(m.id)).collect();

    let mut links = Vec::new();
    if !mods.is_empty() {
        let _hold = steam.hold();
        download(app, steam, &mods, cancel, p)?;
        links = link(steam, &dayz_dir, &mods)?;
    }

    if req.wait_for_room {
        wait_for_room(app, &server.ip, query_port, cancel, p)?;
    }

    p.phase = "starting";
    emit(app, p);
    let address = format!("{}:{}:{query_port}", server.ip, server.game_port);
    start_dayz(&dayz_dir, settings, req, &links, &address)?;
    crate::launch::watch(app.clone(), req.id.clone(), server.name.clone());
    Ok(())
}

/// Whether Steam has the mod downloaded and up to date.
fn is_ready(s: ItemState) -> bool {
    s.contains(ItemState::INSTALLED)
        && !s.intersects(ItemState::NEEDS_UPDATE | ItemState::DOWNLOADING | ItemState::DOWNLOAD_PENDING)
}

/// Subscribes to the mods that aren't subscribed yet, then waits until Steam has downloaded all of them.
fn download(
    app: &AppHandle,
    steam: &Steam,
    mods: &[&ModRef],
    cancel: &AtomicBool,
    p: &mut Progress,
) -> Result<(), Stop> {
    let ids: Vec<u64> = mods.iter().map(|m| m.id).collect();
    let items = || steam.call::<Vec<Item>>(&SteamRequest::Items { ids: ids.clone() });
    let cancelled = || cancel.load(Ordering::SeqCst);

    for (m, item) in mods.iter().zip(items()?) {
        if cancelled() {
            return Err(Stop::Cancelled);
        }
        if !item.state().contains(ItemState::SUBSCRIBED) {
            if let Some(pm) = p.mods.iter_mut().find(|x| x.id == m.id) {
                pm.state = "subscribing";
            }
            emit(app, p);
            steam
                .call::<()>(&SteamRequest::Subscribe { id: m.id })
                .map_err(|e| format!("Steam wouldn't subscribe to {}: {e}", m.name))?;
        }
    }

    // when each mod's download was last asked for
    let mut asked: HashMap<u64, Instant> = HashMap::new();
    loop {
        if cancelled() {
            return Err(Stop::Cancelled);
        }
        let mut all_ready = true;
        for (m, item) in mods.iter().zip(items()?) {
            let Some(pm) = p.mods.iter_mut().find(|x| x.id == m.id) else {
                continue;
            };
            let state = item.state();
            if is_ready(state) {
                pm.state = "ready";
                pm.done = pm.total;
                continue;
            }
            all_ready = false;
            // ask Steam for the download, and ask again if it still hasn't started a while later
            let started = state.intersects(ItemState::DOWNLOADING | ItemState::DOWNLOAD_PENDING);
            let ask = match asked.get(&m.id) {
                None => true,
                Some(at) => !started && at.elapsed() > ASK_AGAIN,
            };
            if ask {
                steam.call::<bool>(&SteamRequest::Download { id: m.id })?;
                asked.insert(m.id, Instant::now());
            }
            if item.total > 0 {
                (pm.done, pm.total) = (item.done, item.total);
            }
            pm.state = if state.contains(ItemState::DOWNLOADING) {
                "downloading"
            } else {
                "waiting"
            };
        }
        emit(app, p);
        if all_ready {
            return Ok(());
        }
        thread::sleep(TICK);
    }
}

/// Waits until the server has a free slot, asking it for its player count every few seconds, then flashes the app in
/// the taskbar. A server that doesn't answer is asked again.
fn wait_for_room(
    app: &AppHandle,
    ip: &str,
    query_port: u16,
    cancel: &AtomicBool,
    p: &mut Progress,
) -> Result<(), Stop> {
    p.phase = "waiting";
    loop {
        p.slots = a2s::slots(ip, query_port);
        emit(app, p);
        if p.slots.is_some_and(|s| s.free()) {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.request_user_attention(Some(UserAttentionType::Informational));
            }
            return Ok(());
        }
        // short naps, so Cancel stops the wait at once
        let asked = Instant::now();
        while asked.elapsed() < ROOM_CHECK {
            if cancel.load(Ordering::SeqCst) {
                return Err(Stop::Cancelled);
            }
            thread::sleep(Duration::from_millis(200));
        }
    }
}

/// Links each mod's Steam folder as `<DayZ>\!zatlas\@<mod name>` and returns the links, for `-mod=`. The link names are
/// also written to a record, so the Mods tab can still tell which mod a log meant after a link is gone.
fn link(steam: &Steam, dayz_dir: &Path, mods: &[&ModRef]) -> Result<Vec<String>, String> {
    let folder = dayz_dir.join(LINKS);
    fs::create_dir_all(&folder).map_err(|e| format!("Couldn't make {}: {e}", folder.display()))?;
    let mut record = mods::read_links_record(dayz_dir);
    let mut taken = HashSet::new();
    let mut links = Vec::new();
    for m in mods {
        let target: Option<String> = steam.call(&SteamRequest::Folder { id: m.id })?;
        let target = target.ok_or_else(|| format!("Steam gave no folder for {}", m.name))?;
        // two mods with the same name: the second one's id goes into its name
        let mut name = link_name(&m.name, m.id);
        if !taken.insert(name.to_lowercase()) {
            name = format!("{name} {}", m.id);
        }
        let link = folder.join(format!("@{name}"));
        relink(&link, Path::new(&target))?;
        links.push(link.display().to_string());
        record.insert(name.to_lowercase(), m.id);
    }
    mods::write_links_record(dayz_dir, &record);
    Ok(links)
}

/// A mod's link name: its name with only letters, digits, spaces, `-` and `_` kept (60 characters at most), or its
/// Workshop id when nothing is left.
pub fn link_name(name: &str, id: u64) -> String {
    let clean: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || " -_".contains(*c))
        .collect();
    let clean = clean.trim();
    if clean.is_empty() {
        id.to_string()
    } else {
        clean.chars().take(60).collect()
    }
}

/// Points the junction `link` at `target`. An older junction there is replaced (removing a junction leaves what it
/// points at alone); anything else in the way stops the join.
fn relink(link: &Path, target: &Path) -> Result<(), String> {
    if fs::symlink_metadata(link).is_ok() {
        if !junction::exists(link).unwrap_or(false) {
            return Err(format!("{} is in the way (not a link Zatlas made)", link.display()));
        }
        fs::remove_dir(link).map_err(|e| format!("Couldn't replace {}: {e}", link.display()))?;
    }
    junction::create(target, link).map_err(|e| format!("Couldn't link {}: {e}", link.display()))
}

/// Starts DayZ through BattlEye, straight into the server at `address` (IP, game port and query port).
fn start_dayz(
    dayz_dir: &Path,
    settings: &Settings,
    req: &Request,
    links: &[String],
    address: &str,
) -> Result<(), String> {
    let status = dayz::StatusReader::default().read();
    // Without Steam, DayZ restarts itself through Steam and seems to do nothing.
    if !status.steam_running || !status.steam_signed_in {
        return Err("Steam isn't running or isn't signed in. Start Steam and join again.".into());
    }
    if status.dayz_started.is_some() {
        if !req.close_running {
            return Err("DayZ is already running. Close it and join again.".into());
        }
        dayz::close_game();
        thread::sleep(Duration::from_secs(3));
    }
    // `0 1 1 -exe <game>` are BattlEye's own arguments, as DZSA Launcher passes them; the rest go to the game.
    let mut args: Vec<String> = ["0", "1", "1", "-exe", GAME_EXE].map(String::from).into();
    if !links.is_empty() {
        args.push(format!("-mod={}", links.join(";")));
    }
    if let Some(pw) = req.password.as_deref().filter(|pw| !pw.is_empty()) {
        args.push(format!("-password={pw}"));
    }
    if !settings.name.trim().is_empty() {
        args.push(format!("-name={}", settings.name.trim()));
    }
    args.extend(settings.params.split_whitespace().map(String::from));
    args.push(format!("-connect={address}"));
    Command::new(dayz_dir.join(BE_EXE))
        .args(&args)
        .current_dir(dayz_dir)
        .spawn()
        .map_err(|e| format!("Couldn't start DayZ: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::link_name;

    #[test]
    fn link_names() {
        assert_eq!(link_name("DayZ-Expansion-Core", 1), "DayZ-Expansion-Core");
        assert_eq!(link_name("SkyZ - Skybox Overhaul!", 1), "SkyZ - Skybox Overhaul");
        assert_eq!(link_name("★★", 42), "42");
    }
}
