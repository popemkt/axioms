//! The fields, tags and option nodes axioms writes, as kb-style nodes.
//!
//! `sys.*` ids are kb's own seeded nodes (referenced, never written here);
//! `ax.*` ids are axioms' schema. A concept node's id is its concept id
//! (a slug, which never contains a dot, so it cannot meet a schema id).

use crate::node::{KbNode, PropValue};
use axioms_core::Knowledge;

pub const TYPE: &str = "sys.f.type";
pub const FIELD: &str = "sys.field";
pub const TAG: &str = "sys.tag";
pub const FIELDS: &str = "sys.f.fields";

pub const STATEMENT: &str = "ax.f.statement";
pub const EXPLANATION: &str = "ax.f.explanation";
pub const CHECK: &str = "ax.f.check";
pub const NEEDS: &str = "ax.f.needs";
pub const KNOWLEDGE: &str = "ax.f.knowledge";
pub const CONCEPT: &str = "ax.tag.concept";
/// The outline node whose ordered children are the stated topics.
pub const TOPICS: &str = "ax.topics";

pub fn knowledge_id(k: Knowledge) -> &'static str {
    match k {
        Knowledge::Unknown => "ax.k.unknown",
        Knowledge::Learning => "ax.k.learning",
        Knowledge::Known => "ax.k.known",
    }
}

pub fn knowledge_of(id: &str) -> Option<Knowledge> {
    [Knowledge::Unknown, Knowledge::Learning, Knowledge::Known]
        .into_iter()
        .find(|k| knowledge_id(*k) == id)
}

/// The schema nodes, in a fixed order. The knowledge field declares its
/// allowed values by its own children (kb's option-set shape).
pub fn nodes() -> Vec<KbNode> {
    let field =
        |id: &str, text: &str| KbNode::new(id, text).with(TYPE, PropValue::Ref(FIELD.into()));
    let mut knowledge = field(KNOWLEDGE, "knowledge");
    let options = [Knowledge::Unknown, Knowledge::Learning, Knowledge::Known];
    knowledge.children = options
        .iter()
        .map(|k| knowledge_id(*k).to_owned())
        .collect();
    let mut out = vec![
        field(STATEMENT, "statement"),
        field(EXPLANATION, "explanation"),
        field(CHECK, "check question"),
        field(NEEDS, "needs"),
        knowledge,
    ];
    out.extend(options.iter().map(|k| {
        let text = format!("{k:?}").to_lowercase();
        KbNode::new(knowledge_id(*k), text)
    }));
    let mut concept = KbNode::new(CONCEPT, "concept").with(TYPE, PropValue::Ref(TAG.into()));
    for f in [STATEMENT, EXPLANATION, CHECK, NEEDS, KNOWLEDGE] {
        concept = concept.with(FIELDS, PropValue::Ref(f.into()));
    }
    out.push(concept);
    out
}
