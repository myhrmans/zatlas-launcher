//! The web APIs: zatlas.app's servers API (the same list as zatlas.app/servers), and Steam's public Workshop and store
//! details. They're fetched here rather than in the window, so the browser's cross-origin rules don't get in the way.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::steam::DAYZ;

const SERVERS: &str = "https://zatlas.app/servers-api/api";
const WORKSHOP: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1/";

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(concat!("Zatlas-app/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// A server's id: its IP and game port.
pub fn valid_id(id: &str) -> bool {
    let Some((ip, port)) = id.split_once(':') else {
        return false;
    };
    ip.parse::<std::net::Ipv4Addr>().is_ok()
        && port.bytes().all(|b| b.is_ascii_digit())
        && !port.starts_with('0')
        && port.parse::<u16>().is_ok()
}

pub fn get(path: &str) -> Result<Value, String> {
    let url = format!("{SERVERS}{path}");
    let mut res = agent().get(&url).call().map_err(|e| match e {
        ureq::Error::StatusCode(404) => "That server isn't on the list any more.".to_string(),
        e => format!("zatlas.app didn't answer: {e}"),
    })?;
    res.body_mut()
        .read_json()
        .map_err(|e| format!("zatlas.app sent something unexpected: {e}"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub server: Server,
    pub mods: Vec<ModRef>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub name: String,
    pub ip: String,
    pub game_port: u16,
    pub query_port: Option<u16>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ModRef {
    pub id: u64,
    pub name: String,
}

/// A server's name, address and mods, for a join.
pub fn detail(id: &str) -> Result<Detail, String> {
    if !valid_id(id) {
        return Err(format!("{id} isn't a server address"));
    }
    // a join doesn't need the player history, so ask for the shortest
    serde_json::from_value(get(&format!("/servers/{id}?hours=1"))?).map_err(|e| e.to_string())
}

/// A Steam game's name from the store, for "In <game>" on the Friends tab.
pub fn app_name(app: u32) -> Option<String> {
    let url = format!("https://store.steampowered.com/api/appdetails?appids={app}&filters=basic");
    let body: Value = agent().get(&url).call().ok()?.body_mut().read_json().ok()?;
    body[app.to_string()]["data"]["name"].as_str().map(String::from)
}

/// What Steam's Workshop says about a mod: its title, download size and last update.
#[derive(Serialize, Clone)]
pub struct WorkshopInfo {
    pub title: String,
    pub size: u64,
    pub updated: u64,
}

/// The DayZ mods among `ids` that are public on the Workshop. An id left out isn't: it was removed, it's private, or it's
/// another game's.
pub fn workshop(ids: &[u64]) -> Result<HashMap<u64, WorkshopInfo>, String> {
    let mut out = HashMap::new();
    for chunk in ids.chunks(100) {
        let mut form = vec![("itemcount".to_string(), chunk.len().to_string())];
        for (i, id) in chunk.iter().enumerate() {
            form.push((format!("publishedfileids[{i}]"), id.to_string()));
        }
        let mut res = agent()
            .post(WORKSHOP)
            .send_form(form)
            .map_err(|e| format!("Steam's Workshop didn't answer: {e}"))?;
        let body: Value = res.body_mut().read_json().map_err(|e| e.to_string())?;
        let items = body["response"]["publishedfiledetails"].as_array();
        if items.is_none_or(|items| items.len() != chunk.len()) {
            return Err("Steam's Workshop sent an incomplete answer.".into());
        }
        for item in items.into_iter().flatten() {
            // numbers come as JSON numbers or as strings
            let num = |k: &str| {
                item[k]
                    .as_u64()
                    .or_else(|| item[k].as_str().and_then(|s| s.parse().ok()))
            };
            let Some(id) = num("publishedfileid") else {
                continue;
            };
            // result 1 means found, 9 removed and 15 private; anything else is a passing error
            match item["result"].as_u64() {
                Some(1) => {}
                Some(9 | 15) => continue,
                _ => return Err(format!("Steam's Workshop couldn't look up mod {id} just now.")),
            }
            if num("consumer_app_id") != Some(u64::from(DAYZ)) {
                continue;
            }
            out.insert(
                id,
                WorkshopInfo {
                    title: item["title"].as_str().unwrap_or_default().to_string(),
                    size: num("file_size").unwrap_or(0),
                    updated: num("time_updated").unwrap_or(0),
                },
            );
        }
    }
    Ok(out)
}
