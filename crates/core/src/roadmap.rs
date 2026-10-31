//! The roadmap: the order to learn the unknown part of a view, from your known
//! frontier up to the goal.
//!
//! It is derived from the [`View`], never stored: a topological order over the
//! drawn concepts that are not yet known, prerequisites first. Ties break by
//! height (how far a concept sits above its unknown leaves), then by name, so
//! the order is deterministic and starts with what you can learn right now.

use crate::concept::{ConceptId, Knowledge};
use crate::view::{View, ViewNode};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize)]
pub struct Step {
    pub id: ConceptId,
    pub name: String,
    pub statement: String,
    pub explanation: String,
    pub check: String,
    pub knowledge: Knowledge,
    /// Known prerequisites this step builds on: the axioms it starts from.
    pub builds_on: Vec<ConceptId>,
    /// Unknown prerequisites, each an earlier step.
    pub after: Vec<ConceptId>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Roadmap {
    pub goal: ConceptId,
    pub steps: Vec<Step>,
}

pub fn roadmap(view: &View) -> Roadmap {
    let nodes: BTreeMap<&ConceptId, &ViewNode> =
        view.nodes.iter().map(|n| (&n.concept.id, n)).collect();
    let unknown = |id: &ConceptId| {
        nodes
            .get(id)
            .is_some_and(|n| n.concept.knowledge != Knowledge::Known)
    };
    let mut needs: BTreeMap<&ConceptId, Vec<&ConceptId>> = BTreeMap::new();
    for (from, to) in &view.edges {
        needs.entry(from).or_default().push(to);
    }
    let todo: Vec<&ConceptId> = nodes.keys().copied().filter(|id| unknown(id)).collect();
    let height = heights(&todo, &needs, &unknown);

    let mut placed: BTreeSet<&ConceptId> = BTreeSet::new();
    let mut steps = Vec::with_capacity(todo.len());
    let pending_of = |id: &ConceptId, placed: &BTreeSet<&ConceptId>| {
        needs.get(id).map_or(0, |ns| {
            ns.iter()
                .filter(|n| unknown(n) && !placed.contains(*n))
                .count()
        })
    };
    while steps.len() < todo.len() {
        // The DAG guarantees at least one ready concept each round.
        let next = todo
            .iter()
            .filter(|id| !placed.contains(*id) && pending_of(id, &placed) == 0)
            .min_by_key(|id| (height[*id], &nodes[*id].concept.name))
            .copied()
            .expect("view edges are acyclic");
        placed.insert(next);
        let node = nodes[next];
        let (after, builds_on): (Vec<&ConceptId>, Vec<&ConceptId>) = needs
            .get(next)
            .into_iter()
            .flatten()
            .partition(|n| unknown(n));
        steps.push(Step {
            id: next.clone(),
            name: node.concept.name.clone(),
            statement: node.concept.statement.clone(),
            explanation: node.concept.explanation.clone(),
            check: node.concept.check.clone(),
            knowledge: node.concept.knowledge,
            builds_on: builds_on.into_iter().cloned().collect(),
            after: after.into_iter().cloned().collect(),
        });
    }
    Roadmap {
        goal: view.goal.clone(),
        steps,
    }
}

/// Longest path from each unknown concept down to an unknown leaf.
fn heights<'v>(
    todo: &[&'v ConceptId],
    needs: &BTreeMap<&'v ConceptId, Vec<&'v ConceptId>>,
    unknown: &impl Fn(&ConceptId) -> bool,
) -> BTreeMap<&'v ConceptId, usize> {
    fn go<'v>(
        id: &'v ConceptId,
        needs: &BTreeMap<&'v ConceptId, Vec<&'v ConceptId>>,
        unknown: &impl Fn(&ConceptId) -> bool,
        memo: &mut BTreeMap<&'v ConceptId, usize>,
    ) -> usize {
        if let Some(&h) = memo.get(id) {
            return h;
        }
        let h = needs
            .get(id)
            .into_iter()
            .flatten()
            .filter(|n| unknown(n))
            .map(|n| 1 + go(n, needs, unknown, memo))
            .max()
            .unwrap_or(0);
        memo.insert(id, h);
        h
    }
    let mut memo = BTreeMap::new();
    for id in todo {
        go(id, needs, unknown, &mut memo);
    }
    memo
}
