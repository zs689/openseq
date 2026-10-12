//! Logseq-dialect markdown → block tree.
//!
//! The on-disk dialect this parser understands:
//!
//! ```text
//! title:: My Page              <- page property (no bullet, indent 0)
//! tags:: a, b
//!
//! - A top level block
//!   - A child block
//!     prop:: value             <- block property (indented under its parent)
//!   - TODO [#A] a task
//!   - ```js                    <- fenced code; following lines are content
//!     const a = 1
//!     ```
//!   continued line of the parent  <- continuation (no bullet)
//! ```
//!
//! Indentation defines nesting; Logseq uses two spaces per level (tabs are
//! accepted and counted as two). Lines without a bullet that are more indented
//! than the current block are folded into that block's `content`.

use std::sync::LazyLock;

use regex::Regex;

use crate::model::{Block, Page, PageKind, TODO_MARKERS};

static BULLET_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([ \t]*)(?:[-*+]|\d+[.)])[ \t]+(.*)$").unwrap());
static PROPERTY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z][A-Za-z0-9_\-]*)::[ \t]*(.*)$").unwrap());
static PRIORITY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[#([A-Za-z0-9]+)\][ \t]*(.*)$").unwrap());

/// Parse a markdown document into a [`Page`].
pub fn parse_page(name: &str, kind: PageKind, path: &str, src: &str) -> Page {
    let mut page = Page::new(name, kind);
    page.path = path.to_string();

    // Nodes whose parent has not been decided yet. `close_below` attaches a
    // popped node to whatever is left on the stack (or to the page root).
    let mut stack: Vec<(usize, Block)> = Vec::new();
    let mut root: Vec<Block> = Vec::new();

    // When Some((base_indent, marker)) we are inside a fenced code block and
    // every following line is raw content of the currently open block.
    let mut fence: Option<(usize, String)> = None;

    for raw_line in src.lines() {
        if let Some((base, marker)) = fence.clone() {
            if raw_line.trim_start().starts_with(&marker) {
                append_content(&mut stack, strip_indent(raw_line, base));
                fence = None;
            } else {
                append_content(&mut stack, strip_indent(raw_line, base));
            }
            continue;
        }

        if raw_line.trim().is_empty() {
            // Blank lines are separators. Logseq drops them when it rewrites a
            // file, so we normalise them away too (see module docs).
            continue;
        }

        if let Some(caps) = BULLET_RE.captures(raw_line) {
            let indent = indent_width(&caps[1]);
            let text = caps[2].to_string();
            let (marker, priority, content) = split_marker_and_priority(&text);

            let mut block = Block::new(content);
            block.marker = marker;
            block.priority = priority;

            close_below(&mut stack, &mut root, indent);

            if let Some(m) = fence_marker(&block.content) {
                fence = Some((indent + 2, m));
            }

            stack.push((indent, block));
            continue;
        }

        // No bullet on this line: either a property or a continuation.
        let trimmed = raw_line.trim();
        let indent = indent_width(&raw_line[..raw_line.len() - raw_line.trim_start().len()]);

        if let Some(caps) = PROPERTY_RE.captures(trimmed) {
            let property = crate::model::Property::new(&caps[1], caps[2].trim());
            let belongs_to_block = matches!(
                stack.last(),
                Some((top_indent, _)) if indent > *top_indent
            );
            if belongs_to_block {
                stack
                    .last_mut()
                    .expect("checked above")
                    .1
                    .properties
                    .push(property);
            } else {
                page.properties.push(property);
            }
            continue;
        }

        // Plain continuation text.
        let continues_block = matches!(
            stack.last(),
            Some((top_indent, _)) if indent > *top_indent
        );
        if continues_block {
            append_content(&mut stack, trimmed);
        } else {
            // Stray top-level text: keep it rather than silently dropping it.
            close_below(&mut stack, &mut root, 0);
            stack.push((0, Block::new(trimmed)));
        }
    }

    close_below(&mut stack, &mut root, 0);

    // Fold any `id::` property into the identity field.
    for block in &mut root {
        block.walk_mut(&mut |b| b.take_persisted_id());
    }

    page.blocks = root;
    page
}

/// Close every open block whose indent is >= `indent`, attaching it to its
/// parent (the new stack top) or to the page root.
fn close_below(stack: &mut Vec<(usize, Block)>, root: &mut Vec<Block>, indent: usize) {
    while stack.last().is_some_and(|(top, _)| *top >= indent) {
        let (_, node) = stack.pop().expect("stack checked above");
        match stack.last_mut() {
            Some((_, parent)) => parent.children.push(node),
            None => root.push(node),
        }
    }
}

fn append_content(stack: &mut [(usize, Block)], text: &str) {
    if let Some((_, block)) = stack.last_mut() {
        block.content.push('\n');
        block.content.push_str(text);
    }
}

/// Split `TODO [#A] text` into `(Some("TODO"), Some("A"), "text")`.
fn split_marker_and_priority(text: &str) -> (Option<String>, Option<String>, String) {
    let mut rest = text.trim_start();

    let mut marker = None;
    for m in TODO_MARKERS {
        if let Some(after) = rest.strip_prefix(m) {
            if after.is_empty() || after.starts_with([' ', '\t']) {
                marker = Some(m.to_string());
                rest = after.trim_start();
                break;
            }
        }
    }

    let (priority, content) = strip_priority(rest);
    (marker, priority, content.trim_end().to_string())
}

/// Split a leading `[#A]` priority off `text`, returning the priority letter
/// (if any) and the remaining block text.
///
/// Kept as its own function so the lifetime of the returned slice is tied to
/// the input by the signature — doing this inline forces `Captures` to outlive
/// the borrow, which does not compile.
fn strip_priority(text: &str) -> (Option<String>, &str) {
    match PRIORITY_RE.captures(text) {
        Some(caps) => {
            // Use `get`/`as_str` rather than `caps[i]`: indexing borrows the
            // `Captures` value, while `get` yields a slice tied to the input.
            // Group 2, not "after the whole match": the pattern ends in
            // `(.*)$`, so the whole match already swallows the rest of the line.
            let priority = caps.get(1).map(|m| m.as_str().to_string());
            let rest = caps.get(2).map(|m| m.as_str()).unwrap_or(text);
            (priority, rest.trim_start())
        }
        None => (None, text),
    }
}

fn fence_marker(content: &str) -> Option<String> {
    let first = content.lines().next().unwrap_or("");
    let t = first.trim_start();
    if t.starts_with("```") {
        Some("```".to_string())
    } else if t.starts_with("~~~") {
        Some("~~~".to_string())
    } else {
        None
    }
}

/// Count leading whitespace, treating a tab as two columns.
fn indent_width(ws: &str) -> usize {
    ws.chars().map(|c| if c == '\t' { 2 } else { 1 }).sum()
}

/// Remove up to `n` leading whitespace columns from `line`.
fn strip_indent(line: &str, n: usize) -> &str {
    let mut removed = 0usize;
    let mut end = 0usize;
    for (byte_idx, ch) in line.char_indices() {
        if removed >= n {
            break;
        }
        match ch {
            ' ' => {
                removed += 1;
                end = byte_idx + 1;
            }
            '\t' => {
                removed += 2;
                end = byte_idx + 1;
            }
            _ => {
                end = byte_idx;
                break;
            }
        }
    }
    &line[end..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_blocks() {
        let src = "- a\n  - b\n    - c\n- d\n";
        let page = parse_page("Test", PageKind::Page, "pages/Test.md", src);
        assert_eq!(page.blocks.len(), 2);
        assert_eq!(page.blocks[0].content, "a");
        assert_eq!(page.blocks[0].children[0].content, "b");
        assert_eq!(page.blocks[0].children[0].children[0].content, "c");
        assert_eq!(page.blocks[1].content, "d");
    }

    #[test]
    fn parses_marker_and_priority() {
        let page = parse_page("T", PageKind::Page, "", "- TODO [#A] ship it\n");
        let b = &page.blocks[0];
        assert_eq!(b.marker.as_deref(), Some("TODO"));
        assert_eq!(b.priority.as_deref(), Some("A"));
        assert_eq!(b.content, "ship it");
    }

    #[test]
    fn parses_page_and_block_properties() {
        let src = "title:: Hello\nalias:: hi\n\n- block\n  prop:: value\n";
        let page = parse_page("Hello", PageKind::Page, "", src);
        assert_eq!(page.property("title"), Some("Hello"));
        assert_eq!(page.blocks[0].properties[0].key, "prop");
        assert_eq!(page.blocks[0].properties[0].value, "value");
    }

    #[test]
    fn parses_fenced_code_block_as_one_block() {
        let src = "- ```js\n  const a = 1\n  ```\n";
        let page = parse_page("T", PageKind::Page, "", src);
        assert_eq!(page.blocks.len(), 1);
        assert_eq!(page.blocks[0].content, "```js\nconst a = 1\n```");
    }
}
