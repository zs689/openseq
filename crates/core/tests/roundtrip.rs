//! End-to-end tests for the core crate: parse → serialise → re-parse and the
//! graph-level features (backlinks, search, graph data).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use logseq_core::{parse_page, serialize_page, Block, Graph, Page, PageKind, Property};

/// A throwaway directory that cleans itself up.
///
/// Hand-rolled on purpose: `tempfile` would pull `rustix`/`windows-sys` into
/// the dev-dependency graph, and this is twenty lines.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(tag: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path =
            std::env::temp_dir().join(format!("logseq-core-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn sample_source() -> &'static str {
    "title:: Sample\ntags:: a, b\n\n- A top block\n  - child one\n    prop:: v\n  - TODO [#A] child two\n- ```js\n  const a = 1\n  ```\n- Link to [[Other Page]] and #tag\n"
}

#[test]
fn parse_serialize_is_a_fixed_point() {
    let page = parse_page("Sample", PageKind::Page, "pages/Sample.md", sample_source());
    let once = serialize_page(&page);
    let reparsed = parse_page("Sample", PageKind::Page, "pages/Sample.md", &once);
    let twice = serialize_page(&reparsed);

    assert_eq!(once, twice, "serialisation is not idempotent");
    assert_eq!(reparsed.property("title"), Some("Sample"));
    assert_eq!(reparsed.blocks.len(), page.blocks.len());
}

#[test]
fn preserves_code_block_verbatim() {
    let page = parse_page("C", PageKind::Page, "", "- ```js\n  const a = 1\n  ```\n");
    let out = serialize_page(&page);
    assert!(out.contains("```js"), "fence lost: {out:?}");
    assert!(out.contains("  const a = 1"), "code indent lost: {out:?}");
}

#[test]
fn graph_roundtrips_through_disk() {
    let dir = TempDir::new("roundtrip");

    let mut graph = Graph::create(dir.path()).unwrap();
    assert!(graph.has_page("welcome"), "seeded welcome page missing");

    let mut other = Page::new("Other Page", PageKind::Page);
    other.blocks.push(Block::new("hello from the other page"));
    graph.save_page(other).unwrap();

    let file = dir.path().join("pages").join("Other Page.md");
    assert!(file.is_file(), "file not written at {file:?}");

    // Re-open from disk: everything must come back.
    let reopened = Graph::open(dir.path()).unwrap();
    let page = reopened
        .page("other page")
        .expect("case-insensitive lookup failed");
    assert_eq!(page.kind, PageKind::Page);
    assert_eq!(page.blocks[0].content, "hello from the other page");
}

#[test]
fn backlinks_and_search_work() {
    let dir = TempDir::new("backlinks");
    let mut graph = Graph::create(dir.path()).unwrap();

    let mut a = Page::new("Alpha", PageKind::Page);
    a.blocks
        .push(Block::new("points at [[Beta]] and talks about 知识管理"));
    graph.save_page(a).unwrap();

    let mut b = Page::new("Beta", PageKind::Page);
    b.blocks.push(Block::new("I am beta"));
    graph.save_page(b).unwrap();

    let links = graph.backlinks("Beta");
    assert_eq!(links.len(), 1, "expected one backlink, got {links:?}");
    assert_eq!(links[0].page, "Alpha");

    let hits = graph.search("知识", 10);
    assert_eq!(hits.len(), 1, "CJK search failed: {hits:?}");
    assert_eq!(hits[0].page, "Alpha");

    let data = graph.graph_data();
    assert!(data.nodes.iter().any(|n| n.id == "Alpha"));
    assert!(data
        .edges
        .iter()
        .any(|e| e.source == "Alpha" && e.target == "Beta"));
}

#[test]
fn journals_are_stored_under_journals_dir() {
    let dir = TempDir::new("journal");
    let mut graph = Graph::create(dir.path()).unwrap();

    let page = graph
        .get_or_create_page("2026-10-11", PageKind::Journal)
        .unwrap();
    assert_eq!(page.kind, PageKind::Journal);
    assert!(
        dir.path().join("journals").join("2026_10_11.md").is_file(),
        "journal file name is wrong"
    );

    let reopened = Graph::open(dir.path()).unwrap();
    assert!(reopened.has_page("2026-10-11"));
}

#[test]
fn block_ids_persist_when_requested() {
    let dir = TempDir::new("blockid");
    let mut graph = Graph::create(dir.path()).unwrap();

    let mut page = Page::new("Refs", PageKind::Page);
    let mut b = Block::new("referenced block");
    b.id_in_file = true;
    let id = b.id.clone();
    page.blocks.push(b);
    page.properties.push(Property::new("note", "x"));
    graph.save_page(page).unwrap();

    let raw = std::fs::read_to_string(dir.path().join("pages").join("Refs.md")).unwrap();
    assert!(
        raw.contains(&format!("id:: {id}")),
        "id not persisted: {raw:?}"
    );

    let reopened = Graph::open(dir.path()).unwrap();
    let stored = reopened.page("Refs").unwrap();
    assert_eq!(stored.blocks[0].id, id);
    assert!(stored.blocks[0].id_in_file);
}

#[test]
fn saves_loads_and_deletes_cleanly() {
    let dir = TempDir::new("lifecycle");
    let mut graph = Graph::create(dir.path()).unwrap();

    let mut page = Page::new("Temp", PageKind::Page);
    page.blocks.push(Block::new("gone soon"));
    graph.save_page(page).unwrap();
    assert!(dir.path().join("pages").join("Temp.md").is_file());

    graph.delete_page("Temp").unwrap();
    assert!(
        !dir.path().join("pages").join("Temp.md").exists(),
        "file left behind"
    );

    let reopened = Graph::open(dir.path()).unwrap();
    assert!(!reopened.has_page("Temp"));
}

#[test]
fn namespaced_page_names_roundtrip_through_the_filesystem() {
    let dir = TempDir::new("namespace");
    let mut graph = Graph::create(dir.path()).unwrap();

    let mut page = Page::new("notes/2026/Q1", PageKind::Page);
    page.blocks.push(Block::new("nested"));
    graph.save_page(page).unwrap();

    let reopened = Graph::open(dir.path()).unwrap();
    assert!(
        reopened.has_page("notes/2026/Q1"),
        "namespaced page lost on reload"
    );
}

/// A link connects **two** pages; it does not have a direction.
///
/// If `Alpha` links `Beta` and `Beta` links `Alpha`, that is one connection.
/// Emitting two edges is not merely cosmetic: the canvas draws both segments
/// along the same coordinates (so you see one line) while the legend counts
/// both (so it says two) — the numbers stop matching the picture.
#[test]
fn mutual_links_collapse_to_a_single_edge() {
    let dir = TempDir::new("mutual");
    let mut graph = Graph::create(dir.path()).unwrap();

    let mut alpha = Page::new("Alpha", PageKind::Page);
    alpha.blocks.push(Block::new("see [[Beta]]"));
    graph.save_page(alpha).unwrap();

    let mut beta = Page::new("Beta", PageKind::Page);
    beta.blocks.push(Block::new("see [[Alpha]]"));
    graph.save_page(beta).unwrap();

    let data = graph.graph_data();
    assert_eq!(
        data.edges.len(),
        1,
        "Alpha<->Beta is one connection, not two: {:?}",
        data.edges
    );

    let edge = &data.edges[0];
    let mut pair = [edge.source.as_str(), edge.target.as_str()];
    pair.sort();
    assert_eq!(pair, ["Alpha", "Beta"], "edge points at the wrong pages");
}

/// The graph committed under `examples/demo-graph` is documentation as much as
/// a fixture — if it stops loading, the docs are lying.
#[test]
fn the_shipped_demo_graph_loads_and_links_up() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/demo-graph");
    assert!(root.is_dir(), "demo graph missing at {root:?}");

    let graph = Graph::open(&root).expect("demo graph should open");
    for name in [
        "Welcome to OpenSeq",
        "Features",
        "知识管理",
        "OpenSeq 架构",
        "2026-10-11",
    ] {
        assert!(
            graph.has_page(name),
            "demo graph is missing the page {name:?}"
        );
    }

    let links = graph.backlinks("Features");
    assert!(
        !links.is_empty(),
        "nothing links to [[Features]] in the demo graph"
    );

    let hits = graph.search("知识", 10);
    assert!(
        !hits.is_empty(),
        "CJK search found nothing in the demo graph"
    );

    let data = graph.graph_data();
    assert!(data.nodes.len() >= 5, "expected all demo pages as nodes");
    assert!(!data.edges.is_empty(), "expected edges between demo pages");

    // No page pair may appear twice. Three of the demo's links are mutual
    // (Features<->Welcome, OpenSeq 架构<->Welcome, Welcome<->知识管理), so the
    // 12 references in the markdown must come out as 9 edges.
    let mut pairs: BTreeSet<(String, String)> = BTreeSet::new();
    for edge in &data.edges {
        let (lo, hi) = if edge.source <= edge.target {
            (&edge.source, &edge.target)
        } else {
            (&edge.target, &edge.source)
        };
        assert!(
            pairs.insert((lo.clone(), hi.clone())),
            "{lo:?} <-> {hi:?} appears twice; those two edges would be drawn \
             exactly on top of each other and the legend would overcount"
        );
    }
    assert_eq!(
        data.edges.len(),
        9,
        "demo graph edge count changed: {:?}",
        data.edges
    );
}
