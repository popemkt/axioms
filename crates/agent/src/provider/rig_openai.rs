//! Any OpenAI-compatible chat endpoint (OpenAI, CLIProxyAPI, …) through Rig.

use super::{JsonRequest, ModelProvider, parse_json_text};
use anyhow::Context;
use async_trait::async_trait;
use rig_core::completion::CompletionRequest;
use rig_core::providers::openai::OpenAIConfig;

pub struct RigOpenAi {
    base_url: Option<String>,
    key_env: String,
    model: String,
}

impl RigOpenAi {
    pub fn new(base_url: Option<String>, key_env: String, model: String) -> Self {
        RigOpenAi {
            base_url,
            key_env,
            model,
        }
    }
}

#[async_trait]
impl ModelProvider for RigOpenAi {
    fn label(&self) -> String {
        format!("rig-openai:{}", self.model)
    }

    async fn complete_json(&self, request: &JsonRequest) -> anyhow::Result<serde_json::Value> {
        let key =
            std::env::var(&self.key_env).with_context(|| format!("{} is not set", self.key_env))?;
        let mut config = OpenAIConfig::new(key);
        if let Some(url) = &self.base_url {
            config = config.with_base_url(url.clone());
        }
        let schema = rig_core::schemars::Schema::try_from(request.schema.clone())
            .map_err(|e| anyhow::anyhow!("bad schema: {e}"))?;
        let system = format!(
            "{}\n\nAnswer with one JSON object matching this schema, and nothing else:\n{}",
            request.system, request.schema
        );
        let call = CompletionRequest::new(request.prompt.clone())
            .preamble(system)
            .output_schema(schema);
        let response = config.client().chat(self.model.clone()).call(call).await?;
        parse_json_text(&response.text())
    }
}
