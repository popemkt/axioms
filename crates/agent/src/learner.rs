//! The learning loop: state a topic, decompose concepts, merge the answers.
//!
//! The library sits behind a mutex that is never held across a model call,
//! so a long decomposition does not block reads (or other decompositions).

use crate::source::{ConceptSource, DecomposeRequest};
use anyhow::Context;
use axioms_core::{ConceptId, Knowledge, Library, Merged, Resolution, normalize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

/// How many names of known and existing concepts a request carries.
const CONTEXT_NAMES: usize = 150;

pub struct Learner {
    source: Arc<dyn ConceptSource>,
}

fn lock(lib: &Mutex<Library>) -> MutexGuard<'_, Library> {
    lib.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Learner {
    pub fn new(source: Arc<dyn ConceptSource>) -> Self {
        Learner { source }
    }

    pub fn label(&self) -> String {
        self.source.label()
    }

    /// State `topic`; if it has never been decomposed, decompose it `depth`
    /// levels deep. Returns the topic's concept id.
    pub async fn learn(
        &self,
        lib: &Mutex<Library>,
        topic: &str,
        depth: u8,
    ) -> anyhow::Result<ConceptId> {
        let (id, fresh) = {
            let mut l = lock(lib);
            let id = l.state_topic(topic);
            let fresh = l.graph.needs(&id).is_empty();
            (id, fresh)
        };
        if fresh {
            self.decompose(lib, &id, Some(&id), depth).await?;
        }
        Ok(id)
    }

    /// Decompose one concept `depth` levels, in the context of `goal`.
    pub async fn decompose(
        &self,
        lib: &Mutex<Library>,
        id: &ConceptId,
        goal: Option<&ConceptId>,
        depth: u8,
    ) -> anyhow::Result<Merged> {
        let request = request_for(&lock(lib), id, goal, depth)?;
        let decomposition = self.source.decompose(&request).await?;
        let collisions = {
            let l = lock(lib);
            let target = l.graph.get(id).context("concept vanished")?;
            decomposition.collisions(&l.graph, target)
        };
        let verdicts = if collisions.is_empty() {
            Vec::new()
        } else {
            self.source.same_concept(&collisions).await?
        };
        let resolutions: BTreeMap<String, Resolution> = collisions
            .iter()
            .zip(verdicts)
            .map(|(c, v)| {
                let r = v.map_or(Resolution::Distinct, Resolution::Same);
                (normalize(&c.proposal.name), r)
            })
            .collect();
        let merged = decomposition.apply(&mut lock(lib).graph, id, &resolutions);
        tracing::info!(target = %id, ?merged, "decomposed");
        Ok(merged)
    }
}

fn request_for(
    lib: &Library,
    id: &ConceptId,
    goal: Option<&ConceptId>,
    depth: u8,
) -> anyhow::Result<DecomposeRequest> {
    let target = lib
        .graph
        .get(id)
        .with_context(|| format!("unknown concept `{id}`"))?;
    let name = |id: &ConceptId| lib.graph.get(id).map(|c| c.name.clone());
    let (known, existing): (Vec<_>, Vec<_>) = lib
        .graph
        .concepts()
        .filter(|c| &c.id != id)
        .partition(|c| c.knowledge == Knowledge::Known);
    let names = |cs: Vec<&axioms_core::Concept>| -> Vec<String> {
        cs.into_iter()
            .take(CONTEXT_NAMES)
            .map(|c| c.name.clone())
            .collect()
    };
    Ok(DecomposeRequest {
        target: target.name.clone(),
        statement: target.statement.clone(),
        goal: goal.and_then(name),
        needed_for: lib
            .graph
            .needed_by(id)
            .into_iter()
            .filter_map(name)
            .collect(),
        known: names(known),
        existing: names(existing),
        depth,
    })
}
