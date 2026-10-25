//! The Friends tab: which Steam friends are online, in which game, and on which DayZ server when Steam knows it.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::Engine;
use serde::Serialize;
use steamworks::{FriendFlags, FriendState};

use crate::api;
use crate::steam::DAYZ;

/// Game names by app id, asked from the Steam store once per game, and when it didn't answer, again a while later.
static GAMES: Mutex<BTreeMap<u32, (Option<String>, Instant)>> = Mutex::new(BTreeMap::new());
const ASK_AGAIN: Duration = Duration::from_secs(300);

fn game_name(app: u32) -> Option<String> {
    if let Some((name, at)) = GAMES.lock().unwrap().get(&app)
        && (name.is_some() || at.elapsed() < ASK_AGAIN)
    {
        return name.clone();
    }
    let name = api::app_name(app);
    GAMES.lock().unwrap().insert(app, (name.clone(), Instant::now()));
    name
}

/// Steam's 64 px avatar (RGBA) as a PNG data URL.
fn avatar(rgba: Vec<u8>) -> Option<String> {
    const SIZE: u32 = 64;
    if rgba.len() != (SIZE * SIZE * 4) as usize {
        return None;
    }
    let mut out = Vec::new();
    let mut png = png::Encoder::new(&mut out, SIZE, SIZE);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header().ok()?.write_image_data(&rgba).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(out)
    ))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Friend {
    pub id: String,
    pub name: String,
    /// "online", "away", "busy" or "snooze"
    pub state: &'static str,
    /// the game they're in
    pub app: Option<u32>,
    /// the game's name, for a game other than DayZ
    pub game: Option<String>,
    pub in_dayz: bool,
    /// the server's IP and game port, when Steam gives it
    pub server: Option<String>,
    pub query_port: Option<u16>,
    /// DayZ's own status line (Steam's "rich presence"), if it sets one
    pub status: Option<String>,
    /// a PNG data URL
    pub avatar: Option<String>,
}

#[derive(Serialize)]
pub struct Friends {
    /// the user's own Steam name
    pub me: String,
    pub total: usize,
    pub online: Vec<Friend>,
}

/// Runs in the Steam helper (steam.rs), which is connected to Steam.
pub fn list(client: &steamworks::Client) -> Friends {
    let friends = client.friends();
    let all = friends.get_friends(FriendFlags::IMMEDIATE);
    // The steamworks crate can't ask for a friend's rich presence, so that goes through Steam's C API. The answer comes
    // a little later, so it shows on the next refresh.
    let steam_friends = unsafe { steamworks_sys::SteamAPI_SteamFriends_v018() };
    let mut online = Vec::new();
    for f in &all {
        let state = match f.state() {
            FriendState::Offline | FriendState::Invisible => continue,
            FriendState::Away => "away",
            FriendState::Busy => "busy",
            FriendState::Snooze => "snooze",
            _ => "online",
        };
        let game = f.game_played();
        let app = game.as_ref().map(|g| g.game.app_id().0);
        let in_dayz = app == Some(DAYZ);
        if in_dayz {
            unsafe { steamworks_sys::SteamAPI_ISteamFriends_RequestFriendRichPresence(steam_friends, f.id().raw()) };
        }
        let on_server = game
            .as_ref()
            .filter(|g| !g.game_address.is_unspecified() && g.game_port != 0);
        online.push(Friend {
            id: f.id().raw().to_string(),
            name: f.name(),
            state,
            app,
            game: app.filter(|a| *a != DAYZ).and_then(game_name),
            in_dayz,
            server: on_server.map(|g| format!("{}:{}", g.game_address, g.game_port)),
            query_port: on_server.map(|g| g.query_port),
            status: if in_dayz { f.rich_presence("status") } else { None },
            avatar: f.medium_avatar().and_then(avatar),
        });
    }
    // in DayZ first, then in another game, then the rest; by name within each
    online.sort_by_key(|f| (!f.in_dayz, f.app.is_none(), f.name.to_lowercase()));
    Friends {
        me: friends.name(),
        total: all.len(),
        online,
    }
}
