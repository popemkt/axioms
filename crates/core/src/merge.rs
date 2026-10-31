//! Merging a decomposition into the graph, reusing concepts rather than
//! duplicating them.
//!
//! Two phases keep this pure: [`Decomposition::collisions`] lists proposals
//! whose normalised name meets an existing concept; someone (the agent's
//! "same concept?" judge) resolves each; [`Decomposition::apply`] then writes.

use crate::concept::{Concept, ConceptId, normalize};
use crate::graph::ConceptGraph;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A proposed prerequisite concept, as a decomposition states it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub name: String,
    pub statement: String,
    pub explanation: String,
    pub check: String,
    /// Names of other proposals (or existing concepts) this one needs.
    #[serde(default)]
    pub needs: Vec<String>,
}

/// What a decomposition of one target concept returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decomposition {
    /// The target restated: used to fill a target that has no text yet.
    pub target: Proposal,
    /// The prerequisite concepts.
    pub concepts: Vec<Proposal>,
}

/// A proposal whose name meets concepts already in the graph.
#[derive(Debug, Clone, Serialize)]
pub struct Collision {
    pub proposal: Proposal,
    pub existing: Vec<Concept>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// The proposal is this existing concept: link to it.
    Same(ConceptId),
    /// The proposal is a different concept that happens to share a name.
    Distinct,
}

/// What [`Decomposition::apply`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Merged {
    pub added: Vec<ConceptId>,
    pub reused: Vec<ConceptId>,
    /// Edges dropped because they would have closed a cycle.
    pub dropped_edges: usize,
}

impl Decomposition {
    /// Proposals, deduplicated by normalised name, minus the target itself.
    fn proposals(&self, target: &Concept) -> BTreeMap<String, &Proposal> {
        let own = normalize(&target.name);
        let mut out = BTreeMap::new();
        for p in &self.concepts {
            let key = normalize(&p.name);
            if !key.is_empty() && key != own {
                out.entry(key).or_insert(p);
            }
        }
        out
    }

    pub fn collisions(&self, graph: &ConceptGraph, target: &Concept) -> Vec<Collision> {
        self.proposals(target)
            .into_values()
            .filter_map(|p| {
                let existing: Vec<Concept> = graph.named(&p.name).into_iter().cloned().collect();
                (!existing.is_empty()).then(|| Collision {
                    proposal: p.clone(),
                    existing,
                })
            })
            .collect()
    }

    /// Write the decomposition below `target`. `resolutions` is keyed by the
    /// proposal's normalised name; an unresolved collision counts as distinct.
    pub fn apply(
        &self,
        graph: &mut ConceptGraph,
        target: &ConceptId,
        resolutions: &BTreeMap<String, Resolution>,
    ) -> Merged {
        let mut merged = Merged::default();
        let Some(tc) = graph.get(target).cloned() else {
            return merged;
        };
        if tc.statement.is_empty() {
            let mut filled = tc.clone();
            fill(&mut filled, &self.target);
            graph.upsert(filled);
        }
        let mut ids: BTreeMap<String, ConceptId> = BTreeMap::new();
        ids.insert(normalize(&tc.name), target.clone());
        for (key, p) in self.proposals(&tc) {
            let id = match resolutions.get(&key) {
                Some(Resolution::Same(id)) if graph.get(id).is_some() => {
                    merged.reused.push(id.clone());
                    id.clone()
                }
                _ => {
                    let id = graph.fresh_id(&p.name);
                    let mut c = Concept::new(id.clone(), p.name.trim());
                    fill(&mut c, p);
                    graph.upsert(c);
                    merged.added.push(id.clone());
                    id
                }
            };
            ids.insert(key, id);
        }
        let resolve = |name: &str, graph: &ConceptGraph| -> Option<ConceptId> {
            ids.get(&normalize(name)).cloned().or_else(|| {
                let found = graph.named(name);
                (found.len() == 1).then(|| found[0].id.clone())
            })
        };
        let mut edges: Vec<(ConceptId, ConceptId)> = Vec::new();
        for need in &self.target.needs {
            if let Some(to) = resolve(need, graph) {
                edges.push((target.clone(), to));
            }
        }
        for p in self.proposals(&tc).values() {
            let from = ids[&normalize(&p.name)].clone();
            for need in &p.needs {
                if let Some(to) = resolve(need, graph) {
                    edges.push((from.clone(), to));
                }
            }
        }
        for (from, to) in edges {
            if graph.add_need(&from, &to).is_err() {
                merged.dropped_edges += 1;
            }
        }
        merged
    }
}

fn fill(c: &mut Concept, p: &Proposal) {
    c.statement = p.statement.trim().to_owned();
    c.explanation = p.explanation.trim().to_owned();
    c.check = p.check.trim().to_owned();
}
