//! Live checks against the configured provider. Ignored by default (they
//! cost a model call); run with `cargo test -p axioms-agent -- --ignored`.

use axioms_agent::{ConceptSource, LlmSource, provider};
use axioms_core::{Collision, Concept, ConceptId, Proposal};
use std::sync::Arc;

fn concept(id: &str, name: &str, statement: &str) -> Concept {
    let mut c = Concept::new(ConceptId::from(id), name);
    c.statement = statement.into();
    c
}

fn proposal(name: &str, statement: &str) -> Proposal {
    Proposal {
        name: name.into(),
        statement: statement.into(),
        explanation: String::new(),
        check: String::new(),
        needs: Vec::new(),
    }
}

#[tokio::test]
#[ignore = "calls the model"]
async fn the_judge_tells_same_from_distinct() {
    let source = LlmSource::new(provider::from_env().unwrap());
    let collisions = vec![
        Collision {
            proposal: proposal(
                "Ring",
                "A circular buffer of fixed capacity that overwrites its oldest entry.",
            ),
            existing: vec![concept(
                "ring",
                "Ring",
                "A set with two operations, + and ×, forming an abelian group under + and a monoid under ×, with × distributing over +.",
            )],
        },
        Collision {
            proposal: proposal(
                "Softmax",
                "softmax(z)_i = e^{z_i} / Σ_j e^{z_j}, turning scores into probabilities.",
            ),
            existing: vec![concept(
                "softmax-function",
                "Softmax function",
                "softmax(z)_i = exp(z_i) / sum_j exp(z_j).",
            )],
        },
    ];
    let verdicts = Arc::new(source).same_concept(&collisions).await.unwrap();
    assert_eq!(
        verdicts,
        vec![None, Some(ConceptId::from("softmax-function"))]
    );
}
