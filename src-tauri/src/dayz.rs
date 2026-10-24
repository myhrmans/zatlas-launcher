//! Steam and DayZ on this PC, read from the registry, Steam's files and the process list, so the status bar needs no
//! Steam connection.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

pub const GAME_EXE: &str = "DayZ_x64.exe";
/// BattlEye's launcher, which starts the game
pub const BE_EXE: &str = "DayZ_BE.exe";

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub steam_running: bool,
    pub steam_signed_in: bool,
    pub dayz_dir: Option<String>,
    pub dayz_version: Option<String>,
    /// when DayZ started (unix seconds), while it runs
    pub dayz_started: Option<u64>,
}

fn steam_dir() -> Option<PathBuf> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Valve\\Steam")
        .ok()?;
    let path: String = key.get_value("SteamPath").ok()?;
    Some(PathBuf::from(path))
}

fn signed_in() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Valve\\Steam\\ActiveProcess")
        .and_then(|k| k.get_value::<u32, _>("ActiveUser"))
        .is_ok_and(|user| user != 0)
}

/// DayZ's folder: from its uninstall entry, else from Steam's list of libraries.
pub fn dayz_dir() -> Option<PathBuf> {
    let has_game = |p: &Path| p.join(GAME_EXE).is_file();
    for path in [
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App 221100",
        "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App 221100",
    ] {
        if let Ok(dir) = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(path)
            .and_then(|k| k.get_value::<String, _>("InstallLocation"))
        {
            let dir = PathBuf::from(dir);
            if has_game(&dir) {
                return Some(dir);
            }
        }
    }
    let libraries = fs::read_to_string(steam_dir()?.join("steamapps/libraryfolders.vdf")).ok()?;
    let vdf = crate::vdf::parse(&libraries);
    vdf.get("libraryfolders")?
        .entries()
        .iter()
        .filter_map(|(_, lib)| lib.get("path")?.as_str())
        .map(|p| Path::new(p).join("steamapps/common/DayZ"))
        .find(|d| has_game(d))
}

/// The game's version as servers report it ("1.30.164200"): the exe's product version without its last part.
fn version(dir: &Path) -> Option<String> {
    use pelite::pe64::{Pe, PeFile};
    let map = pelite::FileMap::open(&dir.join(GAME_EXE)).ok()?;
    let file = PeFile::from_bytes(&map).ok()?;
    let info = file.resources().ok()?.version_info().ok()?;
    let lang = *info.translation().first()?;
    let full = info.value(lang, "ProductVersion")?;
    Some(full.split('.').take(3).collect::<Vec<_>>().join("."))
}

/// Reads the [`Status`]. Kept between reads, so the process list is reused and the exe's version is read once per
/// install.
#[derive(Default)]
pub struct StatusReader {
    system: Option<System>,
    /// DayZ's folder and the version of the exe in it
    version: Option<(PathBuf, Option<String>)>,
}

impl StatusReader {
    pub fn read(&mut self) -> Status {
        let system = self.system.get_or_insert_with(System::new);
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        let started = |name: &str| {
            system
                .processes_by_exact_name(name.as_ref())
                .map(|p| p.start_time())
                .min()
        };
        let steam_running = started("steam.exe").is_some();
        let dayz_started = started(GAME_EXE);
        let dir = dayz_dir();
        let dayz_version = dir.as_ref().and_then(|d| {
            if self.version.as_ref().is_none_or(|(known, _)| known != d) {
                self.version = Some((d.clone(), version(d)));
            }
            self.version.as_ref().unwrap().1.clone()
        });
        Status {
            steam_running,
            steam_signed_in: steam_running && signed_in(),
            dayz_dir: dir.map(|d| d.display().to_string()),
            dayz_version,
            dayz_started,
        }
    }
}

/// Closes DayZ, for a join while it's running.
pub fn close_game() {
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    for p in system.processes_by_exact_name(GAME_EXE.as_ref()) {
        p.kill();
    }
}
