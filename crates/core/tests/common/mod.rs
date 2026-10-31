#![allow(dead_code)]
use axioms_core::{Concept, ConceptGraph, ConceptId, Knowledge};

pub fn id(s: &str) -> ConceptId {
    ConceptId::from(s)
}

pub fn concept(name: &str) -> Concept {
    let mut c = Concept::new(id(&axioms_core::normalize(name)), name);
    c.statement = format!("{name} states something.");
    c
}

/// Build a graph from `(from, to)` name pairs.
pub fn graph(edges: &[(&str, &str)]) -> ConceptGraph {
    let mut g = ConceptGraph::new();
    for (a, b) in edges {
        for n in [a, b] {
            if g.get(&id(&axioms_core::normalize(n))).is_none() {
                g.upsert(concept(n));
            }
        }
        let key = |n: &str| id(&axioms_core::normalize(n));
        g.add_need(&key(a), &key(b)).unwrap();
    }
    g
}

pub fn set(g: &mut ConceptGraph, name: &str, k: Knowledge) {
    g.get_mut(&id(name)).unwrap().knowledge = k;
}
