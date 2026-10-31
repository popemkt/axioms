//! The blueprint view of one goal: what is drawn, and how.
//!
//! Walk down from the goal along "needs". A known concept is one of your
//! axioms: it is drawn, but nothing below it is (it is collapsed, and the
//! number of concepts it hides is reported).

use crate::concept::{Concept, ConceptId, Knowledge};
use crate::graph::ConceptGraph;
use serde::Serialize;
use std::collections::{BTreeSet, VecDeque};

/// How a node is drawn: the goal, or by what you know of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Goal,
    Unknown,
    Learning,
    Known,
}

#[derive(Debug, Clone, Serialize)]
pub struct ViewNode {
    #[serde(flatten)]
    pub concept: Concept,
    pub status: Status,
    /// How many concepts below this one are not drawn because it is known.
    pub hidden_below: usize,
    /// Whether this concept has been decomposed (has any prerequisites at all).
    pub decomposed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct View {
    pub goal: ConceptId,
    pub nodes: Vec<ViewNode>,
    pub edges: Vec<(ConceptId, ConceptId)>,
}

/// Does the walk continue below this concept?
fn expands(c: &Concept) -> bool {
    c.knowledge != Knowledge::Known
}

/// Everything reachable below `from`, excluding `from` itself.
fn descendants<'g>(graph: &'g ConceptGraph, from: &ConceptId) -> BTreeSet<&'g ConceptId> {
    let mut seen = BTreeSet::new();
    let mut queue: VecDeque<&ConceptId> = graph.needs(from).into_iter().collect();
    while let Some(id) = queue.pop_front() {
        if seen.insert(id) {
            queue.extend(graph.needs(id));
        }
    }
    seen
}

/// Project the graph below `goal`. `None` if the goal is not in the graph.
pub fn view(graph: &ConceptGraph, goal: &ConceptId) -> Option<View> {
    graph.get(goal)?;
    let mut visible: BTreeSet<&ConceptId> = BTreeSet::new();
    let mut edges = Vec::new();
    let mut queue = VecDeque::from([goal]);
    while let Some(id) = queue.pop_front() {
        if !visible.insert(id) {
            continue;
        }
        let concept = graph.get(id)?;
        if expands(concept) {
            for need in graph.needs(id) {
                edges.push((id.clone(), need.clone()));
                queue.push_back(need);
            }
        }
    }
    edges.sort();
    let nodes = visible
        .iter()
        .filter_map(|id| graph.get(id))
        .map(|c| ViewNode {
            status: status_of(c, goal),
            hidden_below: if expands(c) {
                0
            } else {
                descendants(graph, &c.id)
                    .into_iter()
                    .filter(|d| !visible.contains(d))
                    .count()
            },
            decomposed: !graph.needs(&c.id).is_empty(),
            concept: c.clone(),
        })
        .collect();
    Some(View {
        goal: goal.clone(),
        nodes,
        edges,
    })
}

fn status_of(c: &Concept, goal: &ConceptId) -> Status {
    match c.knowledge {
        _ if &c.id == goal => Status::Goal,
        Knowledge::Unknown => Status::Unknown,
        Knowledge::Learning => Status::Learning,
        Knowledge::Known => Status::Known,
    }
}
