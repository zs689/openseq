//! Block tree → Logseq-dialect markdown.
//!
//! The exact inverse of [`crate::parser`]. `parse_page(serialize_page(p)) == p`
//! for every page this module produces — see `tests/roundtrip.rs`.

use crate::model::{Block, Page};

/// Serialise a page back to Logseq markdown.
pub fn serialize_page(page: &Page) -> String {
    let mut out = String::new();

    for prop in &page.properties {
        out.push_str(&prop.key);
        out.push_str(":: ");
        out.push_str(&prop.value);
        out.push('\n');
    }
    if !page.properties.is_empty() && !page.blocks.is_empty() {
        out.push('\n');
    }

    for block in &page.blocks {
        write_block(block, 0, &mut out);
    }
    out
}

fn write_block(block: &Block, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    let child_indent = format!("{indent}  ");

    out.push_str(&indent);
    out.push_str("- ");
    if let Some(marker) = &block.marker {
        out.push_str(marker);
        out.push(' ');
    }
    if let Some(priority) = &block.priority {
        out.push_str("[#");
        out.push_str(priority);
        out.push_str("] ");
    }

    let mut lines = block.content.split('\n');
    out.push_str(lines.next().unwrap_or(""));
    out.push('\n');

    for line in lines {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str(&child_indent);
            out.push_str(line);
            out.push('\n');
        }
    }

    for prop in &block.properties {
        out.push_str(&child_indent);
        out.push_str(&prop.key);
        out.push_str(":: ");
        out.push_str(&prop.value);
        out.push('\n');
    }

    if block.id_in_file {
        out.push_str(&child_indent);
        out.push_str("id:: ");
        out.push_str(&block.id);
        out.push('\n');
    }

    for child in &block.children {
        write_block(child, depth + 1, out);
    }
}
