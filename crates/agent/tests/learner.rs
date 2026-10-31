use axioms_agent::{Learner, ScriptedSource};
use axioms_core::{ConceptId, Decomposition, Knowledge, Library, Proposal, roadmap, view};
use std::sync::{Arc, Mutex};

fn p(name: &str, needs: &[&str]) -> Proposal {
    Proposal {
        name: name.into(),
        statement: format!("{name}."),
        explanation: String::new(),
        check: String::new(),
        needs: needs.iter().map(|s| s.to_string()).collect(),
    }
}

fn script(same: bool) -> ScriptedSource {
    let mut s = ScriptedSource {
        same,
        ..Default::default()
    };
    s.decompositions.insert(
        "Attention".into(),
        Decomposition {
            target: p("Attention", &["Softmax", "Dot product"]),
            concepts: vec![
                p("Softmax", &["Exponential function"]),
                p("Dot product", &[]),
                p("Exponential function", &[]),
            ],
        },
    );
    s.decompositions.insert(
        "Logistic regression".into(),
        Decomposition {
            target: p(
                "Logistic regression",
                &["exponential function", "Gradient descent"],
            ),
            concepts: vec![p("exponential function", &[]), p("Gradient descent", &[])],
        },
    );
    s
}

#[tokio::test]
async fn learn_builds_a_roadmap_and_axioms_carry_over() {
    let learner = Learner::new(Arc::new(script(true)));
    let lib = Mutex::new(Library::default());
    let goal = learner.learn(&lib, "Attention", 2).await.unwrap();
    {
        let l = lib.lock().unwrap();
        let v = view(&l.graph, &goal).unwrap();
        let steps: Vec<_> = roadmap(&v).steps.into_iter().map(|s| s.id.0).collect();
        assert_eq!(
            steps,
            [
                "dot-product",
                "exponential-function",
                "softmax",
                "attention"
            ]
        );
    }
    lib.lock()
        .unwrap()
        .graph
        .get_mut(&ConceptId::from("exponential-function"))
        .unwrap()
        .knowledge = Knowledge::Known;

    let second = learner.learn(&lib, "Logistic regression", 1).await.unwrap();
    let l = lib.lock().unwrap();
    assert_eq!(
        l.graph.len(),
        6,
        "the exponential function is reused, not duplicated"
    );
    let v = view(&l.graph, &second).unwrap();
    let steps: Vec<_> = roadmap(&v).steps.into_iter().map(|s| s.id.0).collect();
    assert_eq!(steps, ["gradient-descent", "logistic-regression"]);
    assert_eq!(l.topics[0], second);
}

#[tokio::test]
async fn a_distinct_verdict_keeps_two_concepts() {
    let learner = Learner::new(Arc::new(script(false)));
    let lib = Mutex::new(Library::default());
    learner.learn(&lib, "Attention", 2).await.unwrap();
    learner.learn(&lib, "Logistic regression", 1).await.unwrap();
    let l = lib.lock().unwrap();
    assert!(
        l.graph
            .get(&ConceptId::from("exponential-function-2"))
            .is_some()
    );
}
