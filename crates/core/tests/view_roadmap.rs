mod common;
use axioms_core::{Knowledge, Status, roadmap, view};
use common::*;

/// goal → {x, y}; x → {a}; y → {a, b}; a → {deep}
fn sample() -> axioms_core::ConceptGraph {
    graph(&[
        ("goal", "x"),
        ("goal", "y"),
        ("x", "a"),
        ("y", "a"),
        ("y", "b"),
        ("a", "deep"),
    ])
}

fn order(g: &axioms_core::ConceptGraph) -> Vec<String> {
    let v = view(g, &id("goal")).unwrap();
    roadmap(&v).steps.into_iter().map(|s| s.id.0).collect()
}

#[test]
fn view_shows_everything_when_nothing_is_known() {
    let g = sample();
    let v = view(&g, &id("goal")).unwrap();
    assert_eq!(v.nodes.len(), 6);
    assert_eq!(v.edges.len(), 6);
    let goal = v.nodes.iter().find(|n| n.concept.id == id("goal")).unwrap();
    assert_eq!(goal.status, Status::Goal);
    assert!(view(&g, &id("missing")).is_none());
}

#[test]
fn known_concepts_collapse_what_is_below_them() {
    let mut g = sample();
    set(&mut g, "a", Knowledge::Known);
    let v = view(&g, &id("goal")).unwrap();
    let ids: Vec<_> = v.nodes.iter().map(|n| n.concept.id.0.as_str()).collect();
    assert_eq!(ids, ["a", "b", "goal", "x", "y"]);
    let a = v.nodes.iter().find(|n| n.concept.id == id("a")).unwrap();
    assert_eq!((a.status, a.hidden_below), (Status::Known, 1));
    assert!(!v.edges.iter().any(|(f, _)| f == &id("a")));
}

#[test]
fn roadmap_finishes_each_branch_before_the_next() {
    let g = sample();
    assert_eq!(order(&g), ["deep", "a", "x", "b", "y", "goal"]);
}

#[test]
fn roadmap_skips_known_and_records_what_steps_build_on() {
    let mut g = sample();
    set(&mut g, "a", Knowledge::Known);
    set(&mut g, "y", Knowledge::Learning);
    let v = view(&g, &id("goal")).unwrap();
    let r = roadmap(&v);
    let ids: Vec<_> = r.steps.iter().map(|s| s.id.0.as_str()).collect();
    assert_eq!(ids, ["x", "b", "y", "goal"]);
    let y = &r.steps[2];
    assert_eq!(y.builds_on, vec![id("a")]);
    assert_eq!(y.after, vec![id("b")]);
    assert_eq!(y.knowledge, Knowledge::Learning);
}

#[test]
fn known_goal_has_an_empty_roadmap() {
    let mut g = sample();
    set(&mut g, "goal", Knowledge::Known);
    let v = view(&g, &id("goal")).unwrap();
    assert_eq!(v.nodes.len(), 1);
    assert!(roadmap(&v).steps.is_empty());
}

#[test]
fn view_carries_depth_sections_dependants_and_implied_edges() {
    // goal → {x, y}; x → {a}; y → {a, b}; a → {deep}; plus a shortcut y → deep
    let mut g = sample();
    g.add_need(&id("y"), &id("deep")).unwrap();
    let v = view(&g, &id("goal")).unwrap();
    let n = |s: &str| v.nodes.iter().find(|n| n.concept.id == id(s)).unwrap();
    assert_eq!((n("goal").depth, n("a").depth, n("deep").depth), (0, 2, 2));
    assert_eq!(n("goal").section, None);
    assert_eq!(n("x").section, Some(id("x")));
    assert_eq!(n("b").section, Some(id("y")));
    assert_eq!(
        n("a").section,
        Some(id("x")),
        "a ties x and y, both of size 1: by id"
    );
    assert_eq!(n("a").dependants, 2);
    assert_eq!(v.implied, vec![(id("y"), id("deep"))]);
    assert_eq!(
        n("deep").section,
        Some(id("y")),
        "deep is nearer y (via the shortcut)"
    );
}

#[test]
fn section_roots_come_from_the_transitive_reduction() {
    // goal → {x, a}; x → a: a is implied for the goal, so only x is a section.
    let g = graph(&[("goal", "x"), ("goal", "a"), ("x", "a")]);
    let v = view(&g, &id("goal")).unwrap();
    let a = v.nodes.iter().find(|n| n.concept.id == id("a")).unwrap();
    assert_eq!(a.section, Some(id("x")));
}
