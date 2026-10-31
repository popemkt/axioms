//! Where decompositions come from. One contract ([`ConceptSource`]); the
//! model-backed [`LlmSource`] and the scripted [`ScriptedSource`] (tests,
//! offline demos) both implement it.

use crate::prompts;
use crate::provider::{JsonRequest, ModelProvider};
use anyhow::Context;
use async_trait::async_trait;
use axioms_core::{Collision, ConceptId, Decomposition};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

/// Everything a source is told about the concept to decompose.
#[derive(Debug, Clone, Default)]
pub struct DecomposeRequest {
    pub target: String,
    pub statement: String,
    /// The topic this decomposition ultimately serves.
    pub goal: Option<String>,
    /// Names of the concepts that need the target: its context.
    pub needed_for: Vec<String>,
    /// Names of concepts the learner knows: their axioms.
    pub known: Vec<String>,
    /// Names of other concepts already in the graph, for reuse.
    pub existing: Vec<String>,
    /// 1: direct prerequisites only; 2: and theirs.
    pub depth: u8,
}

#[async_trait]
pub trait ConceptSource: Send + Sync {
    fn label(&self) -> String;
    async fn decompose(&self, request: &DecomposeRequest) -> anyhow::Result<Decomposition>;
    /// For each collision, the existing concept it is the same as, if any.
    async fn same_concept(
        &self,
        collisions: &[Collision],
    ) -> anyhow::Result<Vec<Option<ConceptId>>>;
}

pub struct LlmSource {
    provider: Arc<dyn ModelProvider>,
}

impl LlmSource {
    pub fn new(provider: Arc<dyn ModelProvider>) -> Self {
        LlmSource { provider }
    }
}

#[derive(Deserialize)]
struct Answers {
    answers: Vec<Answer>,
}

#[derive(Deserialize)]
struct Answer {
    index: usize,
    same_as: Option<String>,
}

#[async_trait]
impl ConceptSource for LlmSource {
    fn label(&self) -> String {
        self.provider.label()
    }

    async fn decompose(&self, request: &DecomposeRequest) -> anyhow::Result<Decomposition> {
        let value = self
            .provider
            .complete_json(&JsonRequest {
                system: prompts::DECOMPOSE_SYSTEM.into(),
                prompt: prompts::decompose_prompt(request),
                schema: prompts::decomposition_schema(),
            })
            .await?;
        serde_json::from_value(value).context("decomposition does not match the contract")
    }

    async fn same_concept(
        &self,
        collisions: &[Collision],
    ) -> anyhow::Result<Vec<Option<ConceptId>>> {
        let value = self
            .provider
            .complete_json(&JsonRequest {
                system: prompts::SAME_SYSTEM.into(),
                prompt: prompts::same_prompt(collisions),
                schema: prompts::same_schema(),
            })
            .await?;
        let answers: Answers = serde_json::from_value(value).context("dedupe answer")?;
        let mut out = vec![None; collisions.len()];
        for a in answers.answers {
            // Only accept an id that was actually offered for that index.
            let offered = collisions.get(a.index).is_some_and(|c| {
                c.existing
                    .iter()
                    .any(|e| Some(e.id.as_str()) == a.same_as.as_deref())
            });
            if offered {
                out[a.index] = a.same_as.map(ConceptId);
            }
        }
        Ok(out)
    }
}

/// Answers from a script: decompositions by target name, and a fixed
/// dedupe verdict (same as the first candidate, or always distinct).
#[derive(Default)]
pub struct ScriptedSource {
    pub decompositions: HashMap<String, Decomposition>,
    pub same: bool,
}

#[async_trait]
impl ConceptSource for ScriptedSource {
    fn label(&self) -> String {
        "scripted".into()
    }

    async fn decompose(&self, request: &DecomposeRequest) -> anyhow::Result<Decomposition> {
        self.decompositions
            .get(&request.target)
            .cloned()
            .with_context(|| format!("no script for `{}`", request.target))
    }

    async fn same_concept(
        &self,
        collisions: &[Collision],
    ) -> anyhow::Result<Vec<Option<ConceptId>>> {
        Ok(collisions
            .iter()
            .map(|c| self.same.then(|| c.existing[0].id.clone()))
            .collect())
    }
}
