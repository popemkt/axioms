//! `axioms-worker`: answer decompose requests from SpacetimeDB with the
//! model. The server runs the same loop in-process unless
//! `AXIOMS_WORKER=0`; run this to add workers elsewhere.

use axioms_agent::{Learner, LlmSource, provider};
use axioms_live::{Config, Mirror, worker};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();
    let learner = Arc::new(Learner::new(Arc::new(
        LlmSource::new(provider::from_env()?),
    )));
    let mirror = Mirror::start(Config::from_env("worker"));
    tracing::info!(provider = %learner.label(), "worker starting");
    worker::run(mirror, learner, worker::pace()).await;
    Ok(())
}
