mod common;
use axioms_core::{ConceptGraph, GraphError, normalize};
use common::*;

#[test]
fn normalize_folds_case_punctuation_and_articles() {
    assert_eq!(
        normalize("The Fundamental Theorem of Algebra"),
        "fundamental-theorem-of-algebra"
    );
    assert_eq!(
        normalize("  Gödel's  incompleteness! "),
        "gödels-incompleteness"
    );
    assert_eq!(normalize("Softmax (function)"), "softmax-function");
    assert_eq!(normalize("A"), "a");
}

#[test]
fn needs_are_sorted_and_idempotent() {
    let mut g = graph(&[("goal", "b"), ("goal", "a")]);
    g.add_need(&id("goal"), &id("a")).unwrap();
    assert_eq!(g.needs(&id("goal")), vec![&id("a"), &id("b")]);
    assert_eq!(g.needed_by(&id("a")), vec![&id("goal")]);
    assert_eq!(g.edges().len(), 2);
}

#[test]
fn cycles_and_self_loops_are_refused() {
    let mut g = graph(&[("a", "b"), ("b", "c")]);
    assert_eq!(
        g.add_need(&id("c"), &id("a")),
        Err(GraphError::Cycle(id("c"), id("a")))
    );
    assert!(g.add_need(&id("a"), &id("a")).is_err());
    assert!(matches!(
        g.add_need(&id("a"), &id("zzz")),
        Err(GraphError::Unknown(_))
    ));
}

#[test]
fn fresh_ids_suffix_on_collision() {
    let mut g = ConceptGraph::new();
    assert_eq!(g.fresh_id("Group"), id("group"));
    g.upsert(concept("Group"));
    assert_eq!(g.fresh_id("group"), id("group-2"));
    assert_eq!(g.named("GROUP").len(), 1);
}

#[test]
fn stating_a_topic_reuses_by_name_and_moves_it_first() {
    let mut lib = axioms_core::Library::default();
    let a = lib.state_topic("Fermat's Last Theorem");
    let b = lib.state_topic("Attention");
    assert_eq!(lib.state_topic("fermats last theorem"), a);
    assert_eq!(lib.topics, vec![a, b]);
    assert_eq!(lib.graph.len(), 2);
}

#[test]
fn a_failed_topic_can_be_forgotten_but_linked_concepts_stay() {
    let mut lib = axioms_core::Library::default();
    let t = lib.state_topic("Ghost");
    lib.forget_topic(&t);
    assert!(lib.graph.is_empty() && lib.topics.is_empty());
    let mut g = graph(&[("a", "b")]);
    assert!(!g.remove_isolated(&id("b")));
    g.upsert(concept("c"));
    assert!(g.remove_isolated(&id("c")));
    g.upsert(concept("d"));
    g.add_need(&id("d"), &id("a")).unwrap();
    assert_eq!(g.needs(&id("d")), vec![&id("a")]);
}
