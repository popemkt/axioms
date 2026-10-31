//! What changed between two libraries, as additions: the shape a shared
//! store that only grows (SpacetimeDB) writes. Removals are not diffed; a
//! shared graph is not shrunk by one person's older copy of it.

use crate::concept::{Concept, ConceptId, Knowledge};
use crate::library::Library;
use std::collections::{BTreeSet, VecDeque};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Diff {
    /// New concepts, and concepts whose text changed.
    pub concepts: Vec<Concept>,
    /// New needs, `(from, to)`.
    pub needs: Vec<(ConceptId, ConceptId)>,
    pub topics: Vec<ConceptId>,
    /// Concepts whose knowledge changed, with the new state.
    pub knowledge: Vec<(ConceptId, Knowledge)>,
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        self.concepts.is_empty()
            && self.needs.is_empty()
            && self.topics.is_empty()
            && self.knowledge.is_empty()
    }

    /// The diff split into steps that grow the graph outward from `root`:
    /// each step adds one concept with the needs that reach it, so a view
    /// fed step by step grows node by node, never drawing a dangling edge.
    /// Changed (not new) concepts, topics and knowledge come in the first.
    pub fn steps(&self, root: &ConceptId, before: &Library) -> Vec<Diff> {
        let new: BTreeSet<&ConceptId> = self
            .concepts
            .iter()
            .filter(|c| before.graph.get(&c.id).is_none())
            .map(|c| &c.id)
            .collect();
        let mut first = Diff {
            concepts: self
                .concepts
                .iter()
                .filter(|c| !new.contains(&c.id))
                .cloned()
                .collect(),
            topics: self.topics.clone(),
            knowledge: self.knowledge.clone(),
            ..Diff::default()
        };
        let mut placed: BTreeSet<&ConceptId> = BTreeSet::new();
        let mut order: Vec<&ConceptId> = Vec::new();
        let mut queue = VecDeque::from([root]);
        let mut seen = BTreeSet::from([root]);
        while let Some(at) = queue.pop_front() {
            if new.contains(at) {
                order.push(at);
            }
            for (from, to) in &self.needs {
                if from == at && seen.insert(to) {
                    queue.push_back(to);
                }
            }
        }
        // New concepts the walk from root does not reach still get a step.
        order.extend(new.iter().filter(|id| !seen.contains(*id)));
        let present = |id: &ConceptId, placed: &BTreeSet<&ConceptId>| {
            !new.contains(id) || placed.contains(id)
        };
        let mut out = Vec::new();
        let mut pending: Vec<&(ConceptId, ConceptId)> = self.needs.iter().collect();
        let mut take_needs = |placed: &BTreeSet<&ConceptId>, d: &mut Diff| {
            pending.retain(|(f, t)| {
                let ready = present(f, placed) && present(t, placed);
                if ready {
                    d.needs.push((f.clone(), t.clone()));
                }
                !ready
            });
        };
        take_needs(&placed, &mut first);
        if !first.is_empty() {
            out.push(first);
        }
        for id in order {
            placed.insert(id);
            let mut step = Diff::default();
            step.concepts
                .extend(self.concepts.iter().find(|c| &c.id == id).cloned());
            take_needs(&placed, &mut step);
            out.push(step);
        }
        out
    }
}

/// What `after` adds to or changes in `before`.
pub fn diff(before: &Library, after: &Library) -> Diff {
    let mut d = Diff::default();
    for c in after.graph.concepts() {
        match before.graph.get(&c.id) {
            None => d.concepts.push(c.clone()),
            Some(b) => {
                if text(b) != text(c) {
                    d.concepts.push(c.clone());
                }
                if b.knowledge != c.knowledge {
                    d.knowledge.push((c.id.clone(), c.knowledge));
                }
            }
        }
        if before.graph.get(&c.id).is_none() && c.knowledge != Knowledge::Unknown {
            d.knowledge.push((c.id.clone(), c.knowledge));
        }
    }
    let had: BTreeSet<(&ConceptId, &ConceptId)> = before.graph.edges().into_iter().collect();
    d.needs = after
        .graph
        .edges()
        .into_iter()
        .filter(|e| !had.contains(e))
        .map(|(f, t)| (f.clone(), t.clone()))
        .collect();
    d.topics = after
        .topics
        .iter()
        .filter(|t| !before.topics.contains(t))
        .cloned()
        .collect();
    d
}

fn text(c: &Concept) -> (&str, &str, &str, &str) {
    (&c.name, &c.statement, &c.explanation, &c.check)
}
