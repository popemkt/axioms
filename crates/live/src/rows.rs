//! Rows ⇄ library: the subscribed cache read as a [`Library`] for one
//! person, and a [`Diff`] written back through reducers.

use crate::bindings::*;
use crate::conn::Live;
use anyhow::Context;
use axioms_core::{Concept, ConceptId, Diff, Knowledge, Library};
use spacetimedb_sdk::{Identity, Table};
use std::time::Duration;
use tokio::sync::oneshot;

/// The shared graph with `person`'s knowledge (everything unknown without
/// one). A need that would close a cycle (another client's bad write) is
/// skipped, not fatal.
pub fn library(live: &Live, person: Option<Identity>) -> Library {
    let db = &live.conn.db;
    let mut lib = Library::default();
    for c in db.concept().iter() {
        lib.graph.upsert(Concept {
            id: ConceptId(c.id),
            name: c.name,
            statement: c.statement,
            explanation: c.explanation,
            check: c.check,
            knowledge: Knowledge::Unknown,
        });
    }
    if let Some(p) = person {
        for k in db.knowledge().iter().filter(|k| k.person == p) {
            if let Some(c) = lib.graph.get_mut(&ConceptId(k.concept)) {
                c.knowledge = knowledge_of(&k.state);
            }
        }
    }
    let mut needs: Vec<Need> = db.need().iter().collect();
    needs.sort_by(|a, b| a.key.cmp(&b.key));
    for n in needs {
        let (from, to) = (ConceptId(n.from_id), ConceptId(n.to_id));
        if let Err(e) = lib.graph.add_need(&from, &to) {
            tracing::warn!("skipping need {from} → {to}: {e}");
        }
    }
    let mut topics: Vec<Topic> = db.topic().iter().collect();
    topics.sort_by_key(|t| std::cmp::Reverse(t.seq));
    lib.topics = topics.into_iter().map(|t| ConceptId(t.concept)).collect();
    lib
}

fn knowledge_of(s: &str) -> Knowledge {
    match s {
        "known" => Knowledge::Known,
        "learning" => Knowledge::Learning,
        _ => Knowledge::Unknown,
    }
}

pub fn knowledge_name(k: Knowledge) -> &'static str {
    match k {
        Knowledge::Unknown => "unknown",
        Knowledge::Learning => "learning",
        Knowledge::Known => "known",
    }
}

type Done = oneshot::Receiver<Result<(), String>>;
type Outcome = Result<Result<(), String>, spacetimedb_sdk::__codegen::InternalError>;

/// A reducer's outcome callback, as a receiver.
fn outcome() -> (
    impl FnOnce(&ReducerEventContext, Outcome) + Send + 'static,
    Done,
) {
    let (tx, rx) = oneshot::channel();
    let cb = move |_: &ReducerEventContext, r: Outcome| {
        let r = match r {
            Ok(inner) => inner,
            Err(e) => Err(format!("{e:?}")),
        };
        let _ = tx.send(r);
    };
    (cb, rx)
}

/// Write a diff's graph part (concepts, needs, topics) in one `put`, and
/// its knowledge as the caller's own marks. Answers the receivers to await.
pub fn send(live: &Live, d: &Diff) -> anyhow::Result<Vec<Done>> {
    let mut waits = Vec::new();
    if !(d.concepts.is_empty() && d.needs.is_empty() && d.topics.is_empty()) {
        let concepts = d
            .concepts
            .iter()
            .map(|c| ConceptIn {
                id: c.id.0.clone(),
                name: c.name.clone(),
                statement: c.statement.clone(),
                explanation: c.explanation.clone(),
                check: c.check.clone(),
            })
            .collect();
        let needs = d
            .needs
            .iter()
            .map(|(f, t)| NeedIn {
                from_id: f.0.clone(),
                to_id: t.0.clone(),
            })
            .collect();
        let topics = d.topics.iter().map(|t| t.0.clone()).collect();
        let (cb, rx) = outcome();
        live.conn.reducers.put_then(concepts, needs, topics, cb)?;
        waits.push(rx);
    }
    for (id, k) in &d.knowledge {
        let (cb, rx) = outcome();
        let state = knowledge_name(*k).to_owned();
        live.conn.reducers.mark_then(id.0.clone(), state, cb)?;
        waits.push(rx);
    }
    Ok(waits)
}

/// Await every receiver, failing on the first refusal.
pub async fn confirmed(waits: Vec<Done>) -> anyhow::Result<()> {
    for w in waits {
        tokio::time::timeout(Duration::from_secs(30), w)
            .await
            .context("spacetimedb did not confirm a write in 30 s")?
            .context("spacetimedb connection dropped a write")?
            .map_err(|e| anyhow::anyhow!("spacetimedb refused a write: {e}"))?;
    }
    Ok(())
}

pub fn claim(live: &Live, id: u64) -> anyhow::Result<Done> {
    let (cb, rx) = outcome();
    live.conn.reducers.claim_then(id, cb)?;
    Ok(rx)
}

pub fn resolve(live: &Live, id: u64, concept: &ConceptId) -> anyhow::Result<Done> {
    let (cb, rx) = outcome();
    live.conn.reducers.resolve_then(id, concept.0.clone(), cb)?;
    Ok(rx)
}

pub fn finish(live: &Live, id: u64, ok: bool, note: String) -> anyhow::Result<Done> {
    let (cb, rx) = outcome();
    live.conn.reducers.finish_then(id, ok, note, cb)?;
    Ok(rx)
}
