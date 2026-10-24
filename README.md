<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="brand/zatlas-wordmark-on-dark.svg">
    <img src="brand/zatlas-wordmark-on-light.svg" alt="Zatlas" width="320">
  </picture>
</p>

# Zatlas app

A small Windows app for DayZ players, to go with [zatlas.app](https://zatlas.app):

- **Servers:** the same list as zatlas.app/servers, with search, filters and favourites. A server's panel shows its player count over time, its mods, and which of them you already have.
- **Join:** subscribes to the server's missing mods through Steam, waits for the downloads, and starts DayZ straight into the server with the mods loaded. While DayZ starts, it shows each step, and why when it fails.
- **Mods:** the Workshop mods on your PC, their size, updates and when you last used them. Update or unsubscribe from here.
- **Friends:** your Steam friends who are online, which game they're in, and for DayZ, which server when Steam knows it, with Join.

## How joining works

Like DZSA Launcher: the app connects to Steam as DayZ (app 221100) to subscribe to and download the Workshop mods, links each mod's Workshop folder under `<DayZ>\!zatlas\@<mod name>` (NTFS junctions, nothing is copied), and runs

```
DayZ_BE.exe 0 1 1 -exe DayZ_x64.exe "-mod=<the links>" -connect=<ip>:<game port>:<query port>
```

The server's mods and query port come from zatlas.app's servers API. Mod sizes and updates come from Steam's public Workshop API and Steam's own files, so browsing needs no Steam connection. While the app is connected to Steam (during a join's downloads, while the Friends tab is open, and while mods update), Steam shows you as playing DayZ.

## Build

Needs Rust, Node with pnpm, and Windows with WebView2 (built into Windows 11).

```
pnpm install
pnpm tauri dev      # run it
pnpm tauri build    # an installer in src-tauri/target/release/bundle/nsis/
```

Checks: `cargo test`, `cargo clippy` and `cargo fmt` in `src-tauri/`, and `pnpm check` for the window's code.

## The code

The window is Svelte (`src/`); everything that touches Steam, DayZ or the network is Rust (`src-tauri/src/`).

| File | What it does |
| --- | --- |
| `main.rs` | The commands the window calls |
| `join.rs` | A join: download the mods, link them, start DayZ |
| `launch.rs` | Follows DayZ as it starts, from its processes and logs |
| `steam.rs` | The Steam connection, in a helper process so Steam stops showing DayZ as running when it's done |
| `mods.rs` | The Workshop mods on this PC, from Steam's files and the Workshop API |
| `friends.rs` | Steam friends and the games and servers they're on |
| `dayz.rs` | Finds DayZ and Steam, and whether they're running |
| `api.rs` | zatlas.app's servers API and Steam's web APIs |
| `vdf.rs` | Reads Steam's text files (.vdf, .acf) |
| `settings.rs` | The launch settings |
