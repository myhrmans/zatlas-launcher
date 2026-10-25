//! The connection to Steam, kept in a separate helper process.
//!
//! The app connects to Steam as DayZ (app 221100): that is what lets it subscribe to DayZ's Workshop mods and see which
//! games friends are in. While it's connected, Steam shows the user as playing DayZ, and Steam only stops doing that when
//! the process that connected exits. So the connection lives in a helper process: this same exe, started with
//! [`SESSION_ARG`]. The app starts the helper when a join or the Friends tab needs Steam, and stops it [`IDLE`] after the
//! last request (never while a join holds it, see [`Steam::hold`]).
//!
//! The app and the helper talk over the helper's stdin and stdout: one JSON [`Request`] per line, and one JSON line back
//! for each, `{"ok": …}` or `{"err": "…"}`.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, TryLockError};
use std::thread;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use steamworks::{AppId, Client, ItemState, PublishedFileId};

pub const DAYZ: u32 = 221100;
/// The argument that starts this exe as the Steam helper.
pub const SESSION_ARG: &str = "--steam-session";
/// The helper is stopped when it hasn't been used for this long.
const IDLE: Duration = Duration::from_secs(30);
/// How long to wait for Steam to answer a subscribe or unsubscribe.
const SUBSCRIBE_WAIT: Duration = Duration::from_secs(60);

#[derive(Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    Friends,
    /// each item's state and download progress
    Items {
        ids: Vec<u64>,
    },
    /// subscribes and waits for Steam's answer
    Subscribe {
        id: u64,
    },
    /// unsubscribes and waits for Steam's answer; Steam deletes the files once DayZ (or the helper) isn't running
    Unsubscribe {
        id: u64,
    },
    Download {
        id: u64,
    },
    /// the folder Steam downloaded an item to
    Folder {
        id: u64,
    },
}

/// A Workshop item's state (Steam's `EItemState` flags) and its download progress in bytes.
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Item {
    pub state: u32,
    pub done: u64,
    pub total: u64,
}

impl Item {
    pub fn state(&self) -> ItemState {
        ItemState::from_bits_truncate(self.state)
    }
}

/// The app's side: starts the helper when needed and passes requests to it.
pub struct Steam {
    helper: Mutex<Option<Helper>>,
    holds: AtomicUsize,
}

struct Helper {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    used: Instant,
}

impl Drop for Helper {
    fn drop(&mut self) {
        // Steam notices the process is gone and stops showing DayZ as running.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// While a `Hold` exists, the helper keeps running even when idle. A join holds it while mods download.
pub struct Hold(Arc<Steam>);

impl Drop for Hold {
    fn drop(&mut self) {
        self.0.holds.fetch_sub(1, Ordering::SeqCst);
        if let Some(h) = self.0.helper.lock().unwrap().as_mut() {
            h.used = Instant::now();
        }
    }
}

impl Steam {
    /// No helper runs yet. A thread stops the helper whenever it has been idle for [`IDLE`].
    pub fn start() -> Arc<Steam> {
        let steam = Arc::new(Steam {
            helper: Mutex::new(None),
            holds: AtomicUsize::new(0),
        });
        let watcher = steam.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(1));
                let mut helper = watcher.helper.lock().unwrap();
                let held = watcher.holds.load(Ordering::SeqCst) > 0;
                if helper.as_ref().is_some_and(|h| !held && h.used.elapsed() > IDLE) {
                    *helper = None;
                }
            }
        });
        steam
    }

    /// Sends a request to the helper and waits for its answer, starting the helper first if it isn't running.
    pub fn call<T: DeserializeOwned>(&self, req: &Request) -> Result<T, String> {
        let mut slot = self.helper.lock().unwrap();
        if slot
            .as_mut()
            .is_some_and(|h| h.child.try_wait().ok().flatten().is_some())
        {
            // the helper has exited (Steam was closed, say): start a new one
            *slot = None;
        }
        if slot.is_none() {
            *slot = Some(spawn()?);
        }
        let h = slot.as_mut().unwrap();
        h.used = Instant::now();
        let line = serde_json::to_string(req).unwrap();
        let mut answer = String::new();
        let sent = writeln!(h.stdin, "{line}")
            .and_then(|_| h.stdin.flush())
            .and_then(|_| h.stdout.read_line(&mut answer));
        h.used = Instant::now();
        let answer: Value = match sent {
            Ok(n) if n > 0 => match serde_json::from_str(&answer) {
                Ok(answer) => answer,
                Err(e) => {
                    *slot = None;
                    return Err(format!("The Steam helper answered something unexpected: {e}"));
                }
            },
            _ => {
                *slot = None;
                return Err("The Steam helper stopped. Is Steam running?".into());
            }
        };
        if let Some(err) = answer.get("err").and_then(Value::as_str) {
            if answer.get("fatal").is_some() {
                // it couldn't connect to Steam and has exited
                *slot = None;
            }
            return Err(err.to_string());
        }
        serde_json::from_value(answer.get("ok").cloned().unwrap_or(Value::Null)).map_err(|e| e.to_string())
    }

    pub fn hold(self: &Arc<Self>) -> Hold {
        self.holds.fetch_add(1, Ordering::SeqCst);
        Hold(self.clone())
    }

    /// Whether the helper is running, so the app is connected to Steam as DayZ. While a request is under way it is.
    pub fn connected(&self) -> bool {
        match self.helper.try_lock() {
            Ok(mut helper) => helper
                .as_mut()
                .is_some_and(|h| h.child.try_wait().ok().flatten().is_none()),
            Err(TryLockError::WouldBlock) => true,
            Err(TryLockError::Poisoned(_)) => false,
        }
    }
}

fn spawn() -> Result<Helper, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut child = Command::new(exe)
        .arg(SESSION_ARG)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Couldn't start the Steam helper: {e}"))?;
    let stdin = child.stdin.take().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    Ok(Helper {
        child,
        stdin,
        stdout,
        used: Instant::now(),
    })
}

/// The helper process: connects to Steam as DayZ, then answers requests until its stdin closes.
pub fn serve() {
    let stdout = std::io::stdout();
    let reply = |v: Value| {
        let mut out = stdout.lock();
        let _ = writeln!(out, "{v}");
        let _ = out.flush();
    };
    let client = match Client::init_app(AppId(DAYZ)) {
        Ok(c) => c,
        Err(e) => {
            // answer the first request with the reason, then exit
            let mut line = String::new();
            let _ = std::io::stdin().read_line(&mut line);
            reply(
                json!({ "err": format!("Couldn't connect to Steam: {e}. Is Steam running and signed in?"), "fatal": true }),
            );
            return;
        }
    };
    // Steam delivers results (a subscription's, for one) through callbacks, which have to be run regularly.
    let callbacks = client.clone();
    thread::spawn(move || {
        loop {
            callbacks.run_callbacks();
            thread::sleep(Duration::from_millis(100));
        }
    });
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let answer = match serde_json::from_str::<Request>(&line) {
            Ok(req) => handle(&client, req),
            Err(e) => Err(e.to_string()),
        };
        reply(match answer {
            Ok(v) => json!({ "ok": v }),
            Err(e) => json!({ "err": e }),
        });
    }
}

/// Where a callback leaves Steam's answer for [`wait`].
type Answer = Arc<Mutex<Option<Result<(), steamworks::SteamError>>>>;

/// Waits for Steam's answer to a subscribe or unsubscribe.
fn wait(answer: &Answer) -> Result<(), String> {
    let start = Instant::now();
    loop {
        if let Some(r) = answer.lock().unwrap().take() {
            return r.map_err(|e| e.to_string());
        }
        if start.elapsed() > SUBSCRIBE_WAIT {
            return Err("Steam didn't answer. Is it online?".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn handle(client: &Client, req: Request) -> Result<Value, String> {
    let ugc = client.ugc();
    Ok(match req {
        Request::Friends => serde_json::to_value(crate::friends::list(client)).unwrap(),
        Request::Items { ids } => {
            let items: Vec<Item> = ids
                .iter()
                .map(|id| {
                    let item = PublishedFileId(*id);
                    let (done, total) = ugc.item_download_info(item).unwrap_or((0, 0));
                    Item {
                        state: ugc.item_state(item).bits(),
                        done,
                        total,
                    }
                })
                .collect();
            serde_json::to_value(items).unwrap()
        }
        Request::Subscribe { id } => {
            let answer = Answer::default();
            let slot = answer.clone();
            ugc.subscribe_item(PublishedFileId(id), move |r| *slot.lock().unwrap() = Some(r));
            wait(&answer)?;
            Value::Null
        }
        Request::Unsubscribe { id } => {
            let answer = Answer::default();
            let slot = answer.clone();
            ugc.unsubscribe_item(PublishedFileId(id), move |r| *slot.lock().unwrap() = Some(r));
            wait(&answer)?;
            Value::Null
        }
        Request::Download { id } => json!(ugc.download_item(PublishedFileId(id), true)),
        Request::Folder { id } => {
            json!(ugc.item_install_info(PublishedFileId(id)).map(|i| i.folder))
        }
    })
}
