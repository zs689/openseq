//! # logseq-core
//!
//! The domain core of **OpenSeq** — a Logseq-compatible, local-first outliner
//! re-implemented in Rust + TypeScript + Tauri.
//!
//! This crate intentionally contains **no** Tauri, no UI and no async runtime
//! coupling. Everything here is plain, synchronous, testable Rust:
//!
//! * [`model`]      — the `Block` / `Page` data model
//! * [`parser`]     — Logseq-dialect markdown → block tree
//! * [`serializer`] — block tree → Logseq-dialect markdown (round-trip safe)
//! * [`store`]      — the on-disk *graph* (`pages/`, `journals/`, `assets/`)
//! * [`refs`]       — `[[page]]`, `#tag` and `((block-id))` extraction
//! * [`search`]     — an incremental inverted index with CJK support
//!
//! The Tauri shell in `src-tauri/` is a thin adapter that exposes this crate's
//! API as `invoke` commands. Keeping the split means the interesting logic can
//! be verified with `cargo test` even when no desktop toolchain is available.

pub mod error;
pub mod id;
pub mod model;
pub mod parser;
pub mod refs;
pub mod search;
pub mod serializer;
pub mod store;

pub use error::{CoreError, Result};
pub use id::{is_block_id, new_block_id};
pub use model::{Block, Page, PageKind, Property, TODO_MARKERS};
pub use parser::parse_page;
pub use refs::{extract_block_refs, extract_page_refs};
pub use search::{SearchHit, SearchIndex};
pub use serializer::serialize_page;
pub use store::{Backlink, Graph, GraphData, GraphEdge, GraphNode, PageMeta};
