//! Invite links: `zatlas://join/<ip:port>` opens that server's join dialog. The installer registers the zatlas:// link
//! type with Windows (tauri.conf.json, plugins.deep-link). A link opened while the app runs starts a second copy, which
//! hands the link over and ends (the single-instance plugin). The link people share is https://zatlas.app/join?s=<ip:port>,
//! a page that opens this one, since chat apps don't make zatlas:// links clickable.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

/// The invite that started the app, kept until the window asks for it (it isn't listening yet at the start).
#[derive(Default)]
pub struct Pending(pub Mutex<Option<String>>);

/// The server id in an invite link (`zatlas://join/1.2.3.4:2302`), when it is one.
pub fn server_id(url: &str) -> Option<String> {
    let id = url.strip_prefix("zatlas://join/")?.trim_end_matches('/');
    crate::api::valid_id(id).then(|| id.to_string())
}

/// Takes the invite that started the app, and sends each later one to the window as an `invite` event.
pub fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // A development build points the link type at itself; the installer registers it for the installed app.
    #[cfg(debug_assertions)]
    app.deep_link().register_all()?;
    let first = app
        .deep_link()
        .get_current()?
        .unwrap_or_default()
        .iter()
        .find_map(|url| server_id(url.as_str()));
    *app.state::<Pending>().0.lock().unwrap() = first;
    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        for id in event.urls().iter().filter_map(|url| server_id(url.as_str())) {
            let _ = handle.emit("invite", id);
        }
        show(&handle);
    });
    Ok(())
}

/// Brings the window to the front.
pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::server_id;

    #[test]
    fn server_ids_from_links() {
        assert_eq!(server_id("zatlas://join/1.2.3.4:2302").as_deref(), Some("1.2.3.4:2302"));
        assert_eq!(
            server_id("zatlas://join/1.2.3.4:2302/").as_deref(),
            Some("1.2.3.4:2302")
        );
        assert_eq!(server_id("zatlas://join/not-a-server"), None);
        assert_eq!(server_id("https://zatlas.app/join?s=1.2.3.4:2302"), None);
    }
}
