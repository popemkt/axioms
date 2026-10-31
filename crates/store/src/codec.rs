//! Library ⇄ kb nodes. The only place that knows how a concept is spelled as
//! a node; both stores go through it.

use crate::node::{KbNode, PropValue};
use crate::schema::{self, CHECK, CONCEPT, EXPLANATION, KNOWLEDGE, NEEDS, STATEMENT, TOPICS, TYPE};
use anyhow::{Context, bail};
use axioms_core::{Concept, ConceptId, Library};

/// The library as nodes: schema first, then the topics outline, then
/// concepts sorted by id. Timestamps are left empty for the store to stamp.
pub fn to_nodes(library: &Library) -> Vec<KbNode> {
    let mut out = schema::nodes();
    let mut topics = KbNode::new(TOPICS, "topics");
    topics.children = library.topics.iter().map(|t| t.0.clone()).collect();
    out.push(topics);
    for c in library.graph.concepts() {
        let mut n = KbNode::new(c.id.0.clone(), c.name.clone())
            .with(TYPE, PropValue::Ref(CONCEPT.into()))
            .with(
                KNOWLEDGE,
                PropValue::Ref(schema::knowledge_id(c.knowledge).into()),
            );
        for (field, text) in [
            (STATEMENT, &c.statement),
            (EXPLANATION, &c.explanation),
            (CHECK, &c.check),
        ] {
            if !text.is_empty() {
                n = n.with(field, PropValue::Str(text.clone()));
            }
        }
        for need in library.graph.needs(&c.id) {
            n = n.with(NEEDS, PropValue::Ref(need.0.clone()));
        }
        out.push(n);
    }
    out
}

/// Rebuild a library from nodes. Nodes that are not concepts or the topics
/// outline are ignored, so the file can share a kb store with other nodes.
pub fn from_nodes(nodes: &[KbNode]) -> anyhow::Result<Library> {
    let mut library = Library::default();
    let concepts: Vec<&KbNode> = nodes
        .iter()
        .filter(|n| n.refs(TYPE).any(|t| t == CONCEPT))
        .collect();
    for n in &concepts {
        let knowledge = n
            .refs(KNOWLEDGE)
            .find_map(schema::knowledge_of)
            .unwrap_or_default();
        library.graph.upsert(Concept {
            id: ConceptId(n.id.clone()),
            name: n.text.clone(),
            statement: n.str(STATEMENT).to_owned(),
            explanation: n.str(EXPLANATION).to_owned(),
            check: n.str(CHECK).to_owned(),
            knowledge,
        });
    }
    for n in &concepts {
        for need in n.refs(NEEDS) {
            let (from, to) = (ConceptId(n.id.clone()), ConceptId(need.to_owned()));
            library
                .graph
                .add_need(&from, &to)
                .with_context(|| format!("loading edge {from} → {to}"))?;
        }
    }
    if let Some(t) = nodes.iter().find(|n| n.id == TOPICS) {
        for id in &t.children {
            if library.graph.get(&ConceptId(id.clone())).is_none() {
                bail!("topic `{id}` is not a concept");
            }
            library.topics.push(ConceptId(id.clone()));
        }
    }
    Ok(library)
}
