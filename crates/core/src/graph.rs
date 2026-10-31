//! The concept graph: concepts as nodes, "needs" as edges (A → B: A needs B).
//! Acyclic by construction: an edge that would close a cycle is refused.

use crate::concept::{Concept, ConceptId, normalize};
use petgraph::Direction;
use petgraph::algo::has_path_connecting;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GraphError {
    #[error("unknown concept `{0}`")]
    Unknown(ConceptId),
    #[error("`{0}` needing `{1}` would close a cycle")]
    Cycle(ConceptId, ConceptId),
}

#[derive(Debug, Clone, Default)]
pub struct ConceptGraph {
    concepts: BTreeMap<ConceptId, Concept>,
    index: HashMap<ConceptId, NodeIndex>,
    edges: StableDiGraph<ConceptId, ()>,
}

impl ConceptGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.concepts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.concepts.is_empty()
    }

    pub fn get(&self, id: &ConceptId) -> Option<&Concept> {
        self.concepts.get(id)
    }

    pub fn get_mut(&mut self, id: &ConceptId) -> Option<&mut Concept> {
        self.concepts.get_mut(id)
    }

    pub fn concepts(&self) -> impl Iterator<Item = &Concept> {
        self.concepts.values()
    }

    /// Insert a concept, or replace the one with the same id (keeping its edges).
    pub fn upsert(&mut self, concept: Concept) {
        let id = concept.id.clone();
        if !self.index.contains_key(&id) {
            let ix = self.edges.add_node(id.clone());
            self.index.insert(id.clone(), ix);
        }
        self.concepts.insert(id, concept);
    }

    /// Remove a concept that has no edges. Returns whether it was removed.
    pub fn remove_isolated(&mut self, id: &ConceptId) -> bool {
        let Some(&ix) = self.index.get(id) else {
            return false;
        };
        if self.edges.neighbors_undirected(ix).next().is_some() {
            return false;
        }
        self.edges.remove_node(ix);
        self.index.remove(id);
        self.concepts.remove(id);
        true
    }

    /// Record that `from` needs `to`. Idempotent; refuses self-loops and cycles.
    pub fn add_need(&mut self, from: &ConceptId, to: &ConceptId) -> Result<(), GraphError> {
        let a = self.ix(from)?;
        let b = self.ix(to)?;
        if a == b || has_path_connecting(&self.edges, b, a, None) {
            return Err(GraphError::Cycle(from.clone(), to.clone()));
        }
        self.edges.update_edge(a, b, ());
        Ok(())
    }

    /// What `id` needs, sorted by id for determinism.
    pub fn needs(&self, id: &ConceptId) -> Vec<&ConceptId> {
        self.neighbors(id, Direction::Outgoing)
    }

    /// What needs `id`, sorted by id.
    pub fn needed_by(&self, id: &ConceptId) -> Vec<&ConceptId> {
        self.neighbors(id, Direction::Incoming)
    }

    /// Every edge as (from, to), sorted.
    pub fn edges(&self) -> Vec<(&ConceptId, &ConceptId)> {
        let mut out: Vec<_> = self
            .edges
            .edge_indices()
            .filter_map(|e| self.edges.edge_endpoints(e))
            .map(|(a, b)| (&self.edges[a], &self.edges[b]))
            .collect();
        out.sort();
        out
    }

    /// Concepts whose name normalises to `name`'s normal form, including
    /// suffixed ids (`name-2`) that share the same display name.
    pub fn named(&self, name: &str) -> Vec<&Concept> {
        let key = normalize(name);
        self.concepts
            .values()
            .filter(|c| normalize(&c.name) == key)
            .collect()
    }

    /// A fresh id for `name`: its normal form, suffixed if already taken.
    pub fn fresh_id(&self, name: &str) -> ConceptId {
        let base = match normalize(name) {
            s if s.is_empty() => "concept".to_owned(),
            s => s,
        };
        let mut id = ConceptId(base.clone());
        let mut n = 2;
        while self.concepts.contains_key(&id) {
            id = ConceptId(format!("{base}-{n}"));
            n += 1;
        }
        id
    }

    fn ix(&self, id: &ConceptId) -> Result<NodeIndex, GraphError> {
        self.index
            .get(id)
            .copied()
            .ok_or_else(|| GraphError::Unknown(id.clone()))
    }

    fn neighbors(&self, id: &ConceptId, dir: Direction) -> Vec<&ConceptId> {
        let Some(&ix) = self.index.get(id) else {
            return Vec::new();
        };
        let mut out: Vec<_> = self
            .edges
            .neighbors_directed(ix, dir)
            .map(|n| &self.edges[n])
            .collect();
        out.sort();
        out.dedup();
        out
    }
}
