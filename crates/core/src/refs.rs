//! `[[page]]`, `#tag` and `((block-id))` extraction.
//!
//! Logseq treats `[[foo]]` and `#foo` as links to the *same* page space, so
//! both are decoded into "page references" here. `((uuid))` is a reference to
//! a specific block, which is what powers the "Linked References" of a block.

use std::sync::LazyLock;

use regex::Regex;

use crate::model::Block;

static PAGE_REF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[([^\[\]\n]+)\]\]").unwrap());
static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?:^|[\s(\[{'"])#([^\s#\[\](){}]+)"#).unwrap());
static BLOCK_REF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\(\(([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})\)\)",
    )
    .unwrap()
});

/// Extract all page names referenced by `text`, via `[[..]]` or `#tag`.
///
/// Names are returned de-duplicated and trimmed but **not** lower-cased — the
/// caller decides how to normalise (Logseq lower-cases page *filenames* but
/// keeps display case in the link text).
pub fn extract_page_refs(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for cap in PAGE_REF_RE.captures_iter(text) {
        push_unique(&mut out, cap[1].trim());
    }
    for cap in TAG_RE.captures_iter(text) {
        push_unique(&mut out, trim_trailing_punctuation(cap[1].trim()));
    }
    out
}

/// Extract all block ids referenced by `text`.
pub fn extract_block_refs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for cap in BLOCK_REF_RE.captures_iter(text) {
        push_unique(&mut out, &cap[1]);
    }
    out
}

/// All page references made by a whole page, together with the block that made
/// them. Useful for building the backlink index and the graph view.
pub fn collect_page_refs(blocks: &[Block]) -> Vec<(String, String)> {
    // (referenced page, referencing block id)
    let mut out = Vec::new();
    for block in blocks {
        block.walk(&mut |b| {
            for r in extract_page_refs(&format!("{}\n{}", b.content, b.plain_text())) {
                out.push((r, b.id.clone()));
            }
        });
    }
    out
}

/// Page names whose *own* link target is `name` — i.e. everything that points
/// at `name` from anywhere else in the graph.
pub fn references_to<'a>(
    candidates: impl IntoIterator<Item = (&'a str, &'a Block)>,
    name: &str,
) -> Vec<&'a Block> {
    let needle = name.to_lowercase();
    candidates
        .into_iter()
        .filter(|(_, b)| {
            extract_page_refs(&format!("{}\n{}", b.content, b.plain_text()))
                .iter()
                .any(|r| r.to_lowercase() == needle)
        })
        .map(|(_, b)| b)
        .collect()
}

fn push_unique(out: &mut Vec<String>, value: &str) {
    if value.is_empty() {
        return;
    }
    if !out.iter().any(|v| v == value) {
        out.push(value.to_string());
    }
}

fn trim_trailing_punctuation(value: &str) -> &str {
    value.trim_end_matches([
        '.', ',', ';', ':', '!', '?', '。', '，', '；', '：', '！', '？',
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_wiki_links() {
        let refs = extract_page_refs("see [[Project Roadmap]] and [[a/b]]");
        assert_eq!(refs, vec!["Project Roadmap", "a/b"]);
    }

    #[test]
    fn extracts_tags() {
        let refs = extract_page_refs("task #work #中文标签 done");
        assert_eq!(refs, vec!["work", "中文标签"]);
    }

    #[test]
    fn extracts_block_refs() {
        let text = "ref ((3f2504e0-4f89-41d3-9a0c-0305e82c3301)) end";
        assert_eq!(
            extract_block_refs(text),
            vec!["3f2504e0-4f89-41d3-9a0c-0305e82c3301"]
        );
    }

    #[test]
    fn ignores_code_fences_inside_links() {
        assert!(extract_page_refs("no links here").is_empty());
    }
}
