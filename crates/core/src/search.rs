//! A small incremental inverted index with CJK support.
//!
//! ASCII text is tokenised into words; CJK text is tokenised into single
//! characters **plus** overlapping bigrams, which is a cheap and surprisingly
//! effective "poor man's" segmentation for Chinese/Japanese. Queries are
//! tokenised the same way, so "知识管理" and "知识" both hit the same block.
//!
//! The index is rebuilt from the in-memory page map after every save. For a
//! personal knowledge graph (thousands of blocks) that is well under a frame
//! budget, and it keeps the code honest — there is no way for the index to
//! drift out of sync with the files.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::model::{Block, Page};

/// One search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub page: String,
    pub block_id: String,
    pub text: String,
    pub score: u32,
}

#[derive(Debug, Clone)]
struct Doc {
    page: String,
    block_id: String,
    text: String,
    lower: String,
}

/// In-memory inverted index over every block of every page.
#[derive(Debug, Default)]
pub struct SearchIndex {
    docs: Vec<Doc>,
    postings: HashMap<String, Vec<usize>>,
}

impl SearchIndex {
    /// Build an index over a set of pages.
    pub fn build<'a>(pages: impl IntoIterator<Item = &'a Page>) -> Self {
        let mut index = SearchIndex::default();
        for page in pages {
            index.add_page(page);
        }
        index
    }

    /// Add every block of one page.
    pub fn add_page(&mut self, page: &Page) {
        for block in page.all_blocks() {
            if block.content.trim().is_empty() {
                continue;
            }
            self.push_doc(page.name.clone(), block.id.clone(), block.plain_text());
        }
    }

    fn push_doc(&mut self, page: String, block_id: String, text: String) {
        let lower = text.to_lowercase();
        let tokens: HashSet<String> = tokenize(&lower).into_iter().collect();
        let idx = self.docs.len();
        self.docs.push(Doc {
            page,
            block_id,
            text,
            lower,
        });
        for token in tokens {
            self.postings.entry(token).or_default().push(idx);
        }
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// Search for `query`, returning at most `limit` results, best first.
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }
        let needle = query.to_lowercase();
        let tokens = tokenize(&needle);

        let mut scores: HashMap<usize, u32> = HashMap::new();
        for token in &tokens {
            if let Some(ids) = self.postings.get(token) {
                for &id in ids {
                    *scores.entry(id).or_insert(0) += 1;
                }
            }
        }

        let mut hits: Vec<SearchHit> = scores
            .into_iter()
            .map(|(id, mut score)| {
                let doc = &self.docs[id];
                if doc.lower.contains(&needle) {
                    score += 6;
                }
                let covered = tokens
                    .iter()
                    .filter(|t| doc.lower.contains(t.as_str()))
                    .count() as u32;
                score += covered * 2;
                SearchHit {
                    page: doc.page.clone(),
                    block_id: doc.block_id.clone(),
                    text: doc.text.clone(),
                    score,
                }
            })
            .collect();

        // Fallback: partial / substring queries that produce no token hits.
        if hits.is_empty() {
            for doc in &self.docs {
                if doc.lower.contains(&needle) {
                    hits.push(SearchHit {
                        page: doc.page.clone(),
                        block_id: doc.block_id.clone(),
                        text: doc.text.clone(),
                        score: 1,
                    });
                }
            }
        }

        hits.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.page.to_lowercase().cmp(&b.page.to_lowercase()))
        });
        hits.truncate(limit);
        hits
    }
}

/// Tokenise mixed ASCII/CJK text into index terms.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut word = String::new();
    let mut prev_cjk: Option<char> = None;

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            word.push(ch);
            prev_cjk = None;
            continue;
        }
        if is_cjk(ch) {
            if !word.is_empty() {
                tokens.push(std::mem::take(&mut word));
            }
            tokens.push(ch.to_string());
            if let Some(prev) = prev_cjk {
                tokens.push(format!("{prev}{ch}"));
            }
            prev_cjk = Some(ch);
            continue;
        }
        if !word.is_empty() {
            tokens.push(std::mem::take(&mut word));
        }
        prev_cjk = None;
    }
    if !word.is_empty() {
        tokens.push(word);
    }
    tokens
}

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32,
        0x3040..=0x30FF   // Hiragana + Katakana
        | 0x3400..=0x4DBF // CJK Ext A
        | 0x4E00..=0x9FFF // CJK Unified
        | 0xF900..=0xFAFF // CJK Compatibility Ideographs
    )
}

/// Convenience helper used by tests and the graph view legend.
pub fn prime_for_blocks(blocks: &[Block]) -> Vec<String> {
    blocks
        .iter()
        .flat_map(|b| tokenize(&b.plain_text()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_ascii_and_cjk() {
        let toks = tokenize("hello world 知识管理");
        assert!(toks.contains(&"hello".to_string()));
        assert!(toks.contains(&"world".to_string()));
        assert!(toks.contains(&"知识".to_string()));
        assert!(toks.contains(&"管理".to_string()));
    }

    #[test]
    fn searches_chinese_by_substring() {
        let mut page = Page::new("中文页", crate::model::PageKind::Page);
        page.blocks.push(Block::new("知识管理是一种实践"));
        page.blocks.push(Block::new("无关内容"));
        let index = SearchIndex::build([&page]);
        let hits = index.search("知识", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "知识管理是一种实践");
    }

    #[test]
    fn searches_ascii_words() {
        let mut page = Page::new("English", crate::model::PageKind::Page);
        page.blocks.push(Block::new("Local-first outliner"));
        page.blocks.push(Block::new("something else"));
        let index = SearchIndex::build([&page]);
        let hits = index.search("outliner", 10);
        assert_eq!(hits.len(), 1);
    }
}
