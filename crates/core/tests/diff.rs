mod common;

use axioms_core::{ConceptId, Knowledge, Library, diff};
use common::{graph, id, set};

fn lib(edges: &[(&str, &str)]) -> Library {
    Library {
        graph: graph(edges),
        topics: vec![id("goal")],
    }
}

#[test]
fn diff_holds_only_what_after_adds_or_changes() {
    let before = lib(&[("goal", "a")]);
    let mut after = lib(&[("goal", "a"), ("a", "b"), ("goal", "c")]);
    after.graph.get_mut(&id("a")).unwrap().statement = "new".into();
    set(&mut after.graph, "goal", Knowledge::Known);
    let d = diff(&before, &after);
    let ids: Vec<&str> = d.concepts.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["a", "b", "c"]);
    assert_eq!(d.needs.len(), 2);
    assert_eq!(d.knowledge, [(id("goal"), Knowledge::Known)]);
    assert!(d.topics.is_empty());
    assert!(diff(&after, &after).is_empty());
}

#[test]
fn a_known_new_concept_carries_its_knowledge() {
    let before = Library::default();
    let mut after = lib(&[("goal", "a")]);
    set(&mut after.graph, "a", Knowledge::Known);
    let d = diff(&before, &after);
    assert_eq!(d.knowledge, [(id("a"), Knowledge::Known)]);
    assert_eq!(d.topics, [id("goal")]);
}

#[test]
fn steps_grow_outward_from_the_root_without_dangling_edges() {
    let before = lib(&[("goal", "x")]);
    let after = lib(&[
        ("goal", "x"),
        ("goal", "a"),
        ("a", "b"),
        ("x", "b"),
        ("goal", "c"),
    ]);
    let d = diff(&before, &after);
    let steps = d.steps(&id("goal"), &before);
    let added: Vec<Vec<&str>> = steps
        .iter()
        .map(|s| s.concepts.iter().map(|c| c.id.as_str()).collect())
        .collect();
    assert_eq!(added, [vec!["a"], vec!["c"], vec!["b"]]);
    let mut present: Vec<ConceptId> = vec![id("goal"), id("x")];
    let mut needs = 0;
    for s in &steps {
        present.extend(s.concepts.iter().map(|c| c.id.clone()));
        for (f, t) in &s.needs {
            assert!(
                present.contains(f) && present.contains(t),
                "dangling {f}→{t}"
            );
        }
        needs += s.needs.len();
    }
    assert_eq!(needs, d.needs.len());
}

#[test]
fn absorb_takes_concepts_needs_and_topics() {
    let mut mine = lib(&[("goal", "a")]);
    let mut theirs = lib(&[("other", "a"), ("a", "b")]);
    theirs.topics = vec![id("other")];
    set(&mut theirs.graph, "b", Knowledge::Known);
    assert_eq!(mine.absorb(theirs), 0);
    assert_eq!(mine.graph.len(), 4);
    assert_eq!(mine.topics, [id("goal"), id("other")]);
    assert_eq!(
        mine.graph.get(&id("b")).unwrap().knowledge,
        Knowledge::Known
    );
    assert!(diff(&mine, &mine.clone()).is_empty());
}
