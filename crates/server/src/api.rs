//! The JSON API. Every call that changes the library saves it and answers
//! with the goal's fresh [`Blueprint`].

use crate::state::{ApiError, AppState, Blueprint};
use axioms_core::{ConceptId, Knowledge};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

type Api<T> = Result<Json<T>, ApiError>;
type St = State<Arc<AppState>>;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/meta", get(meta))
        .route("/api/topics", get(topics).post(learn))
        .route("/api/topics/{id}", get(topic))
        .route("/api/concepts/{id}/knowledge", post(set_knowledge))
        .route("/api/concepts/{id}/decompose", post(decompose))
        .with_state(state)
}

#[derive(Serialize)]
struct Meta {
    provider: String,
    concepts: usize,
    known: usize,
}

async fn meta(State(s): St) -> Json<Meta> {
    let lib = s.lib();
    let known = lib
        .graph
        .concepts()
        .filter(|c| c.knowledge == Knowledge::Known)
        .count();
    Json(Meta {
        provider: s.learner.label(),
        concepts: lib.graph.len(),
        known,
    })
}

#[derive(Serialize)]
struct TopicRef {
    id: ConceptId,
    name: String,
}

async fn topics(State(s): St) -> Json<Vec<TopicRef>> {
    let lib = s.lib();
    let refs = lib
        .topics
        .iter()
        .filter_map(|id| lib.graph.get(id))
        .map(|c| TopicRef {
            id: c.id.clone(),
            name: c.name.clone(),
        });
    Json(refs.collect())
}

async fn topic(State(s): St, Path(id): Path<String>) -> Api<Blueprint> {
    Ok(Json(s.blueprint(&ConceptId(id))?))
}

#[derive(Deserialize)]
struct LearnBody {
    topic: String,
    #[serde(default = "two")]
    depth: u8,
}

fn two() -> u8 {
    2
}

async fn learn(State(s): St, Json(body): Json<LearnBody>) -> Api<Blueprint> {
    let topic = body.topic.trim();
    if topic.is_empty() || topic.len() > 200 {
        return Err(ApiError::bad_request(
            "state a topic (at most 200 characters)",
        ));
    }
    let _permit = s.model_calls.acquire().await.expect("semaphore open");
    let goal = s
        .learner
        .learn(&s.lib, topic, body.depth.clamp(1, 2))
        .await?;
    s.save()?;
    Ok(Json(s.blueprint(&goal)?))
}

#[derive(Deserialize)]
struct KnowledgeBody {
    knowledge: Knowledge,
    goal: ConceptId,
}

async fn set_knowledge(
    State(s): St,
    Path(id): Path<String>,
    Json(b): Json<KnowledgeBody>,
) -> Api<Blueprint> {
    let id = ConceptId(id);
    {
        let mut lib = s.lib();
        if lib.graph.get(&b.goal).is_none() {
            return Err(ApiError::not_found(&b.goal));
        }
        let c = lib
            .graph
            .get_mut(&id)
            .ok_or_else(|| ApiError::not_found(&id))?;
        c.knowledge = b.knowledge;
    }
    s.save()?;
    Ok(Json(s.blueprint(&b.goal)?))
}

#[derive(Deserialize)]
struct DecomposeBody {
    goal: ConceptId,
}

async fn decompose(
    State(s): St,
    Path(id): Path<String>,
    Json(b): Json<DecomposeBody>,
) -> Api<Blueprint> {
    let id = ConceptId(id);
    for c in [&id, &b.goal] {
        if s.lib().graph.get(c).is_none() {
            return Err(ApiError::not_found(c));
        }
    }
    let _permit = s.model_calls.acquire().await.expect("semaphore open");
    s.learner.decompose(&s.lib, &id, Some(&b.goal), 1).await?;
    s.save()?;
    Ok(Json(s.blueprint(&b.goal)?))
}
