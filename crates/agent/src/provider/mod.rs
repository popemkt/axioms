//! The model provider contract: a system prompt, a user prompt and a JSON
//! schema in; one JSON value conforming to that schema out.

mod claude_cli;
mod rig_openai;

pub use claude_cli::ClaudeCli;
pub use rig_openai::RigOpenAi;

use anyhow::{Context, bail};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct JsonRequest {
    pub system: String,
    pub prompt: String,
    pub schema: serde_json::Value,
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// A short label for logs and the UI, e.g. `claude-cli:sonnet`.
    fn label(&self) -> String;
    async fn complete_json(&self, request: &JsonRequest) -> anyhow::Result<serde_json::Value>;
}

/// The provider named by the environment. Keys are read from the
/// environment at call time and never stored.
///
/// - `AXIOMS_PROVIDER=claude-cli` (default): `claude -p` as a subprocess;
///   `AXIOMS_CLAUDE_BIN` (default `claude`), `AXIOMS_MODEL` (default `sonnet`).
/// - `AXIOMS_PROVIDER=openai`: any OpenAI-compatible endpoint through Rig
///   (e.g. CLIProxyAPI); `AXIOMS_OPENAI_BASE_URL`, `AXIOMS_MODEL`, and the key
///   in the variable named by `AXIOMS_OPENAI_KEY_ENV` (default `OPENAI_API_KEY`).
pub fn from_env() -> anyhow::Result<Arc<dyn ModelProvider>> {
    let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let model = var("AXIOMS_MODEL");
    match var("AXIOMS_PROVIDER").as_deref().unwrap_or("claude-cli") {
        "claude-cli" => Ok(Arc::new(ClaudeCli::new(
            var("AXIOMS_CLAUDE_BIN").unwrap_or_else(|| "claude".into()),
            model.unwrap_or_else(|| "sonnet".into()),
        ))),
        "openai" => {
            let key_env = var("AXIOMS_OPENAI_KEY_ENV").unwrap_or_else(|| "OPENAI_API_KEY".into());
            Ok(Arc::new(RigOpenAi::new(
                var("AXIOMS_OPENAI_BASE_URL"),
                key_env,
                model.context("AXIOMS_MODEL is required for the openai provider")?,
            )))
        }
        other => bail!("unknown AXIOMS_PROVIDER `{other}` (claude-cli | openai)"),
    }
}

/// Parse a model's text answer as JSON, tolerating a fenced code block.
pub(crate) fn parse_json_text(text: &str) -> anyhow::Result<serde_json::Value> {
    let t = text.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .unwrap_or(t);
    serde_json::from_str(t.trim()).with_context(|| format!("model answer is not JSON: {text:.200}"))
}
