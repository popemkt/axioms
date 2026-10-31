//! Structure of a drawn view, for display: how deep each concept sits below
//! the goal, which section (top-level subtree) it belongs to, how many drawn
//! concepts need it, and which edges are implied by longer paths.
//!
//! A **section** is one of the goal's direct prerequisites together with
//! everything drawn below it, like the Mazur / Ribet / Wiles panels of the
//! FLT blueprint. A concept reached from two sections is shared: no section.

use crate::concept::ConceptId;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(crate) struct Structure {
    pub depth: BTreeMap<ConceptId, usize>,
    pub section: BTreeMap<ConceptId, ConceptId>,
    pub dependants: BTreeMap<ConceptId, usize>,
    pub implied: Vec<(ConceptId, ConceptId)>,
}

type Adj<'e> = BTreeMap<&'e ConceptId, Vec<&'e ConceptId>>;

fn below<'e>(from: &'e ConceptId, needs: &Adj<'e>) -> BTreeSet<&'e ConceptId> {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([from]);
    while let Some(id) = queue.pop_front() {
        if seen.insert(id) {
            queue.extend(needs.get(id).into_iter().flatten());
        }
    }
    seen
}

pub(crate) fn structure(goal: &ConceptId, edges: &[(ConceptId, ConceptId)]) -> Structure {
    let mut needs: Adj = BTreeMap::new();
    let mut dependants: BTreeMap<ConceptId, usize> = BTreeMap::new();
    for (from, to) in edges {
        needs.entry(from).or_default().push(to);
        *dependants.entry(to.clone()).or_default() += 1;
    }
    let mut depth = BTreeMap::from([(goal.clone(), 0)]);
    let mut queue = VecDeque::from([goal]);
    while let Some(id) = queue.pop_front() {
        let d = depth[id];
        for n in needs.get(id).into_iter().flatten() {
            if !depth.contains_key(*n) {
                depth.insert((*n).clone(), d + 1);
                queue.push_back(n);
            }
        }
    }
    let mut owners: BTreeMap<&ConceptId, Vec<&ConceptId>> = BTreeMap::new();
    for root in needs.get(goal).into_iter().flatten() {
        for id in below(root, &needs) {
            owners.entry(id).or_default().push(root);
        }
    }
    let section = owners
        .into_iter()
        .filter(|(_, roots)| roots.len() == 1)
        .map(|(id, roots)| (id.clone(), roots[0].clone()))
        .collect();
    let implied = edges
        .iter()
        .filter(|(from, to)| {
            needs
                .get(from)
                .into_iter()
                .flatten()
                .any(|mid| *mid != to && below(mid, &needs).contains(to))
        })
        .cloned()
        .collect();
    Structure {
        depth,
        section,
        dependants,
        implied,
    }
}
