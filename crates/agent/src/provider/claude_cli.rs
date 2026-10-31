//! `claude -p --output-format json --json-schema …` as a subprocess: uses the
//! host's signed-in Claude Code, so no key lives in axioms.

use super::{JsonRequest, ModelProvider, parse_json_text};
use anyhow::{Context, bail};
use async_trait::async_trait;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

const TIMEOUT: Duration = Duration::from_secs(240);

pub struct ClaudeCli {
    bin: String,
    model: String,
}

impl ClaudeCli {
    pub fn new(bin: impl Into<String>, model: impl Into<String>) -> Self {
        ClaudeCli {
            bin: bin.into(),
            model: model.into(),
        }
    }
}

#[async_trait]
impl ModelProvider for ClaudeCli {
    fn label(&self) -> String {
        format!("claude-cli:{}", self.model)
    }

    async fn complete_json(&self, request: &JsonRequest) -> anyhow::Result<serde_json::Value> {
        let mut child = Command::new(&self.bin)
            .args(["-p", "--output-format", "json", "--no-session-persistence"])
            .args([
                "--model",
                &self.model,
                "--tools",
                "",
                "--setting-sources",
                "",
            ])
            .arg("--strict-mcp-config")
            .args(["--system-prompt", &request.system])
            .args(["--json-schema", &request.schema.to_string()])
            // A neutral directory, so no project instructions leak in.
            .current_dir(std::env::temp_dir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("spawning `{}`", self.bin))?;
        let mut stdin = child.stdin.take().context("claude stdin")?;
        stdin.write_all(request.prompt.as_bytes()).await?;
        drop(stdin);
        let out = tokio::time::timeout(TIMEOUT, child.wait_with_output())
            .await
            .context("claude -p timed out")??;
        if !out.status.success() {
            bail!(
                "claude -p failed ({}): {}{}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim(),
                String::from_utf8_lossy(&out.stdout)
                    .chars()
                    .take(400)
                    .collect::<String>()
            );
        }
        let envelope: serde_json::Value =
            serde_json::from_slice(&out.stdout).context("claude -p output is not JSON")?;
        if envelope["is_error"].as_bool() == Some(true) {
            bail!("claude -p reported an error: {}", envelope["result"]);
        }
        match envelope.get("structured_output") {
            Some(v) if !v.is_null() => Ok(v.clone()),
            _ => parse_json_text(envelope["result"].as_str().unwrap_or_default()),
        }
    }
}
