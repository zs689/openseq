//! The on-disk **graph**: a plain folder of markdown files.
//!
//! Layout (identical to Logseq's so an existing graph can be opened directly):
//!
//! ```text
//! my-graph/
//! ├── pages/        Foo.md, "a%2Fb".md, …
//! ├── journals/     2026_10_11.md, …
//! ├── assets/       images & attachments
//! └── logseq/       (optional, ignored by this build)
//! ```
//!
//! Nothing leaves the machine: the graph is just files, and the in-memory map
//! is only ever a cache derived from them.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};
use crate::model::{Block, Page, PageKind};
use crate::parser::parse_page;
use crate::refs;
use crate::search::{SearchHit, SearchIndex};
use crate::serializer::serialize_page;

pub const PAGES_DIR: &str = "pages";
pub const JOURNALS_DIR: &str = "journals";
pub const ASSETS_DIR: &str = "assets";

/// How deep below `pages/` and `journals/` we look for markdown files.
/// Namespaced pages use at most a couple of levels; this keeps a stray deep
/// tree from turning into a long scan.
const MAX_SCAN_DEPTH: usize = 4;

/// Collect `.md` files under `dir`, recursively but depth-limited.
fn collect_markdown(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return; // unreadable directory: treat as empty rather than failing the load
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, out, depth - 1);
        } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

/// Lightweight page descriptor for the sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMeta {
    pub name: String,
    pub kind: PageKind,
    pub title: String,
    pub block_count: usize,
}

/// A block on another page that links to the page you are looking at.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backlink {
    pub page: String,
    pub block_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: PageKind,
    pub size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}

/// Nodes + edges for the graph view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// An opened graph directory and its in-memory index.
pub struct Graph {
    root: PathBuf,
    pages: BTreeMap<String, Page>,
    index: SearchIndex,
}

impl Graph {
    /// Open an existing graph directory. Fails if it does not exist.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        if !root.is_dir() {
            return Err(CoreError::GraphMissing(root));
        }
        let mut graph = Graph {
            root,
            pages: BTreeMap::new(),
            index: SearchIndex::default(),
        };
        graph.ensure_dirs()?;
        graph.reload()?;
        Ok(graph)
    }

    /// Open a graph, creating it (with a welcome page) when it is empty.
    pub fn create(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|e| CoreError::io(&root, e))?;
        let mut graph = Graph {
            root,
            pages: BTreeMap::new(),
            index: SearchIndex::default(),
        };
        graph.ensure_dirs()?;
        graph.reload()?;
        if graph.pages.is_empty() {
            graph.seed()?;
        }
        Ok(graph)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Re-read every markdown file from disk.
    pub fn reload(&mut self) -> Result<()> {
        self.pages.clear();

        let mut loaded: Vec<Page> = Vec::new();
        for (dir, kind) in [
            (PAGES_DIR, PageKind::Page),
            (JOURNALS_DIR, PageKind::Journal),
        ] {
            let base = self.root.join(dir);
            if !base.is_dir() {
                continue;
            }
            let mut files = Vec::new();
            collect_markdown(&base, &mut files, MAX_SCAN_DEPTH);
            files.sort(); // deterministic load order across platforms

            for path in files {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let name = match kind {
                    PageKind::Journal => journal_file_to_name(stem),
                    PageKind::Page => file_to_page_name(stem),
                };
                let rel = path.strip_prefix(&self.root).unwrap_or(&path);
                let rel = rel.to_string_lossy().replace('\\', "/");
                let src = fs::read_to_string(&path).map_err(|e| CoreError::io(&path, e))?;
                loaded.push(parse_page(&name, kind, &rel, &src));
            }
        }

        for page in loaded {
            self.index.add_page(&page);
            self.pages.insert(normalize(&page.name), page);
        }
        Ok(())
    }

    /// Every page, sorted by name.
    pub fn pages(&self) -> Vec<PageMeta> {
        self.pages
            .values()
            .map(|p| PageMeta {
                name: p.name.clone(),
                kind: p.kind,
                title: p.display_title(),
                block_count: p.all_blocks().len(),
            })
            .collect()
    }

    pub fn page(&self, name: &str) -> Option<&Page> {
        self.pages.get(&normalize(name))
    }

    pub fn page_mut(&mut self, name: &str) -> Option<&mut Page> {
        self.pages.get_mut(&normalize(name))
    }

    /// Whether a page exists (case-insensitively).
    pub fn has_page(&self, name: &str) -> bool {
        self.pages.contains_key(&normalize(name))
    }

    /// Create a page if missing, then return a clone of it.
    pub fn get_or_create_page(&mut self, name: &str, kind: PageKind) -> Result<Page> {
        if let Some(page) = self.page(name) {
            return Ok(page.clone());
        }
        let mut page = Page::new(name, kind);
        if kind == PageKind::Journal {
            page.set_property("title", name);
        }
        let page = self.save_page(page)?;
        Ok(page)
    }

    /// Write a page to disk and refresh the index. Returns the stored page
    /// (with `path` filled in).
    pub fn save_page(&mut self, mut page: Page) -> Result<Page> {
        let path = self.page_path(&page);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, e))?;
        }
        let rel = path
            .strip_prefix(&self.root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        page.path = rel;

        let text = serialize_page(&page);
        write_atomic(&path, &text)?;

        self.pages.insert(normalize(&page.name), page.clone());
        self.reindex();
        Ok(page)
    }

    /// Delete a page from memory and from disk.
    pub fn delete_page(&mut self, name: &str) -> Result<()> {
        let key = normalize(name);
        let page = self
            .pages
            .remove(&key)
            .ok_or_else(|| CoreError::PageNotFound(name.to_string()))?;
        let path = self.page_path(&page);
        if path.is_file() {
            fs::remove_file(&path).map_err(|e| CoreError::io(&path, e))?;
        }
        self.reindex();
        Ok(())
    }

    /// Rename a page (and its file).
    pub fn rename_page(&mut self, from: &str, to: &str) -> Result<Page> {
        let mut page = self
            .page(from)
            .cloned()
            .ok_or_else(|| CoreError::PageNotFound(from.to_string()))?;
        let old_path = self.page_path(&page);
        page.name = to.to_string();
        let new_page = self.save_page(page)?;
        if old_path.is_file() {
            let _ = fs::remove_file(&old_path);
        }
        Ok(new_page)
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        self.index.search(query, limit.max(1))
    }

    /// Blocks elsewhere in the graph that link to `name`.
    pub fn backlinks(&self, name: &str) -> Vec<Backlink> {
        let needle = normalize(name);
        let mut out: Vec<Backlink> = Vec::new();
        for page in self.pages.values() {
            if normalize(&page.name) == needle {
                continue; // references from the page itself are shown inline
            }
            for block in page.all_blocks() {
                if refs::extract_page_refs(&block.content)
                    .iter()
                    .any(|r| normalize(r) == needle)
                {
                    out.push(Backlink {
                        page: page.name.clone(),
                        block_id: block.id.clone(),
                        text: block.plain_text(),
                    });
                }
            }
        }
        out.sort_by(|a, b| a.page.cmp(&b.page));
        out
    }

    /// Build the node/edge set for the graph view.
    pub fn graph_data(&self) -> GraphData {
        let mut edges: Vec<GraphEdge> = Vec::new();
        let mut degree: BTreeMap<String, usize> = BTreeMap::new();

        for page in self.pages.values() {
            let source = page.name.clone();
            for block in page.all_blocks() {
                for target in refs::extract_page_refs(&block.content) {
                    if !self.has_page(&target) {
                        continue;
                    }
                    if normalize(&target) == normalize(&source) {
                        continue;
                    }
                    edges.push(GraphEdge {
                        source: source.clone(),
                        target: target.clone(),
                    });
                    *degree.entry(normalize(&source)).or_insert(0) += 1;
                    *degree.entry(normalize(&target)).or_insert(0) += 1;
                }
            }
        }
        // Collapse mutual links (`A -> B` together with `B -> A`) into a single
        // edge. Sorting by the unordered key keeps the kept direction stable —
        // `self.pages` is a `BTreeMap`, so iteration order is deterministic.
        edges.sort_by_cached_key(|e| link_key(&e.source, &e.target));
        edges.dedup_by(|a, b| link_key(&a.source, &a.target) == link_key(&b.source, &b.target));

        let nodes = self
            .pages
            .values()
            .map(|p| GraphNode {
                id: p.name.clone(),
                label: p.display_title(),
                kind: p.kind,
                size: *degree.get(&normalize(&p.name)).unwrap_or(&0),
            })
            .collect();

        GraphData { nodes, edges }
    }

    /// Absolute path a page would be stored at.
    pub fn page_path(&self, page: &Page) -> PathBuf {
        match page.kind {
            PageKind::Journal => self
                .root
                .join(JOURNALS_DIR)
                .join(format!("{}.md", journal_name_to_file(&page.name))),
            PageKind::Page => self
                .root
                .join(PAGES_DIR)
                .join(format!("{}.md", page_name_to_file(&page.name))),
        }
    }

    fn ensure_dirs(&self) -> Result<()> {
        for dir in [PAGES_DIR, JOURNALS_DIR, ASSETS_DIR] {
            let path = self.root.join(dir);
            fs::create_dir_all(&path).map_err(|e| CoreError::io(&path, e))?;
        }
        Ok(())
    }

    fn reindex(&mut self) {
        self.index = SearchIndex::build(self.pages.values());
    }

    fn seed(&mut self) -> Result<()> {
        let mut welcome = Page::new("welcome", PageKind::Page);
        welcome.set_property("title", "Welcome to OpenSeq");

        welcome.blocks.push(Block::new(
            "OpenSeq is a local-first outliner. This **graph** is just a folder of markdown files.",
        ));

        let mut features = Block::new("Core features implemented:");
        for text in [
            "Block-based outliner with nesting (Tab / Shift+Tab)",
            "[[page links]] and #tags, resolved into a graph",
            "Linked references (backlinks) panel",
            "Journals / daily notes (see journals/)",
            "Full-text search with CJK support",
            "Graph view rendered on canvas",
        ] {
            features.children.push(Block::new(text));
        }
        welcome.blocks.push(features);

        welcome
            .blocks
            .push(Block::new("Open [[features]] to read the coverage list."));
        welcome.blocks.push(Block::new(
            "Nothing is uploaded anywhere: no account, no telemetry, no network.",
        ));

        self.save_page(welcome)?;
        Ok(())
    }
}

/// Normalise a page name for map keys / comparisons.
pub fn normalize(name: &str) -> String {
    name.trim().to_lowercase()
}

/// The *unordered* key for the connection between two pages.
///
/// A graph edge is a link *between* two pages, not a direction: if `A` links
/// `B` and `B` also links `A`, that is one connection and it is drawn as one
/// line. Keying on the sorted pair lets [`Graph::graph_data`] collapse mutual
/// links — otherwise it emits two edges that land exactly on top of each
/// other, and the legend reports one more edge than is actually visible.
fn link_key(a: &str, b: &str) -> (String, String) {
    let (na, nb) = (normalize(a), normalize(b));
    if na <= nb {
        (na, nb)
    } else {
        (nb, na)
    }
}

/// Encode a page name into a file stem (Logseq-compatible).
pub fn page_name_to_file(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        match ch {
            '/' | '?' | '*' | '<' | '>' | ':' | '"' | '\\' | '|' | '#' | '^' | '[' | ']' => {
                out.push_str(&format!("%{:02X}", ch as u32));
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Decode a file stem back into a page name.
pub fn file_to_page_name(stem: &str) -> String {
    let chars: Vec<char> = stem.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' && i + 2 < chars.len() {
            let hex: String = chars[i + 1..i + 3].iter().collect();
            if let Ok(code) = u32::from_str_radix(&hex, 16) {
                if let Some(ch) = char::from_u32(code) {
                    out.push(ch);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `2026-10-11` → `2026_10_11` (Logseq journal file naming).
pub fn journal_name_to_file(name: &str) -> String {
    name.replace('-', "_")
}

/// `2026_10_11` → `2026-10-11`.
pub fn journal_file_to_name(stem: &str) -> String {
    stem.replace('_', "-")
}

/// Write a file atomically: temp file in the same directory, then rename.
fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = parent.join(format!(
        ".{}.tmp",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("page")
    ));
    fs::write(&tmp, contents).map_err(|e| CoreError::io(&tmp, e))?;
    fs::rename(&tmp, path).map_err(|e| CoreError::io(path, e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_name_roundtrip() {
        for name in ["Foo", "a/b", "notes/2026/Q1", "what? why: that"] {
            let stem = page_name_to_file(name);
            assert_eq!(file_to_page_name(&stem), name, "stem was {stem}");
        }
    }

    #[test]
    fn journal_names_roundtrip() {
        assert_eq!(journal_name_to_file("2026-10-11"), "2026_10_11");
        assert_eq!(journal_file_to_name("2026_10_11"), "2026-10-11");
    }
}
