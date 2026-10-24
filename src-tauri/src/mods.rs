//! The Workshop mods on this PC, for the Mods tab and the server panel. They're read from Steam's own record of DayZ's
//! Workshop items (appworkshop_221100.acf) and the public Workshop API, so no Steam connection is needed. Updating and
//! unsubscribing go through the Steam helper (steam.rs).

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{api, dayz, join, vdf};

/// The app's record of its links, `@<name>` → Workshop id, kept in the links folder. DayZ's logs name a mod by its link
/// (`!zatlas\@<name>`), so this says which mod that was after the link itself is gone (unsubscribed, or removed by
/// another launcher).
const LINKS_RECORD: &str = "zatlas-links.json";

pub fn read_links_record(dayz: &Path) -> HashMap<String, u64> {
    fs::read_to_string(dayz.join(join::LINKS).join(LINKS_RECORD))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn write_links_record(dayz: &Path, record: &HashMap<String, u64>) {
    let _ = fs::write(
        dayz.join(join::LINKS).join(LINKS_RECORD),
        serde_json::to_string_pretty(record).unwrap(),
    );
}

/// What Steam's file on this PC says about a Workshop item.
struct LocalMod {
    /// on disk, in bytes
    size: u64,
    /// when the version on this PC was published (unix seconds)
    updated: u64,
    /// when the newest version was published, as far as Steam knows
    latest: u64,
    installed: bool,
    subscribed: bool,
}

/// Steam's record of DayZ's Workshop items: id → what it says. `WorkshopItemDetails` lists the subscribed items,
/// `WorkshopItemsInstalled` the downloaded ones.
fn local(dayz: &Path) -> HashMap<u64, LocalMod> {
    // <library>\steamapps\common\DayZ → <library>\steamapps\workshop
    let Some(dir) = dayz
        .parent()
        .and_then(Path::parent)
        .map(|steamapps| steamapps.join("workshop"))
    else {
        return HashMap::new();
    };
    let Ok(text) = fs::read_to_string(dir.join("appworkshop_221100.acf")) else {
        return HashMap::new();
    };
    let content = dir.join("content/221100");
    let acf = vdf::parse(&text);
    let Some(workshop) = acf.get("AppWorkshop") else {
        return HashMap::new();
    };
    let num = |item: &vdf::Vdf, key: &str| {
        item.get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    };
    let mut out: HashMap<u64, LocalMod> = HashMap::new();
    for (id, item) in workshop
        .get("WorkshopItemDetails")
        .map(|d| d.entries())
        .unwrap_or_default()
    {
        let Ok(id) = id.parse() else { continue };
        out.insert(
            id,
            LocalMod {
                size: 0,
                updated: num(item, "timeupdated"),
                latest: num(item, "latest_timeupdated"),
                installed: false,
                subscribed: num(item, "subscribedby") != 0,
            },
        );
    }
    for (id, item) in workshop
        .get("WorkshopItemsInstalled")
        .map(|d| d.entries())
        .unwrap_or_default()
    {
        let Ok(id) = id.parse::<u64>() else { continue };
        let entry = out.entry(id).or_insert(LocalMod {
            size: 0,
            updated: 0,
            latest: 0,
            installed: false,
            subscribed: false,
        });
        entry.size = num(item, "size");
        entry.updated = entry.updated.max(num(item, "timeupdated"));
        entry.installed = content.join(id.to_string()).is_dir();
    }
    out
}

/// A server's mod, as far as this PC is concerned (the server panel and the join dialog).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModState {
    pub id: u64,
    pub title: Option<String>,
    /// the download size from the Workshop
    pub size: u64,
    pub installed: bool,
    /// installed, but the Workshop has a newer version
    pub outdated: bool,
    /// public on the Workshop (also true when the Workshop couldn't be reached); a join skips the others
    pub on_workshop: bool,
}

pub fn states(ids: &[u64]) -> Vec<ModState> {
    let local = dayz::dayz_dir().map(|d| local(&d)).unwrap_or_default();
    let workshop = api::workshop(ids).ok();
    ids.iter()
        .map(|id| {
            let installed = local.get(id).filter(|l| l.installed);
            let w = workshop.as_ref().and_then(|w| w.get(id));
            ModState {
                id: *id,
                title: w.map(|w| w.title.clone()),
                size: w.map_or(0, |w| w.size),
                installed: installed.is_some(),
                outdated: matches!((installed, w), (Some(l), Some(w)) if w.updated > l.updated),
                on_workshop: workshop.is_none() || w.is_some(),
            }
        })
        .collect()
}

/// A row of the Mods tab.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopMod {
    pub id: u64,
    pub title: Option<String>,
    /// on disk when downloaded, else the Workshop's download size
    pub size: u64,
    pub installed: bool,
    pub subscribed: bool,
    /// when the version on this PC was published (unix seconds); 0 if none
    pub updated: u64,
    /// when the newest version was published, as far as Steam or the Workshop know
    pub latest: u64,
    /// downloaded, and a newer version exists
    pub outdated: bool,
    /// removed or private on the Workshop
    pub gone: bool,
    /// a `!zatlas` link points at it
    pub linked: bool,
    /// when DayZ last ran with it, from DayZ's logs (unix seconds); 0 if they don't say
    pub last_used: u64,
}

pub fn list() -> Result<Vec<WorkshopMod>, String> {
    let dayz = dayz::dayz_dir().ok_or("DayZ isn't installed (Steam's library has no DayZ_x64.exe).")?;
    let local = local(&dayz);
    let ids: Vec<u64> = local.keys().copied().collect::<BTreeSet<_>>().into_iter().collect();
    let web = api::workshop(&ids).ok();
    let linked = links(&dayz);
    // Link names to find a mod by in DayZ's logs when its link is gone: the app's record, then the Workshop titles (the
    // official launcher names its links after them).
    let mut names = read_links_record(&dayz);
    for (id, w) in web.iter().flatten() {
        names.entry(w.title.to_lowercase()).or_insert(*id);
        names
            .entry(join::link_name(&w.title, *id).to_lowercase())
            .or_insert(*id);
    }
    let used = used_from_logs(&dayz, &names);
    Ok(ids
        .iter()
        .map(|id| {
            let l = &local[id];
            let w = web.as_ref().and_then(|w| w.get(id));
            let latest = l.latest.max(w.map_or(0, |w| w.updated));
            WorkshopMod {
                id: *id,
                title: w.map(|w| w.title.clone()),
                size: if l.installed { l.size } else { w.map_or(0, |w| w.size) },
                installed: l.installed,
                subscribed: l.subscribed,
                updated: l.updated,
                latest,
                outdated: l.installed && latest > l.updated,
                gone: web.is_some() && w.is_none(),
                linked: linked.contains_key(id),
                last_used: used.get(id).copied().unwrap_or(0),
            }
        })
        .collect())
}

/// The `!zatlas` links: mod id → link.
fn links(dayz: &Path) -> HashMap<u64, PathBuf> {
    let Ok(dir) = fs::read_dir(dayz.join(join::LINKS)) else {
        return HashMap::new();
    };
    dir.flatten()
        .filter_map(|e| {
            let target = junction::get_target(e.path()).ok()?;
            Some((target.file_name()?.to_str()?.parse().ok()?, e.path()))
        })
        .collect()
}

/// When DayZ last ran with each mod: mod id → unix seconds. Each start of DayZ writes a log
/// (%LOCALAPPDATA%\DayZ\DayZ_x64_*.RPT) that begins with the command line, whose `-mod=` lists the mods, whichever
/// launcher started the game. The log's last change is when that game ended.
fn used_from_logs(dayz: &Path, names: &HashMap<String, u64>) -> HashMap<u64, u64> {
    let mut used: HashMap<u64, u64> = HashMap::new();
    let Some(dir) = crate::launch::log_dir() else {
        return used;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return used;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !(name.starts_with("DayZ_x64_") && name.ends_with(".RPT")) {
            continue;
        }
        let modified = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
        let Some(at) = modified.map(|d| d.as_secs()) else {
            continue;
        };
        let Some(line) = command_line(&e.path()) else {
            continue;
        };
        for id in mod_ids(&line, dayz, names) {
            let last = used.entry(id).or_insert(0);
            *last = (*last).max(at);
        }
    }
    used
}

/// The command line at the start of a log (`== "C:\…\DayZ_x64.exe" -mod=…`).
fn command_line(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut buf = vec![0; 16 * 1024];
    let n = fs::File::open(path).ok()?.read(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf[..n]).to_string();
    text.lines().find(|l| l.contains("-mod=")).map(String::from)
}

/// The Workshop ids of the mods in a command line's `-mod=` list.
fn mod_ids(line: &str, dayz: &Path, names: &HashMap<String, u64>) -> Vec<u64> {
    let Some(at) = line.find("-mod=") else {
        return vec![];
    };
    let rest = &line[at + "-mod=".len()..];
    // the list ends at the closing quote when it's quoted, else at the first space
    let quoted = line[..at].ends_with('"');
    let list = if quoted {
        rest.split('"').next()
    } else {
        rest.split_whitespace().next()
    }
    .unwrap_or("");
    // a relative folder is in DayZ's folder (joining an absolute path gives that path)
    list.split(';')
        .filter(|p| !p.is_empty())
        .filter_map(|p| mod_id(&dayz.join(p), names))
        .collect()
}

/// A mod folder's Workshop id, found one of three ways:
/// - it's a link (`!zatlas\@…`, or the official launcher's `!Workshop\@…`) to Steam's folder for the mod, which is
///   named by the id
/// - a part of its path is the id (`mods\2545327648`, `…\content\221100\2545327648`)
/// - its name without the `@` is in `names`, lowercase (a link that has been removed since)
fn mod_id(folder: &Path, names: &HashMap<String, u64>) -> Option<u64> {
    let number = |s: &str| s.parse::<u64>().ok();
    if let Some(id) = junction::get_target(folder)
        .ok()
        .and_then(|t| t.file_name()?.to_str().and_then(number))
    {
        return Some(id);
    }
    // Workshop ids are long numbers; shorter ones are something else
    let in_path = folder
        .components()
        .rev()
        .filter_map(|c| c.as_os_str().to_str())
        .find(|s| s.len() >= 6 && number(s).is_some());
    if let Some(id) = in_path.and_then(number) {
        return Some(id);
    }
    let name = folder.file_name()?.to_str()?.trim_start_matches('@').to_lowercase();
    names.get(&name).copied()
}

/// Removes the `!zatlas` links to these mods (after unsubscribing they'd point at nothing).
pub fn unlink(ids: &[u64]) {
    let Some(dayz) = dayz::dayz_dir() else { return };
    for (id, link) in links(&dayz) {
        if ids.contains(&id) && junction::exists(&link).unwrap_or(false) {
            let _ = fs::remove_dir(&link);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mod_ids;
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn ids_from_command_lines() {
        let dayz = Path::new(r"C:\nowhere\DayZ");
        let none = HashMap::new();
        // the format some launchers use: relative folders named by the Workshop id
        let line = r#"== "C:\Program Files (x86)\Steam\steamapps\common\DayZ\DayZ_x64.exe" -name Survivor -connect 1.2.3.4:2302 -password "" -mod=mods\2545327648;mods\3401182744;"#;
        assert_eq!(mod_ids(line, dayz, &none), vec![2545327648, 3401182744]);
        // quoted, with spaces; links that don't exist here and names that aren't ids give nothing
        let line = r#"== "C:\x\DayZ_x64.exe" "-mod=C:\nowhere\!zatlas\@Gauss Rifle;C:\Steam\steamapps\workshop\content\221100\1559212036" -connect=1.2.3.4:2302:2303"#;
        assert_eq!(mod_ids(line, dayz, &none), vec![1559212036]);
        assert!(
            mod_ids(
                r#"== "C:\x\DayZ_x64.exe" -mod= -connect=1.2.3.4:2302:2303"#,
                dayz,
                &none
            )
            .is_empty()
        );
        // a link that's gone, known by its name
        let names = HashMap::from([("gauss rifle".to_string(), 2307022759)]);
        assert_eq!(mod_ids(line, dayz, &names), vec![2307022759, 1559212036]);
    }
}
