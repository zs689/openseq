//! Application state: the currently opened graph plus the persisted config.

use std::path::PathBuf;

use logseq_core::Graph;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// Persisted, non-graph application configuration (small and JSON-shaped).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Path of the graph that was open when the app last closed.
    pub last_graph: Option<String>,
}

/// Everything the command layer needs, behind a single mutex.
#[derive(Default)]
pub struct AppState {
    pub graph: Option<Graph>,
    pub root: Option<PathBuf>,
    pub config: Config,
    /// Kept alive for as long as the graph is open.
    pub watcher: Option<notify::RecommendedWatcher>,
}

impl AppState {
    pub fn graph(&self) -> Result<&Graph, String> {
        self.graph
            .as_ref()
            .ok_or_else(|| "no graph is open".to_string())
    }

    pub fn graph_mut(&mut self) -> Result<&mut Graph, String> {
        self.graph
            .as_mut()
            .ok_or_else(|| "no graph is open".to_string())
    }
}

fn config_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("config.json"))
}

/// Load the config, falling back to defaults on any error.
pub fn load_config(app: &AppHandle) -> Config {
    let Some(path) = config_path(app) else {
        return Config::default();
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Best-effort persist; a failure here must never break the app.
pub fn save_config(app: &AppHandle, config: &Config) {
    let Some(path) = config_path(app) else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(&path, text);
    }
}
