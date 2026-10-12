//! Domain model: [`Block`], [`Page`] and their helper methods.
//!
//! The shape of this model is deliberately close to Logseq's own concepts so
//! that the parser/serializer can stay faithful to the on-disk format:
//!
//! * a **page** owns a list of top-level blocks,
//! * a **block** is a single outline node which may itself contain children,
//! * *task marker*, *priority* and *properties* are kept as structured fields
//!   rather than as raw text, so the UI can render and cycle them.

use serde::{Deserialize, Serialize};

/// Task markers recognised by Logseq, listed in the order Logseq cycles them.
pub const TODO_MARKERS: [&str; 7] = [
    "TODO", "DOING", "NOW", "LATER", "WAITING", "DONE", "CANCELED",
];

/// A `key:: value` property attached to a page or a block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    pub key: String,
    pub value: String,
}

impl Property {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// A single outline node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Stable identity (uuid v4). Always present in memory.
    pub id: String,
    /// Block text with the leading marker/priority stripped out. May contain
    /// newlines (multi-line blocks such as fenced code).
    pub content: String,
    /// One of [`TODO_MARKERS`] when the block starts with one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    /// `[#A]` style priority letter, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<Property>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Block>,
    #[serde(default)]
    pub collapsed: bool,
    /// Whether `id:: <uuid>` is written to disk for this block. Logseq only
    /// persists a block id when the block is referenced from somewhere.
    #[serde(default)]
    pub id_in_file: bool,
}

impl Block {
    /// Create a block with a fresh identity.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: crate::id::new_block_id(),
            content: content.into(),
            marker: None,
            priority: None,
            properties: Vec::new(),
            children: Vec::new(),
            collapsed: false,
            id_in_file: false,
        }
    }

    /// Create a block that already knows its persisted id.
    pub fn with_id(id: impl Into<String>, content: impl Into<String>) -> Self {
        let mut b = Block::new(content);
        b.id = id.into();
        b.id_in_file = true;
        b
    }

    /// Depth-first visit of `self` and every descendant.
    pub fn walk<'a>(&'a self, f: &mut impl FnMut(&'a Block)) {
        f(self);
        for child in &self.children {
            child.walk(f);
        }
    }

    /// Mutable depth-first visit of `self` and every descendant.
    pub fn walk_mut(&mut self, f: &mut impl FnMut(&mut Block)) {
        f(self);
        for child in &mut self.children {
            child.walk_mut(f);
        }
    }

    /// Find a descendant (or self) by id.
    pub fn find(&self, id: &str) -> Option<&Block> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|c| c.find(id))
    }

    /// Find a descendant (or self) by id, mutably.
    pub fn find_mut(&mut self, id: &str) -> Option<&mut Block> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter_mut().find_map(|c| c.find_mut(id))
    }

    /// Number of nodes in this subtree, including `self`.
    pub fn subtree_len(&self) -> usize {
        1 + self.children.iter().map(Block::subtree_len).sum::<usize>()
    }

    /// Return a fixed-up id if the block's `properties` contain `id::`.
    pub fn take_persisted_id(&mut self) {
        if let Some(pos) = self
            .properties
            .iter()
            .position(|p| p.key.eq_ignore_ascii_case("id"))
        {
            let value = self.properties.remove(pos).value;
            let value = value.trim().to_string();
            if !value.is_empty() {
                self.id = value;
                self.id_in_file = true;
            }
        }
    }

    /// Human-readable text: markup stripped, useful for search previews and
    /// for backlink rendering.
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        if let Some(m) = &self.marker {
            out.push_str(m);
            out.push(' ');
        }
        if let Some(p) = &self.priority {
            out.push('[');
            out.push('#');
            out.push_str(p);
            out.push_str("] ");
        }
        out.push_str(&strip_inline_markup(&self.content));
        out
    }

    /// Remove `self` from the tree rooted at `root`; returns the removed node.
    pub fn remove_from(root: &mut Block, id: &str) -> Option<Block> {
        if let Some(pos) = root.children.iter().position(|c| c.id == id) {
            return Some(root.children.remove(pos));
        }
        for child in &mut root.children {
            if let Some(found) = Block::remove_from(child, id) {
                return Some(found);
            }
        }
        None
    }

    /// Append a child. Used by the command layer when inserting blocks.
    pub fn push_child(&mut self, child: Block) {
        self.children.push(child);
    }
}

/// Strip `[[..]]`, `((..))`, `**`/`_`/`` ` `` etc. for display purposes.
pub fn strip_inline_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        // [[page]]
        if bytes[i] == '[' && i + 1 < bytes.len() && bytes[i + 1] == '[' {
            if let Some(end) = find_seq(&bytes, i + 2, &[']', ']']) {
                let inner: String = bytes[i + 2..end].iter().collect();
                out.push_str(&inner);
                i = end + 2;
                continue;
            }
        }
        // ((block-id))
        if bytes[i] == '(' && i + 1 < bytes.len() && bytes[i + 1] == '(' {
            if let Some(end) = find_seq(&bytes, i + 2, &[')', ')']) {
                i = end + 2;
                continue;
            }
        }
        match bytes[i] {
            '*' | '_' | '`' | '~' => {}
            c => out.push(c),
        }
        i += 1;
    }
    out
}

fn find_seq(haystack: &[char], from: usize, needle: &[char]) -> Option<usize> {
    if needle.is_empty() || from >= haystack.len() {
        return None;
    }
    (from..haystack.len().saturating_sub(needle.len() - 1))
        .find(|&i| &haystack[i..i + needle.len()] == needle)
}

/// Whether a page is a regular page or a journal (daily note).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageKind {
    Page,
    Journal,
}

/// A page: the unit that maps 1:1 to a markdown file on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    /// Logical name, e.g. `Project/Roadmap` or `2026-10-11`.
    pub name: String,
    pub kind: PageKind,
    /// Page-level properties (`title::`, `alias::`, `tags::` …).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<Property>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>,
    /// Path relative to the graph root, e.g. `pages/Foo.md`.
    #[serde(default)]
    pub path: String,
}

impl Page {
    pub fn new(name: impl Into<String>, kind: PageKind) -> Self {
        Self {
            name: name.into(),
            kind,
            properties: Vec::new(),
            blocks: Vec::new(),
            path: String::new(),
        }
    }

    pub fn property(&self, key: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|p| p.key.eq_ignore_ascii_case(key))
            .map(|p| p.value.as_str())
    }

    pub fn set_property(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();
        match self
            .properties
            .iter_mut()
            .find(|p| p.key.eq_ignore_ascii_case(key))
        {
            Some(p) => p.value = value,
            None => self.properties.push(Property::new(key, value)),
        }
    }

    /// Every block in the page, flattened depth-first.
    pub fn all_blocks(&self) -> Vec<&Block> {
        let mut out = Vec::new();
        for b in &self.blocks {
            b.walk(&mut |n| out.push(n));
        }
        out
    }

    pub fn block(&self, id: &str) -> Option<&Block> {
        self.blocks.iter().find_map(|b| b.find(id))
    }

    pub fn block_mut(&mut self, id: &str) -> Option<&mut Block> {
        self.blocks.iter_mut().find_map(|b| b.find_mut(id))
    }

    /// Title shown in the UI: explicit `title::` property, else the page name.
    pub fn display_title(&self) -> String {
        self.property("title")
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.name.clone())
    }
}
