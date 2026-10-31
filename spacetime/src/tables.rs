//! The tables. All are public: the graph is shared, and on a tailnet what a
//! person knows is not secret, only theirs (see DESIGN.md → SpacetimeDB).

use spacetimedb::{Identity, SpacetimeType, Timestamp, table};

/// A concept of the shared graph; its id is the slug axioms-core mints.
#[table(accessor = concept, public)]
pub struct Concept {
    #[primary_key]
    pub id: String,
    pub name: String,
    pub statement: String,
    pub explanation: String,
    pub check: String,
    /// Who last wrote it, and when.
    pub by: Identity,
    pub at: Timestamp,
}

/// `from_id` needs `to_id`. The key is `from>to`.
#[table(accessor = need, public)]
pub struct Need {
    #[primary_key]
    pub key: String,
    #[index(btree)]
    pub from_id: String,
    pub to_id: String,
}

/// A stated topic: a concept someone set out to learn. `seq` orders them:
/// the most recently stated is the highest.
#[table(accessor = topic, public)]
pub struct Topic {
    #[primary_key]
    pub concept: String,
    #[unique]
    #[auto_inc]
    pub seq: u64,
    pub by: Identity,
    pub at: Timestamp,
}

/// What one person knows of one concept. The key is `<identity hex>:<id>`;
/// no row means unknown.
#[table(accessor = knowledge, public)]
pub struct Knowledge {
    #[primary_key]
    pub key: String,
    #[index(btree)]
    pub person: Identity,
    pub concept: String,
    pub state: String,
}

/// A decomposition asked for: `topic` (text is the topic) or `decompose`
/// (text is a concept id). pending → working (claimed) → done | failed.
#[table(accessor = request, public)]
pub struct Request {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub kind: String,
    pub text: String,
    pub goal: String,
    pub depth: u8,
    pub by: Identity,
    #[index(btree)]
    pub status: String,
    /// The concept it resolved to (a topic's id), once the worker knows.
    pub concept: String,
    pub worker: Option<Identity>,
    pub note: String,
    pub at: Timestamp,
}

#[derive(SpacetimeType)]
pub struct ConceptIn {
    pub id: String,
    pub name: String,
    pub statement: String,
    pub explanation: String,
    pub check: String,
}

#[derive(SpacetimeType)]
pub struct NeedIn {
    pub from_id: String,
    pub to_id: String,
}
