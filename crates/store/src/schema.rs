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
pub const FIELD_TYPE: &str = "sys.f.fieldType";
pub const TEXT: &str = "sys.ft.text";
pub const REF: &str = "sys.ft.ref";
pub const TARGET_TAG: &str = "sys.f.targetTag";
pub const CARDINALITY: &str = "sys.f.cardinality";
pub const ONE: &str = "sys.cardinality.one";

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

/// The schema nodes, in a fixed order, typed as kb types its own fields:
/// prose fields are single text, `needs` refs only concepts, and the
/// knowledge field declares its allowed values by its own children (kb's
/// option-set shape).
pub fn nodes() -> Vec<KbNode> {
    let r = |id: &str| PropValue::Ref(id.into());
    let field = |id: &str, text: &str| KbNode::new(id, text).with(TYPE, r(FIELD));
    let prose = |id: &str, text: &str| {
        field(id, text)
            .with(FIELD_TYPE, r(TEXT))
            .with(CARDINALITY, r(ONE))
    };
    let mut knowledge = field(KNOWLEDGE, "knowledge")
        .with(FIELD_TYPE, r(REF))
        .with(CARDINALITY, r(ONE));
    let options = [Knowledge::Unknown, Knowledge::Learning, Knowledge::Known];
    knowledge.children = options
        .iter()
        .map(|k| knowledge_id(*k).to_owned())
        .collect();
    let mut out = vec![
        prose(STATEMENT, "statement"),
        prose(EXPLANATION, "explanation"),
        prose(CHECK, "check question"),
        field(NEEDS, "needs")
            .with(FIELD_TYPE, r(REF))
            .with(TARGET_TAG, r(CONCEPT)),
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
