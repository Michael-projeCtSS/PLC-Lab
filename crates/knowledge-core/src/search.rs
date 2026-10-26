//! Deterministic in-memory full-text index (derived data, rebuilt at startup).
//!
//! BM25-style scoring over weighted fields with prefix matching for longer terms and a
//! bonus when consecutive query terms appear consecutively. Ties break by identifier.

use crate::model::Category;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedDocument {
    pub id: String,
    pub doc_type: String,
    pub category: Category,
    pub title: String,
    pub summary: String,
    pub vendor: Option<String>,
    pub family: Option<String>,
    pub protocol: Option<String>,
    /// Weighted text fields: (weight, text).
    pub fields: Vec<(u8, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub id: String,
    pub doc_type: String,
    pub category: Category,
    pub title: String,
    pub summary: String,
    pub vendor: Option<String>,
    pub score: f64,
    pub snippet: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchQuery {
    pub text: String,
    pub category: Option<Category>,
    pub vendor: Option<String>,
    pub doc_type: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SearchIndex {
    documents: Vec<IndexedDocument>,
    /// term -> postings (document, weighted term frequency)
    postings: BTreeMap<String, Vec<(usize, u32)>>,
    /// token sequences per document for phrase matching (title + summary + tags only).
    sequences: Vec<Vec<String>>,
    lengths: Vec<u32>,
    average_length: f64,
}

/// Lowercase alphanumeric tokens. Hyphenated or dotted compounds (4-20, s7-1500, 1.0)
/// are kept whole and also split into parts.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '%')) {
        let word = raw.trim_matches(|c: char| c == '-' || c == '.' || c == '_').to_lowercase();
        if word.is_empty() {
            continue;
        }
        let parts: Vec<&str> = word.split(['-', '.', '_']).filter(|p| !p.is_empty()).collect();
        if parts.len() > 1 {
            out.push(word.clone());
            out.extend(parts.into_iter().map(str::to_owned));
        } else {
            out.push(word);
        }
    }
    out
}

const STOP_WORDS: [&str; 18] = [
    "a", "an", "the", "of", "to", "in", "on", "is", "and", "or", "for", "with", "how", "what", "why", "does", "my", "vs",
];

impl SearchIndex {
    pub fn build(documents: Vec<IndexedDocument>) -> Self {
        let mut postings: BTreeMap<String, BTreeMap<usize, u32>> = BTreeMap::new();
        let mut lengths = Vec::with_capacity(documents.len());
        let mut sequences = Vec::with_capacity(documents.len());
        for (index, doc) in documents.iter().enumerate() {
            let mut length = 0u32;
            let mut seq = Vec::new();
            let mut fields = vec![(5u8, doc.title.clone()), (3u8, doc.summary.clone())];
            fields.extend(doc.fields.iter().cloned());
            for (weight, text) in &fields {
                for token in tokenize(text) {
                    length += 1;
                    *postings.entry(token.clone()).or_default().entry(index).or_default() += u32::from(*weight);
                    if *weight >= 3 {
                        seq.push(token);
                    }
                }
            }
            lengths.push(length.max(1));
            sequences.push(seq);
        }
        let average_length = if lengths.is_empty() {
            1.0
        } else {
            lengths.iter().map(|l| f64::from(*l)).sum::<f64>() / lengths.len() as f64
        };
        Self {
            postings: postings.into_iter().map(|(t, m)| (t, m.into_iter().collect())).collect(),
            documents,
            sequences,
            lengths,
            average_length,
        }
    }
    pub fn len(&self) -> usize {
        self.documents.len()
    }
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
    pub fn documents(&self) -> &[IndexedDocument] {
        &self.documents
    }

    pub fn search(&self, query: &SearchQuery) -> Vec<SearchHit> {
        let terms: Vec<String> = tokenize(&query.text)
            .into_iter()
            .filter(|t| !STOP_WORDS.contains(&t.as_str()))
            .collect();
        let allowed = |d: &IndexedDocument| {
            query.category.is_none_or(|c| c == d.category)
                && query.vendor.as_ref().is_none_or(|v| d.vendor.as_deref() == Some(v.as_str()))
                && query.doc_type.as_ref().is_none_or(|t| &d.doc_type == t)
        };
        let n = self.documents.len() as f64;
        let mut scores: BTreeMap<usize, f64> = BTreeMap::new();
        if terms.is_empty() {
            for (i, d) in self.documents.iter().enumerate() {
                if allowed(d) {
                    scores.insert(i, 0.0);
                }
            }
        }
        let unique: BTreeSet<&String> = terms.iter().collect();
        for term in unique {
            // Exact matches score fully; prefix matches (terms of 3+ characters) score less.
            let mut matched: Vec<(&String, f64)> = Vec::new();
            if self.postings.contains_key(term) {
                matched.push((term, 1.0));
            }
            if term.chars().count() >= 3 {
                for (key, _) in self.postings.range(term.clone()..).take_while(|(k, _)| k.starts_with(term.as_str())) {
                    if key != term {
                        matched.push((key, 0.6));
                    }
                }
            }
            for (key, factor) in matched.into_iter().take(40) {
                let postings = &self.postings[key];
                let df = postings.len() as f64;
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                for (doc, tf) in postings {
                    if !allowed(&self.documents[*doc]) {
                        continue;
                    }
                    let tf = f64::from(*tf);
                    let norm = 1.2 * (0.25 + 0.75 * f64::from(self.lengths[*doc]) / self.average_length);
                    *scores.entry(*doc).or_default() += factor * idf * (tf * 2.2) / (tf + norm);
                }
            }
        }
        // Phrase bonus for consecutive query terms in title/summary/tags.
        if terms.len() > 1 {
            for (doc, score) in scores.iter_mut() {
                let seq = &self.sequences[*doc];
                let bonus = terms
                    .windows(2)
                    .filter(|w| seq.windows(2).any(|s| s[0] == w[0] && s[1] == w[1]))
                    .count();
                *score += bonus as f64 * 1.5;
            }
        }
        let mut hits: Vec<(usize, f64)> = scores.into_iter().filter(|(_, s)| terms.is_empty() || *s > 0.0).collect();
        hits.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| self.documents[a.0].id.cmp(&self.documents[b.0].id)));
        let limit = if query.limit == 0 { 20 } else { query.limit.min(100) };
        hits.into_iter()
            .take(limit)
            .map(|(i, score)| {
                let d = &self.documents[i];
                SearchHit {
                    id: d.id.clone(),
                    doc_type: d.doc_type.clone(),
                    category: d.category,
                    title: d.title.clone(),
                    summary: d.summary.clone(),
                    vendor: d.vendor.clone(),
                    score,
                    snippet: snippet(d, &terms),
                }
            })
            .collect()
    }
}

fn snippet(doc: &IndexedDocument, terms: &[String]) -> String {
    for (_, text) in &doc.fields {
        for sentence in text.split(['.', '\n']) {
            let lower = sentence.to_lowercase();
            if terms.iter().any(|t| t.len() >= 2 && lower.contains(t.as_str())) {
                let trimmed = sentence.trim().trim_start_matches(['-', '*', '#', ' ']);
                if trimmed.len() > 12 {
                    let mut s: String = trimmed.chars().take(220).collect();
                    if trimmed.chars().count() > 220 {
                        s.push('…');
                    }
                    return s;
                }
            }
        }
    }
    doc.summary.chars().take(220).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn doc(id: &str, title: &str, body: &str) -> IndexedDocument {
        IndexedDocument {
            id: id.into(),
            doc_type: "topic".into(),
            category: Category::Fundamental,
            title: title.into(),
            summary: String::new(),
            vendor: None,
            family: None,
            protocol: None,
            fields: vec![(1, body.into())],
        }
    }
    #[test]
    fn tokenizer_keeps_compounds_and_parts() {
        assert_eq!(tokenize("4-20 mA S7-1500"), ["4-20", "4", "20", "ma", "s7-1500", "s7", "1500"]);
        assert_eq!(tokenize("Modbus 40001 offset."), ["modbus", "40001", "offset"]);
    }
    #[test]
    fn ranking_is_deterministic_and_relevant() {
        let index = SearchIndex::build(vec![
            doc("a", "4-20 mA current loop", "Live zero detects a broken loop at 0 mA."),
            doc("b", "Modbus register addressing", "Holding register 40001 is protocol address 0."),
            doc("c", "PROFINET device name", "The controller assigns IP addresses by device name."),
        ]);
        let q = |t: &str| SearchQuery { text: t.into(), limit: 5, ..Default::default() };
        assert_eq!(index.search(&q("4-20 mA 0 value"))[0].id, "a");
        assert_eq!(index.search(&q("Modbus 40001 offset"))[0].id, "b");
        assert_eq!(index.search(&q("profin"))[0].id, "c", "prefix match");
        assert_eq!(index.search(&q("PROFINET device name")), index.search(&q("PROFINET device name")));
        assert!(index.search(&q("zzzz")).is_empty());
    }
}
