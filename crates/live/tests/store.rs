//! SpacetimeStore against a running SpacetimeDB, in a throwaway database:
//! `scripts/spacetime publish -s axioms axioms-test --module-path spacetime -c -y`
//! then `cargo test -p axioms-live -- --ignored`.

use axioms_core::{Knowledge, Library};
use axioms_live::{Config, SpacetimeStore};
use axioms_store::Store;

fn store(who: &str) -> SpacetimeStore {
    let mut cfg = Config::from_env("test");
    cfg.db = std::env::var("AXIOMS_STDB_TEST_DB").unwrap_or_else(|_| "axioms-test".into());
    cfg.creds = format!("axioms-test-{who}");
    SpacetimeStore::new(cfg)
}

#[test]
#[ignore = "needs a running SpacetimeDB with the axioms-test database"]
fn the_graph_is_shared_and_knowledge_is_each_persons() {
    let (ada, bob) = (store("ada"), store("bob"));
    let mut lib = ada.load().unwrap();
    let goal = lib.state_topic("Store test topic");
    let part = lib.state_topic("Store test part");
    lib.topics.retain(|t| t != &part);
    lib.graph.add_need(&goal, &part).unwrap();
    lib.graph.get_mut(&part).unwrap().statement = "a part".into();
    lib.graph.get_mut(&part).unwrap().knowledge = Knowledge::Known;
    ada.save(&lib).unwrap();

    let ada_sees = ada.load().unwrap();
    assert!(ada_sees.graph.needs(&goal).contains(&&part));
    assert_eq!(ada_sees.graph.get(&part).unwrap().statement, "a part");
    assert_eq!(
        ada_sees.graph.get(&part).unwrap().knowledge,
        Knowledge::Known
    );
    assert!(ada_sees.topics.contains(&goal));

    let bob_sees: Library = bob.load().unwrap();
    assert!(
        bob_sees.graph.needs(&goal).contains(&&part),
        "the graph is shared"
    );
    assert_eq!(
        bob_sees.graph.get(&part).unwrap().knowledge,
        Knowledge::Unknown,
        "what ada knows is hers"
    );
}
