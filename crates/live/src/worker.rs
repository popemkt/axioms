//! The decompose worker: watches the request table, claims a request,
//! runs the learner (the model call SpacetimeDB cannot make) on the
//! requester's library, and writes the answer back step by step, so every
//! open view grows node by node.

use crate::bindings::RequestTableAccess;
use crate::conn::Live;
use crate::mirror::Mirror;
use crate::rows::{self, confirmed, library, send};
use axioms_agent::Learner;
use axioms_core::{ConceptId, diff};
use spacetimedb_sdk::Table;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;

/// At most this many requests at once: the host is kept gentle.
const PARALLEL: usize = 2;

/// The pause between written steps: `AXIOMS_STREAM_MS` (default 140).
pub fn pace() -> Duration {
    let ms = std::env::var("AXIOMS_STREAM_MS")
        .ok()
        .and_then(|v| v.parse().ok());
    Duration::from_millis(ms.unwrap_or(140))
}

/// Run forever. `pace` is the pause between written steps (presentation:
/// a view sees the graph grow rather than appear).
pub async fn run(mirror: Arc<Mirror>, learner: Arc<Learner>, pace: Duration) {
    let slots = Arc::new(Semaphore::new(PARALLEL));
    let busy: Arc<Mutex<HashSet<u64>>> = Arc::default();
    loop {
        tokio::time::sleep(Duration::from_millis(400)).await;
        let Some(live) = mirror.get() else { continue };
        let mut todo: Vec<_> = live
            .conn
            .db
            .request()
            .iter()
            .filter(|r| {
                r.status == "pending" || (r.status == "working" && r.worker == Some(live.me))
            })
            .collect();
        todo.sort_by_key(|r| r.id);
        for r in todo {
            if !busy.lock().unwrap_or_else(|p| p.into_inner()).insert(r.id) {
                continue;
            }
            let Ok(slot) = slots.clone().acquire_owned().await else {
                return;
            };
            let (live, learner, busy) = (live.clone(), learner.clone(), busy.clone());
            tokio::spawn(async move {
                if let Err(e) = handle(&live, &learner, &r, pace).await {
                    tracing::warn!(request = r.id, "{e:#}");
                }
                busy.lock().unwrap_or_else(|p| p.into_inner()).remove(&r.id);
                drop(slot);
            });
        }
    }
}

async fn handle(
    live: &Live,
    learner: &Learner,
    r: &crate::bindings::Request,
    pace: Duration,
) -> anyhow::Result<()> {
    if confirmed(vec![rows::claim(live, r.id)?]).await.is_err() {
        return Ok(()); // another worker took it
    }
    tracing::info!(request = r.id, kind = %r.kind, text = %r.text, "working");
    let outcome = work(live, learner, r, pace).await;
    let (ok, note) = match &outcome {
        Ok(n) => (true, n.clone()),
        Err(e) => (false, format!("{e:#}")),
    };
    confirmed(vec![rows::finish(live, r.id, ok, note)?]).await?;
    outcome.map(drop)
}

async fn work(
    live: &Live,
    learner: &Learner,
    r: &crate::bindings::Request,
    pace: Duration,
) -> anyhow::Result<String> {
    let lib = Mutex::new(library(live, Some(r.by)));
    let lock = || lib.lock().unwrap_or_else(|p| p.into_inner());
    let (target, goal, depth) = if r.kind == "topic" {
        let before = lock().clone();
        let (id, fresh) = {
            let mut l = lock();
            let id = l.state_topic(&r.text);
            let fresh = l.graph.needs(&id).is_empty();
            (id, fresh)
        };
        // The topic exists at once, so the asker's view can open it and
        // watch it grow.
        let opened = diff(&before, &lock());
        confirmed(send(live, &opened)?).await?;
        confirmed(vec![rows::resolve(live, r.id, &id)?]).await?;
        if !fresh {
            return Ok("already decomposed".into());
        }
        (id.clone(), id, r.depth)
    } else {
        let goal = if r.goal.is_empty() { &r.text } else { &r.goal };
        (ConceptId(r.text.clone()), ConceptId(goal.clone()), 1)
    };
    let before = lock().clone();
    learner.decompose(&lib, &target, Some(&goal), depth).await?;
    let d = diff(&before, &lock());
    for step in d.steps(&target, &before) {
        confirmed(send(live, &step)?).await?;
        tokio::time::sleep(pace).await;
    }
    Ok(format!(
        "{} concepts, {} needs",
        d.concepts.len(),
        d.needs.len()
    ))
}
