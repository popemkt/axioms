use axioms_agent::{Learner, ScriptedSource};
use axioms_core::{Decomposition, Library, Proposal};
use axioms_server::{api, state::AppState};
use axioms_store::{MemoryStore, Store};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

fn p(name: &str, needs: &[&str]) -> Proposal {
    Proposal {
        name: name.into(),
        statement: format!("{name}."),
        explanation: "x".into(),
        check: "y?".into(),
        needs: needs.iter().map(|s| s.to_string()).collect(),
    }
}

fn app() -> (axum::Router, Arc<MemoryStore>) {
    let mut source = ScriptedSource::default();
    source.decompositions.insert(
        "Group theory".into(),
        Decomposition {
            target: p("Group theory", &["Sets", "Functions"]),
            concepts: vec![p("Sets", &[]), p("Functions", &[])],
        },
    );
    source.decompositions.insert(
        "Functions".into(),
        Decomposition {
            target: p("Functions", &["Ordered pairs"]),
            concepts: vec![p("Ordered pairs", &[])],
        },
    );
    let store = Arc::new(MemoryStore::new());
    let state = AppState::new(
        Library::default(),
        store.clone(),
        Learner::new(Arc::new(source)),
    );
    (api::router(Arc::new(state)), store)
}

async fn call(app: &axum::Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn steps(bp: &Value) -> Vec<&str> {
    bp["roadmap"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn learn_mark_decompose_round_trip() {
    let (app, store) = app();
    let (status, bp) = call(
        &app,
        "POST",
        "/api/topics",
        json!({"topic": "Group theory"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{bp}");
    assert_eq!(steps(&bp), ["functions", "sets", "group-theory"]);

    let (_, bp) = call(
        &app,
        "POST",
        "/api/concepts/sets/knowledge",
        json!({"knowledge": "known", "goal": "group-theory"}),
    )
    .await;
    assert_eq!(steps(&bp), ["functions", "group-theory"]);
    assert_eq!(
        store
            .load()
            .unwrap()
            .graph
            .get(&"sets".into())
            .unwrap()
            .knowledge,
        axioms_core::Knowledge::Known
    );

    let (_, bp) = call(
        &app,
        "POST",
        "/api/concepts/functions/decompose",
        json!({"goal": "group-theory"}),
    )
    .await;
    assert_eq!(steps(&bp), ["ordered-pairs", "functions", "group-theory"]);

    let (_, topics) = call(&app, "GET", "/api/topics", Value::Null).await;
    assert_eq!(topics[0]["id"], "group-theory");
    let (status, _) = call(&app, "GET", "/api/topics/nope", Value::Null).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
