//! The agent: asks a model to decompose a concept into prerequisites (with
//! statements, explanations and check questions) and to judge whether two
//! same-named concepts are the same, then merges the answer into the library.

pub mod learner;
mod prompts;
pub mod provider;
pub mod source;

pub use learner::Learner;
pub use provider::{JsonRequest, ModelProvider};
pub use source::{ConceptSource, DecomposeRequest, LlmSource, ScriptedSource};
