mod common;
use axioms_core::{ConceptGraph, Decomposition, Proposal, Resolution};
use common::*;
use std::collections::BTreeMap;

fn p(name: &str, needs: &[&str]) -> Proposal {
    Proposal {
        name: name.into(),
        statement: format!("{name} is a thing."),
        explanation: "Because.".into(),
        check: "Why?".into(),
        needs: needs.iter().map(|s| s.to_string()).collect(),
    }
}

fn decomposition(target: &str, needs: &[&str], concepts: Vec<Proposal>) -> Decomposition {
    Decomposition {
        target: p(target, needs),
        concepts,
    }
}

fn with_goal(name: &str) -> ConceptGraph {
    let mut g = ConceptGraph::new();
    g.upsert(axioms_core::Concept::new(
        id(&axioms_core::normalize(name)),
        name,
    ));
    g
}

#[test]
fn apply_adds_concepts_edges_and_fills_the_target() {
    let mut g = with_goal("Attention");
    let d = decomposition(
        "Attention",
        &["Softmax", "Dot product"],
        vec![
            p("Softmax", &["Exponential"]),
            p("Dot product", &[]),
            p("Exponential", &[]),
        ],
    );
    assert!(
        d.collisions(&g, g.get(&id("attention")).unwrap())
            .is_empty()
    );
    let m = d.apply(&mut g, &id("attention"), &BTreeMap::new());
    assert_eq!(m.added.len(), 3);
    assert_eq!(
        g.get(&id("attention")).unwrap().statement,
        "Attention is a thing."
    );
    assert_eq!(
        g.needs(&id("attention")),
        vec![&id("dot-product"), &id("softmax")]
    );
    assert_eq!(g.needs(&id("softmax")), vec![&id("exponential")]);
}

#[test]
fn collisions_resolve_to_reuse_or_a_distinct_concept() {
    let mut g = graph(&[("old goal", "Group"), ("old goal", "Ring")]);
    g.upsert(axioms_core::Concept::new(id("flt"), "FLT"));
    let d = decomposition(
        "FLT",
        &["group", "ring", "FLT"],
        vec![p("group", &[]), p("ring", &[]), p("FLT", &[])],
    );
    let target = g.get(&id("flt")).unwrap().clone();
    let cs = d.collisions(&g, &target);
    let names: Vec<_> = cs.iter().map(|c| c.proposal.name.as_str()).collect();
    assert_eq!(
        names,
        ["group", "ring"],
        "the target itself is never a collision"
    );
    let res = BTreeMap::from([
        ("group".to_string(), Resolution::Same(id("group"))),
        ("ring".to_string(), Resolution::Distinct),
    ]);
    let m = d.apply(&mut g, &id("flt"), &res);
    assert_eq!(m.reused, vec![id("group")]);
    assert_eq!(m.added, vec![id("ring-2")]);
    assert_eq!(g.needs(&id("flt")), vec![&id("group"), &id("ring-2")]);
    assert_eq!(
        g.get(&id("group")).unwrap().statement,
        "Group states something."
    );
}

#[test]
fn cycle_closing_edges_are_dropped() {
    let mut g = with_goal("A");
    let d = decomposition("A", &["B"], vec![p("B", &["C"]), p("C", &["B"])]);
    let m = d.apply(&mut g, &id("a"), &BTreeMap::new());
    assert_eq!(m.dropped_edges, 1);
    assert_eq!(g.edges().len(), 2);
}
