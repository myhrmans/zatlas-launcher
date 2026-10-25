//! Following DayZ as it starts after a join, so the window can show what it's doing, and say why when it goes wrong.
//! Every second it checks the processes (DayZ_BE.exe, DayZ_x64.exe, and whether the game's window responds) and the new
//! lines in DayZ's logs in %LOCALAPPDATA%\DayZ, and sends a `launch` event to the window.
//!
//! The log lines of a join that works, in order:
//! - RPT `Creating in minimized window mode`: the game window exists (about 20 s after the start)
//! - script log `Module: World; loaded …`: the game's and the mods' scripts are compiled
//! - RPT `NETWORK      : Network simulation`, then `Loading landscape 'nasdara'`: connected, loading the map
//! - script log `Creating Mission: mpmissions\__cur_mp.nasdara\mission.c`: in the server's mission, so in game
//!
//! `Creating Mission: …\scenes\intro…` instead means DayZ opened its main menu and didn't get in. A crash writes
//! `Minidump has been generated` in the RPT, and a crash_*.log with a `Reason:` line.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter};

use crate::dayz::{BE_EXE, GAME_EXE};

const TICK: Duration = Duration::from_secs(1);
/// Stop following after this long, whatever happens.
const GIVE_UP: Duration = Duration::from_secs(20 * 60);
/// A game window that hasn't responded for this long gets a warning (loading freezes it for shorter spells).
const HUNG: Duration = Duration::from_secs(20);
/// Seconds to wait for BattlEye to start the game before warning.
const SLOW_BATTLEYE: u64 = 40;
/// Seconds after which BattlEye having closed without starting the game counts as a failure.
const BATTLEYE_GONE: u64 = 10;
/// Seconds to keep following after DayZ opened its main menu, in case the player joins from there.
const MENU_WAIT: u64 = 120;

/// What the window gets each second.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Launch {
    pub id: String,
    pub name: String,
    /// seconds since the start
    pub elapsed: u64,
    /// "running", "ingame" or "failed"
    pub state: &'static str,
    /// when each step was reached, in seconds since the start
    pub steps: Vec<Step>,
    /// the map being loaded
    pub map: Option<String>,
    /// something worth knowing while it goes on (not responding, BattlEye slow)
    pub warning: Option<String>,
    /// why it failed
    pub problem: Option<String>,
    /// the crash log's reason line
    pub detail: Option<String>,
    pub game_running: bool,
}

#[derive(Serialize, Clone)]
pub struct Step {
    pub key: &'static str,
    pub at: Option<u64>,
}

const STEPS: [&str; 6] = ["battleye", "game", "window", "scripts", "connecting", "ingame"];

/// What the log lines have shown so far.
#[derive(Default, Debug, PartialEq)]
struct Marks {
    window: bool,
    scripts: bool,
    network: bool,
    map: Option<String>,
    intro: bool,
    mission: bool,
    minidump: bool,
}

fn scan(line: &str, m: &mut Marks) {
    if line.contains("Creating in minimized window mode") {
        m.window = true;
    } else if line.contains("Module: World;") {
        m.scripts = true;
    } else if line.contains("NETWORK") && line.contains("Network simulation") {
        m.network = true;
    } else if let Some(rest) = line.split("Loading landscape '").nth(1) {
        // the main menu loads a landscape too; only one loaded after connecting is the server's map
        if m.network {
            m.map = rest.split('\'').next().map(str::to_lowercase);
        }
    } else if let Some(mission) = line.split("Creating Mission:").nth(1) {
        let mission = mission.to_lowercase();
        if mission.contains("mpmissions") {
            m.mission = true;
        } else if mission.contains("intro") {
            m.intro = true;
        }
    } else if line.contains("Minidump has been generated") {
        m.minidump = true;
    }
}

/// Reads the lines added to a log file since the last call.
struct Tail {
    path: PathBuf,
    pos: u64,
    /// the start of a line that isn't finished yet
    rest: String,
}

impl Tail {
    fn new(path: PathBuf) -> Tail {
        Tail {
            path,
            pos: 0,
            rest: String::new(),
        }
    }

    fn lines(&mut self) -> Vec<String> {
        let Ok(mut f) = File::open(&self.path) else {
            return vec![];
        };
        let mut buf = Vec::new();
        if f.seek(SeekFrom::Start(self.pos)).is_err() || f.read_to_end(&mut buf).is_err() {
            return vec![];
        }
        self.pos += buf.len() as u64;
        let text = std::mem::take(&mut self.rest) + &String::from_utf8_lossy(&buf);
        let mut lines: Vec<String> = text.split('\n').map(|l| l.trim_end_matches('\r').to_string()).collect();
        self.rest = lines.pop().unwrap_or_default();
        lines
    }
}

pub fn log_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|d| Path::new(&d).join("DayZ"))
}

/// The newest file named `<prefix>…<ext>` in `dir` made since `since`.
fn newest(dir: &Path, prefix: &str, ext: &str, since: SystemTime) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with(prefix) && name.ends_with(ext)
        })
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let made = meta.created().or_else(|_| meta.modified()).ok()?;
            (made >= since).then_some((made, e.path()))
        })
        .max_by_key(|(made, _)| *made)
        .map(|(_, p)| p)
}

/// The `Reason:` line of the newest crash log since the start.
fn crash_reason(dir: &Path, since: SystemTime) -> Option<String> {
    let text = fs::read_to_string(newest(dir, "crash_", ".log", since)?).ok()?;
    text.lines()
        .find_map(|l| l.strip_prefix("Reason:"))
        .map(|r| r.trim().chars().take(300).collect())
}

/// Whether a visible window of process `pid` exists and has stopped responding.
fn hung(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsHungAppWindow, IsWindowVisible,
    };
    struct Search {
        pid: u32,
        hung: bool,
    }
    unsafe extern "system" fn each(hwnd: HWND, data: LPARAM) -> i32 {
        let search = unsafe { &mut *(data as *mut Search) };
        let mut owner = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
        if owner == search.pid && unsafe { IsWindowVisible(hwnd) } != 0 && unsafe { IsHungAppWindow(hwnd) } != 0 {
            search.hung = true;
            return 0; // found: stop looking
        }
        1
    }
    let mut search = Search { pid, hung: false };
    unsafe { EnumWindows(Some(each), &mut search as *mut Search as LPARAM) };
    search.hung
}

/// The number of the newest launch. A launch's watcher stops when a newer one starts, so it can't take the new game
/// for its own.
static NEWEST: AtomicU64 = AtomicU64::new(0);

/// Follows a launch for server `id` that was just started, on its own thread.
pub fn watch(app: AppHandle, id: String, name: String) {
    let number = NEWEST.fetch_add(1, Ordering::SeqCst) + 1;
    thread::spawn(move || follow(&app, id, name, number));
}

fn follow(app: &AppHandle, id: String, name: String, number: u64) {
    let start = Instant::now();
    // a little earlier than now, so logs made as the game starts count
    let since = SystemTime::now() - Duration::from_secs(3);
    let since_unix = since.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let dir = log_dir();
    let mut report = Launch {
        id,
        name,
        elapsed: 0,
        state: "running",
        steps: STEPS.iter().map(|k| Step { key: k, at: None }).collect(),
        map: None,
        warning: None,
        problem: None,
        detail: None,
        game_running: false,
    };
    // the join has just started BattlEye
    report.steps[0].at = Some(0);
    let mut marks = Marks::default();
    let (mut rpt, mut script): (Option<Tail>, Option<Tail>) = (None, None);
    let mut system = System::new();
    let mut seen_game = false;
    let mut hung_since: Option<Instant> = None;
    let mut menu_at: Option<u64> = None;

    loop {
        thread::sleep(TICK);
        if NEWEST.load(Ordering::SeqCst) != number {
            break;
        }
        let secs = start.elapsed().as_secs();
        report.elapsed = secs;
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        let battleye = system.processes_by_exact_name(BE_EXE.as_ref()).next().is_some();
        // the game started by this join, not one that was running before (start times are in whole seconds)
        let game = system
            .processes_by_exact_name(GAME_EXE.as_ref())
            .find(|p| p.start_time() + 2 >= since_unix)
            .map(|p| p.pid().as_u32());
        report.game_running = game.is_some();
        seen_game |= game.is_some();

        // the logs: DayZ makes a new RPT and script log each time it starts
        if let Some(dir) = &dir {
            if rpt.is_none() {
                rpt = newest(dir, "DayZ_x64_", ".RPT", since).map(Tail::new);
            }
            if script.is_none() {
                script = newest(dir, "script_", ".log", since).map(Tail::new);
            }
            for tail in [&mut rpt, &mut script].into_iter().flatten() {
                for line in tail.lines() {
                    scan(&line, &mut marks);
                }
            }
        }
        // each step gets the time it was first reached
        for (key, reached) in [
            ("game", game.is_some()),
            ("window", marks.window),
            ("scripts", marks.scripts),
            ("connecting", marks.network),
            ("ingame", marks.mission),
        ] {
            if reached {
                let step = report.steps.iter_mut().find(|s| s.key == key).unwrap();
                step.at.get_or_insert(secs);
            }
        }
        if marks.map.is_some() {
            report.map = marks.map.clone();
        }

        // a warning while it goes on
        report.warning = None;
        match game {
            Some(pid) if hung(pid) => {
                let hung_at = *hung_since.get_or_insert_with(Instant::now);
                if hung_at.elapsed() >= HUNG {
                    report.warning = Some(format!(
                        "DayZ hasn't responded for {} s. Loading can freeze it for a while; if it stays like this, close it and join again.",
                        hung_at.elapsed().as_secs()
                    ));
                }
            }
            Some(_) => hung_since = None,
            None if battleye && secs >= SLOW_BATTLEYE => {
                report.warning = Some(format!(
                    "Still waiting for BattlEye to start DayZ ({secs} s). A Windows prompt from BattlEye may be waiting behind other windows."
                ));
            }
            None => {}
        }

        // how it ended, if it has
        if marks.mission {
            report.state = "ingame";
            report.problem = None;
            report.detail = None;
        } else if marks.intro && menu_at.is_none() {
            menu_at = Some(secs);
            report.state = "failed";
            report.problem = Some(
                "DayZ opened its main menu instead of the server: it couldn't connect, or the server didn't let you in (full, whitelist, other mods or version). Join again, or pick the server in DayZ's own list."
                    .into(),
            );
        }
        if seen_game && game.is_none() && !marks.mission {
            report.state = "failed";
            report.detail = dir.as_deref().and_then(|d| crash_reason(d, since));
            if marks.minidump || report.detail.is_some() {
                report.problem = Some("DayZ crashed before it got into the server.".into());
            } else if report.problem.is_none() {
                report.problem = Some("DayZ closed before it got into the server.".into());
            }
        } else if !seen_game && !battleye && secs >= BATTLEYE_GONE {
            report.state = "failed";
            report.problem =
                Some("BattlEye closed without starting DayZ. Run Install_BattlEye.bat in DayZ's BattlEye folder as administrator, then join again.".into());
        }
        let _ = app.emit("launch", &report);

        // Stop when it's in game, or when it failed and the game is gone. At the main menu, wait a while first: the
        // player may still join from there.
        let menu_open_long = menu_at.is_some_and(|at| secs > at + MENU_WAIT);
        let done = match report.state {
            "ingame" => true,
            "failed" => game.is_none() || menu_open_long,
            _ => false,
        };
        if done || start.elapsed() > GIVE_UP {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(lines: &[&str]) -> Marks {
        let mut m = Marks::default();
        for l in lines {
            scan(l, &mut m);
        }
        m
    }

    #[test]
    fn a_join_that_works() {
        // lines from a real join's logs, the rest left out
        let m = read(&[
            "17:23:01.153 ENGINE    (W): Creating in minimized window mode",
            "17:23:09.270 SCRIPT       : Module: World; loaded 2322x files; 6186x classes; used 9631/33554 kB (28%) of static memory",
            "17:23:42.869 NETWORK      : Network simulation",
            "17:23:42.869  Loading landscape 'nasdara'",
            "17:23:44.902  SCRIPT       : Creating Mission: mpmissions\\__cur_mp.nasdara\\mission.c",
        ]);
        assert_eq!(
            m,
            Marks {
                window: true,
                scripts: true,
                network: true,
                map: Some("nasdara".into()),
                intro: false,
                mission: true,
                minidump: false
            }
        );
    }

    #[test]
    fn the_main_menu_instead_of_the_server() {
        // a start that never connected: the main menu's own landscape isn't the server's map
        let m = read(&[
            "17:36:57.169 Loading landscape 'ChernarusPlus'",
            "17:37:03.081 SCRIPT       : Creating Mission: dz\\Worlds\\ChernarusPlus\\data\\scenes\\intro.ChernarusPlus\\mission.c",
        ]);
        assert!(m.intro && !m.mission && m.map.is_none() && !m.network);
    }

    #[test]
    fn a_crash() {
        let m = read(&[
            "note: Minidump has been generated into the file C:\\Users\\x\\AppData\\Local\\DayZ\\DayZ_x64_2026-10-16_18-00-45.mdmp",
        ]);
        assert!(m.minidump);
    }
}
