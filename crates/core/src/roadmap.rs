//! The roadmap: the order to learn the unknown part of a view, from your known
//! frontier up to the goal.
//!
//! It is derived from the [`View`], never stored. The order is a depth-first
//! post-order from the goal over concepts not yet known: each concept comes
//! right after its own unknown prerequisites, so a branch is finished before
//! the next begins (a valid topological order that reads as a path, not as
//! layers). Siblings go simplest first: by height (how far a concept sits
//! above its unknown leaves), then by name.

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

struct Walk<'v> {
    nodes: BTreeMap<&'v ConceptId, &'v ViewNode>,
    needs: BTreeMap<&'v ConceptId, Vec<&'v ConceptId>>,
    heights: BTreeMap<&'v ConceptId, usize>,
    placed: BTreeSet<&'v ConceptId>,
    steps: Vec<Step>,
}

impl<'v> Walk<'v> {
    fn unknown(&self, id: &ConceptId) -> bool {
        self.nodes
            .get(id)
            .is_some_and(|n| n.concept.knowledge != Knowledge::Known)
    }

    fn needs_of(&self, id: &ConceptId) -> Vec<&'v ConceptId> {
        self.needs.get(id).cloned().unwrap_or_default()
    }

    fn height(&mut self, id: &'v ConceptId) -> usize {
        if let Some(&h) = self.heights.get(id) {
            return h;
        }
        let kids: Vec<_> = self
            .needs_of(id)
            .into_iter()
            .filter(|n| self.unknown(n))
            .collect();
        let h = kids
            .into_iter()
            .map(|n| 1 + self.height(n))
            .max()
            .unwrap_or(0);
        self.heights.insert(id, h);
        h
    }

    fn visit(&mut self, id: &'v ConceptId) {
        if !self.unknown(id) || !self.placed.insert(id) {
            return;
        }
        let (mut after, builds_on): (Vec<_>, Vec<_>) =
            self.needs_of(id).into_iter().partition(|n| self.unknown(n));
        let mut keyed: Vec<_> = after
            .iter()
            .map(|n| (self.height(n), &self.nodes[*n].concept.name, *n))
            .collect();
        keyed.sort();
        after = keyed.into_iter().map(|(_, _, n)| n).collect();
        for n in &after {
            self.visit(n);
        }
        let c = &self.nodes[id].concept;
        self.steps.push(Step {
            id: id.clone(),
            name: c.name.clone(),
            statement: c.statement.clone(),
            explanation: c.explanation.clone(),
            check: c.check.clone(),
            knowledge: c.knowledge,
            builds_on: builds_on.into_iter().cloned().collect(),
            after: after.into_iter().cloned().collect(),
        });
    }
}

pub fn roadmap(view: &View) -> Roadmap {
    let mut needs: BTreeMap<&ConceptId, Vec<&ConceptId>> = BTreeMap::new();
    for (from, to) in &view.edges {
        needs.entry(from).or_default().push(to);
    }
    let mut walk = Walk {
        nodes: view.nodes.iter().map(|n| (&n.concept.id, n)).collect(),
        needs,
        heights: BTreeMap::new(),
        placed: BTreeSet::new(),
        steps: Vec::new(),
    };
    walk.visit(&view.goal);
    Roadmap {
        goal: view.goal.clone(),
        steps: walk.steps,
    }
}
