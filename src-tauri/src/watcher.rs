//! Watches the graph folder so edits made outside the app show up inside it.

use std::path::Path;

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

/// Start a recursive watcher on `root`, emitting `graph-changed` with the list
/// of affected paths. The returned watcher must be kept alive by the caller.
pub fn start(app: AppHandle, root: &Path) -> notify::Result<RecommendedWatcher> {
    let handle = app.clone();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| match res {
        Ok(event) => {
            // Our atomic writes create a short-lived `.name.md.tmp`; ignore it
            // so we do not refresh twice for a single save.
            let paths: Vec<String> = event
                .paths
                .iter()
                .filter(|p| !p.to_string_lossy().contains(".tmp"))
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            if !paths.is_empty() {
                let _ = handle.emit("graph-changed", paths);
            }
        }
        Err(err) => {
            let _ = handle.emit("graph-error", err.to_string());
        }
    })?;

    watcher.watch(root, RecursiveMode::Recursive)?;
    Ok(watcher)
}
