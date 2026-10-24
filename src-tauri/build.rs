use std::path::{Path, PathBuf};

fn main() {
    copy_steam_api();
    tauri_build::build()
}

/// Copies Steam's API library (steam_api64.dll) into this folder. The app and its Steam helper load it from next to the
/// exe, and the installer takes it from here (tauri.conf.json, `resources`).
///
/// The steamworks-sys crate comes with the library, and its build script copies it to its own build output. On a clean
/// build that may not have run yet, so the copy in the crate's downloaded source is the fallback.
fn copy_steam_api() {
    match from_build_output().or_else(from_crate_source) {
        Some(dll) => {
            std::fs::copy(&dll, "steam_api64.dll").expect("copy steam_api64.dll");
        }
        None => println!("cargo:warning=steam_api64.dll not found in steamworks-sys"),
    }
}

/// `<target>\<profile>\build\steamworks-sys-<hash>\out\steam_api64.dll`
fn from_build_output() -> Option<PathBuf> {
    // OUT_DIR is <target>\<profile>\build\zatlas-<hash>\out
    let out = std::env::var("OUT_DIR").ok()?;
    let build = Path::new(&out).ancestors().nth(2)?;
    std::fs::read_dir(build).ok()?.flatten().find_map(|e| {
        let dll = e.path().join("out/steam_api64.dll");
        (e.file_name().to_string_lossy().starts_with("steamworks-sys-") && dll.is_file()).then_some(dll)
    })
}

/// `<Cargo home>\registry\src\<registry>\steamworks-sys-0.13.*\lib\steam\redistributable_bin\win64\steam_api64.dll`
fn from_crate_source() -> Option<PathBuf> {
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|home| Path::new(&home).join(".cargo")))?;
    let registries = std::fs::read_dir(cargo_home.join("registry/src")).ok()?;
    let mut crates = registries
        .flatten()
        .flat_map(|registry| std::fs::read_dir(registry.path()).into_iter().flatten().flatten());
    crates.find_map(|krate| {
        let dll = krate.path().join("lib/steam/redistributable_bin/win64/steam_api64.dll");
        (krate.file_name().to_string_lossy().starts_with("steamworks-sys-0.13") && dll.is_file()).then_some(dll)
    })
}
