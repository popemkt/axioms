use axioms_core::{Knowledge, Library};
use axioms_store::{JsonlStore, MemoryStore, Store, schema};

fn sample() -> Library {
    let mut lib = Library::default();
    let goal = lib.state_topic("Attention");
    let soft = lib.state_topic("Softmax");
    lib.topics.retain(|t| t != &soft);
    {
        let c = lib.graph.get_mut(&soft).unwrap();
        c.statement = "softmax(x)_i = e^{x_i} / Σ e^{x_j}".into();
        c.knowledge = Knowledge::Known;
    }
    lib.graph.add_need(&goal, &soft).unwrap();
    lib
}

fn assert_same(a: &Library, b: &Library) {
    assert_eq!(a.topics, b.topics);
    assert_eq!(
        a.graph.concepts().collect::<Vec<_>>(),
        b.graph.concepts().collect::<Vec<_>>()
    );
    assert_eq!(a.graph.edges(), b.graph.edges());
}

#[test]
fn every_store_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let stores: Vec<Box<dyn Store>> = vec![
        Box::new(MemoryStore::new()),
        Box::new(JsonlStore::new(dir.path().join("nodes.jsonl"))),
    ];
    for store in stores {
        assert!(store.load().unwrap().graph.is_empty());
        let lib = sample();
        store.save(&lib).unwrap();
        assert_same(&lib, &store.load().unwrap());
    }
}

#[test]
fn concepts_are_kb_nodes_with_ref_props() {
    let store = MemoryStore::new();
    store.save(&sample()).unwrap();
    let nodes = store.nodes();
    let attention = nodes.iter().find(|n| n.id == "attention").unwrap();
    assert_eq!(
        attention.refs(schema::NEEDS).collect::<Vec<_>>(),
        ["softmax"]
    );
    assert_eq!(
        attention.refs(schema::TYPE).collect::<Vec<_>>(),
        [schema::CONCEPT]
    );
    let line = serde_json::to_string(attention).unwrap();
    assert!(
        line.contains(r#""ax.f.needs":[{"t":"ref","v":"softmax"}]"#),
        "{line}"
    );
}

#[test]
fn jsonl_keeps_timestamps_of_unchanged_nodes() {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonlStore::new(dir.path().join("nodes.jsonl"));
    let mut lib = sample();
    store.save(&lib).unwrap();
    let before = std::fs::read_to_string(store.path()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    store.save(&lib).unwrap();
    assert_eq!(before, std::fs::read_to_string(store.path()).unwrap());
    lib.graph.get_mut(&"attention".into()).unwrap().knowledge = Knowledge::Learning;
    store.save(&lib).unwrap();
    let after = std::fs::read_to_string(store.path()).unwrap();
    let changed = before
        .lines()
        .zip(after.lines())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(changed, 1);
}

#[test]
fn jsonl_keeps_nodes_it_does_not_own() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nodes.jsonl");
    std::fs::write(
        &path,
        r#"{"id":"01JNOTE","text":"a kb note","props":{},"children":[]}"#,
    )
    .unwrap();
    let store = JsonlStore::new(&path);
    store.save(&sample()).unwrap();
    store.save(&store.load().unwrap()).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text.matches("01JNOTE").count(), 1);
    assert_eq!(store.load().unwrap().graph.len(), 2);
}

#[test]
fn a_need_on_a_non_concept_is_skipped() {
    let mut nodes = axioms_store::to_nodes(&sample());
    let goal = nodes.iter_mut().find(|n| n.id == "attention").unwrap();
    goal.props
        .entry(schema::NEEDS.into())
        .or_default()
        .push(axioms_store::node::PropValue::Ref("someones-note".into()));
    let lib = axioms_store::from_nodes(&nodes).unwrap();
    assert_same(&sample(), &lib);
}
