//! The `invoke` command surface.
//!
//! The frontend owns *editing* (block tree mutations driven by caret and key
//! events); the backend owns *meaning* — parsing, serialising, indexing and
//! writing files. Every command therefore either reads the index or writes a
//! whole page.

use std::sync::Mutex;

use logseq_core::{Backlink, Graph, GraphData, Page, PageKind, PageMeta, Property, SearchHit};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::state::{save_config, AppState};
use crate::watcher;

type CmdResult<T> = Result<T, String>;

/// Summary returned after opening or creating a graph.
#[derive(Debug, Clone, Serialize)]
pub struct GraphInfo {
    pub root: String,
    pub page_count: usize,
}

fn info(graph: &Graph) -> GraphInfo {
    GraphInfo {
        root: graph.root().to_string_lossy().to_string(),
        page_count: graph.pages().len(),
    }
}

fn install_graph(app: &AppHandle, state: &mut AppState, root: String, graph: Graph) -> GraphInfo {
    let out = info(&graph);
    state.watcher = watcher::start(app.clone(), graph.root()).ok();
    state.root = Some(graph.root().to_path_buf());
    state.graph = Some(graph);
    state.config.last_graph = Some(root);
    save_config(app, &state.config);
    out
}

/// Open an existing graph folder.
#[tauri::command]
pub fn open_graph(
    app: AppHandle,
    path: String,
    state: State<'_, Mutex<AppState>>,
) -> CmdResult<GraphInfo> {
    let graph = Graph::open(&path).map_err(|e| e.to_string())?;
    let mut guard = state.lock().unwrap();
    Ok(install_graph(&app, &mut guard, path, graph))
}

/// Create (or open) a graph folder, seeding a welcome page when empty.
#[tauri::command]
pub fn create_graph(
    app: AppHandle,
    path: String,
    state: State<'_, Mutex<AppState>>,
) -> CmdResult<GraphInfo> {
    let graph = Graph::create(&path).map_err(|e| e.to_string())?;
    let mut guard = state.lock().unwrap();
    Ok(install_graph(&app, &mut guard, path, graph))
}

/// Re-read every file from disk.
#[tauri::command]
pub fn reload_graph(state: State<'_, Mutex<AppState>>) -> CmdResult<GraphInfo> {
    let mut guard = state.lock().unwrap();
    let graph = guard.graph_mut()?;
    graph.reload().map_err(|e| e.to_string())?;
    Ok(info(graph))
}

#[tauri::command]
pub fn current_graph(state: State<'_, Mutex<AppState>>) -> CmdResult<Option<GraphInfo>> {
    let guard = state.lock().unwrap();
    Ok(guard.graph.as_ref().map(info))
}

#[tauri::command]
pub fn list_pages(state: State<'_, Mutex<AppState>>) -> CmdResult<Vec<PageMeta>> {
    let guard = state.lock().unwrap();
    Ok(guard.graph()?.pages())
}

#[tauri::command]
pub fn get_page(name: String, state: State<'_, Mutex<AppState>>) -> CmdResult<Page> {
    let guard = state.lock().unwrap();
    guard
        .graph()?
        .page(&name)
        .cloned()
        .ok_or_else(|| format!("page not found: {name}"))
}

/// Fetch a page, creating it first when `create` is true.
#[tauri::command]
pub fn get_or_create_page(
    name: String,
    journal: Option<bool>,
    state: State<'_, Mutex<AppState>>,
) -> CmdResult<Page> {
    let mut guard = state.lock().unwrap();
    let kind = if journal.unwrap_or(false) {
        PageKind::Journal
    } else {
        PageKind::Page
    };
    guard
        .graph_mut()?
        .get_or_create_page(&name, kind)
        .map_err(|e| e.to_string())
}

/// Persist a whole page (the frontend sends the edited block tree).
#[tauri::command]
pub fn save_page(page: Page, state: State<'_, Mutex<AppState>>) -> CmdResult<Page> {
    let mut guard = state.lock().unwrap();
    guard
        .graph_mut()?
        .save_page(page)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_page(name: String, state: State<'_, Mutex<AppState>>) -> CmdResult<()> {
    let mut guard = state.lock().unwrap();
    guard
        .graph_mut()?
        .delete_page(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_page(from: String, to: String, state: State<'_, Mutex<AppState>>) -> CmdResult<Page> {
    let mut guard = state.lock().unwrap();
    guard
        .graph_mut()?
        .rename_page(&from, &to)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search(
    query: String,
    limit: Option<usize>,
    state: State<'_, Mutex<AppState>>,
) -> CmdResult<Vec<SearchHit>> {
    let guard = state.lock().unwrap();
    Ok(guard.graph()?.search(&query, limit.unwrap_or(50)))
}

#[tauri::command]
pub fn backlinks(name: String, state: State<'_, Mutex<AppState>>) -> CmdResult<Vec<Backlink>> {
    let guard = state.lock().unwrap();
    Ok(guard.graph()?.backlinks(&name))
}

#[tauri::command]
pub fn graph_data(state: State<'_, Mutex<AppState>>) -> CmdResult<GraphData> {
    let guard = state.lock().unwrap();
    Ok(guard.graph()?.graph_data())
}

/// Today's (or a given day's) journal page, created if needed.
#[tauri::command]
pub fn get_journal(date: String, state: State<'_, Mutex<AppState>>) -> CmdResult<Page> {
    let mut guard = state.lock().unwrap();
    guard
        .graph_mut()?
        .get_or_create_page(&date, PageKind::Journal)
        .map_err(|e| e.to_string())
}

/// Mark one block as persistent (`id::`) so `((block ref))` survives reloads.
#[tauri::command]
pub fn ensure_block_id(
    page: String,
    block_id: String,
    state: State<'_, Mutex<AppState>>,
) -> CmdResult<Page> {
    let mut guard = state.lock().unwrap();
    let graph = guard.graph_mut()?;
    let mut page_doc = graph
        .page(&page)
        .cloned()
        .ok_or_else(|| format!("page not found: {page}"))?;
    let block = page_doc
        .block_mut(&block_id)
        .ok_or_else(|| format!("block not found: {block_id}"))?;
    block.id_in_file = true;
    graph.save_page(page_doc).map_err(|e| e.to_string())
}

/// Absolute path of the open graph (for display in the UI).
#[tauri::command]
pub fn graph_root(state: State<'_, Mutex<AppState>>) -> CmdResult<Option<String>> {
    let guard = state.lock().unwrap();
    Ok(guard.root.as_ref().map(|p| p.to_string_lossy().to_string()))
}

/// List the `key:: value` properties of a page (used by the property editor).
#[tauri::command]
pub fn page_properties(
    name: String,
    state: State<'_, Mutex<AppState>>,
) -> CmdResult<Vec<Property>> {
    let guard = state.lock().unwrap();
    Ok(guard
        .graph()?
        .page(&name)
        .map(|p| p.properties.clone())
        .unwrap_or_default())
}

/// Used by the frontend to decide whether it is running inside Tauri at all.
#[tauri::command]
pub fn ping() -> &'static str {
    "openseq"
}
