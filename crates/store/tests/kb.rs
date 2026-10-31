//! KbStore against a real kb, in a throwaway store with a throwaway home
//! (`KB_HOME`), so the person's own kb is never opened. Needs the `kb`
//! binary: `cargo test -p axioms-store -- --ignored`.

use axioms_core::{ConceptId, Knowledge, Library};
use axioms_store::{KbStore, Store};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

fn kb(dir: &Path, args: &[&str]) -> Value {
    let out = Command::new("kb")
        .env("KB_HOME", dir.join("home"))
        .arg("--json")
        .arg("--root")
        .arg(dir.join("store"))
        .args(args)
        .output()
        .expect("kb on PATH");
    assert!(
        out.status.success(),
        "kb {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or(Value::Null)
}

fn scratch() -> (tempfile::TempDir, KbStore) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("store")).unwrap();
    kb(dir.path(), &["init", "--bare"]);
    let store = KbStore::new("kb", dir.path().join("store")).with_home(dir.path().join("home"));
    (dir, store)
}

fn sample() -> (Library, ConceptId, ConceptId, ConceptId) {
    let mut lib = Library::default();
    let goal = lib.state_topic("Attention");
    let soft = lib.state_topic("Softmax");
    let dot = lib.state_topic("Dot product");
    lib.topics.retain(|t| t == &goal);
    lib.graph.get_mut(&soft).unwrap().statement = "softmax(x)_i = e^{x_i} / Σ e^{x_j}".into();
    lib.graph.add_need(&goal, &soft).unwrap();
    lib.graph.add_need(&goal, &dot).unwrap();
    (lib, goal, soft, dot)
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
#[ignore = "needs the kb binary"]
fn round_trips_and_follows_edits_through_kb() {
    let (dir, store) = scratch();
    assert!(store.load().unwrap().graph.is_empty());
    let (lib, _, soft, dot) = sample();
    store.save(&lib).unwrap();
    assert_same(&lib, &store.load().unwrap());

    // The person adds their own note field to a concept in kb.
    kb(
        dir.path(),
        &["set", soft.0.as_str(), "my note", "revisit", "--create"],
    );

    // The same library without Dot product (its edge goes with it), and
    // Softmax now known.
    let mut lib = Library::default();
    let goal = lib.state_topic("Attention");
    let soft = lib.state_topic("Softmax");
    lib.topics.retain(|t| t == &goal);
    lib.graph.get_mut(&soft).unwrap().knowledge = Knowledge::Known;
    lib.graph.add_need(&goal, &soft).unwrap();
    store.save(&lib).unwrap();
    assert_same(&lib, &store.load().unwrap());

    let found = kb(dir.path(), &["search", "Softmax"]);
    assert!(
        found.to_string().contains(&soft.0),
        "kb search finds it: {found}"
    );
    let node = kb(dir.path(), &["get", soft.0.as_str(), "--depth", "0"]);
    assert!(
        node.to_string().contains("revisit"),
        "the person's field stays: {node}"
    );
    let gone = kb(dir.path(), &["search", "Dot product"]);
    assert!(
        !gone.to_string().contains(&dot.0),
        "deleted from kb: {gone}"
    );
}

#[test]
#[ignore = "needs the kb binary"]
fn kb_refuses_a_need_on_no_node() {
    let (dir, store) = scratch();
    let (lib, goal, ..) = sample();
    store.save(&lib).unwrap();
    // needs is a ref field, so kb itself refuses a dangling edge.
    let out = Command::new("kb")
        .env("KB_HOME", dir.path().join("home"))
        .args(["--json", "--root"])
        .arg(dir.path().join("store"))
        .args(["set", goal.0.as_str(), "ax.f.needs", "no-such-concept"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "kb took a needs ref to no node");
}
