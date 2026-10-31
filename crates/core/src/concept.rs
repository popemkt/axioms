//! A concept: one idea, its canonical one-line statement, and what you know of it.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Stable concept id: the normalised name, plus a `-N` suffix when two
/// distinct concepts share a name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConceptId(pub String);

impl ConceptId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConceptId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for ConceptId {
    fn from(s: &str) -> Self {
        ConceptId(s.to_owned())
    }
}

/// How well you know a concept. `Known` makes it one of your axioms.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Knowledge {
    #[default]
    Unknown,
    Learning,
    Known,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Concept {
    pub id: ConceptId,
    pub name: String,
    /// The canonical one-line statement: this concept's single home for "what it says".
    pub statement: String,
    /// A short explanation, written assuming the prerequisites are known.
    pub explanation: String,
    /// A question that checks understanding.
    pub check: String,
    pub knowledge: Knowledge,
}

impl Concept {
    pub fn new(id: ConceptId, name: impl Into<String>) -> Self {
        Concept {
            id,
            name: name.into(),
            statement: String::new(),
            explanation: String::new(),
            check: String::new(),
            knowledge: Knowledge::Unknown,
        }
    }
}

const ARTICLES: [&str; 3] = ["the", "a", "an"];

/// Normalise a concept name into its id form: lowercase ASCII words joined by
/// `-`, punctuation dropped, a leading article dropped. Non-ASCII letters are
/// kept so "Gödel" and "Gödel's" still meet.
pub fn normalize(name: &str) -> String {
    let lowered = name.to_lowercase().replace(['\'', '’'], "");
    let words: Vec<&str> = lowered
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let start = usize::from(words.len() > 1 && ARTICLES.contains(&words[0]));
    words[start..].join("-")
}
