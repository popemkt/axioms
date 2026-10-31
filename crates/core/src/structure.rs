//! Structure of a drawn view, for display: how deep each concept sits below
//! the goal, which section (top-level subtree) it belongs to, how many drawn
//! concepts need it, and which edges are implied by longer paths.
//!
//! A **section** is one of the goal's direct prerequisites (in the
//! transitive reduction) with what is drawn below it, like the Mazur /
//! Ribet / Wiles panels of the FLT blueprint. A concept below several
//! section roots joins the nearest one. A tie goes to the section most of
//! its dependants are in (so a branch keeps one colour), then to the section
//! with fewer members so far (so panels stay balanced), then by the root's
//! id; concepts nearest the goal are placed first. Every concept but the goal has a section,
//! as every branch of the FLT video has a colour.

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

/// Fewest steps from `from` to everything below it, `from` included.
fn distances<'e>(from: &'e ConceptId, needs: &Adj<'e>) -> BTreeMap<&'e ConceptId, usize> {
    let mut dist = BTreeMap::from([(from, 0)]);
    let mut queue = VecDeque::from([from]);
    while let Some(id) = queue.pop_front() {
        let d = dist[id];
        for n in needs.get(id).into_iter().flatten() {
            if !dist.contains_key(n) {
                dist.insert(n, d + 1);
                queue.push_back(n);
            }
        }
    }
    dist
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
    let implied: Vec<(ConceptId, ConceptId)> = edges
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
    let mut roots: Vec<&ConceptId> = needs.get(goal).cloned().unwrap_or_default();
    roots.retain(|r| !implied.iter().any(|(a, b)| a == goal && b == *r));
    // For each concept: its distance to the nearest root, and those roots.
    let mut nearest: BTreeMap<&ConceptId, (usize, Vec<&ConceptId>)> = BTreeMap::new();
    for root in &roots {
        for (id, d) in distances(root, &needs) {
            let e = nearest.entry(id).or_insert((d, Vec::new()));
            if d < e.0 {
                *e = (d, Vec::new());
            }
            if d == e.0 {
                e.1.push(root);
            }
        }
    }
    let mut order: Vec<_> = nearest.into_iter().collect();
    order.sort_by_key(|(id, (d, _))| (*d, *id));
    let mut size: BTreeMap<&ConceptId, usize> = BTreeMap::new();
    let mut section: BTreeMap<ConceptId, ConceptId> = BTreeMap::new();
    for (id, (_, tied)) in order {
        let pull = |r: &ConceptId| {
            edges
                .iter()
                .filter(|(from, to)| to == id && section.get(from) == Some(r))
                .count()
        };
        let root = *tied
            .iter()
            .min_by_key(|r| {
                (
                    std::cmp::Reverse(pull(r)),
                    size.get(**r).copied().unwrap_or(0),
                    **r,
                )
            })
            .expect("a reached concept has a root");
        *size.entry(root).or_default() += 1;
        section.insert(id.clone(), root.clone());
    }
    Structure {
        depth,
        section,
        dependants,
        implied,
    }
}
