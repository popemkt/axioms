//! `axioms-server`: the JSON API and the static web UI on one port.
//!
//! - `AXIOMS_ADDR` (default `$HOST:$PORT`, then `127.0.0.1:4401`)
//! - `AXIOMS_DATA`: the library file (see `axioms_store::default_path`)
//! - `AXIOMS_STORE=jsonl|kb`, `AXIOMS_KB_ROOT`: keep it in a kb store instead
//!   (see `axioms_store::open`)
//! - `AXIOMS_STORE=spacetime`: live mode. Reads come from a SpacetimeDB
//!   mirror (`AXIOMS_STDB_URI`, `AXIOMS_STDB_DB`), browsers subscribe and
//!   write through reducers, and the decompose worker runs in-process unless
//!   `AXIOMS_WORKER=0`. Browsers reach SpacetimeDB through this server's own
//!   origin (`relay`), or at `AXIOMS_STDB_PUBLIC_URI` when that is set.
//! - `AXIOMS_WEB`: the web UI directory (default: this repo's `web/`)
//! - provider settings: see `axioms_agent::provider::from_env`

use axioms_agent::{Learner, LlmSource, provider};
use axioms_server::{api, state};
use axum::http::HeaderValue;
use axum::http::header::CACHE_CONTROL;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();
    let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let addr = var("AXIOMS_ADDR").unwrap_or_else(|| {
        let host = var("HOST").unwrap_or_else(|| "127.0.0.1".into());
        format!("{host}:{}", var("PORT").unwrap_or_else(|| "4401".into()))
    });
    let web = std::env::var("AXIOMS_WEB")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../web").into());
    let state = Arc::new(app_state()?);
    // Revalidate static files on every load, so a redeploy is seen at once.
    let web = tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        .service(ServeDir::new(web));
    let app = api::router(state)
        .fallback_service(web)
        .layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

fn app_state() -> anyhow::Result<state::AppState> {
    let learner = || -> anyhow::Result<Learner> {
        Ok(Learner::new(Arc::new(
            LlmSource::new(provider::from_env()?),
        )))
    };
    if std::env::var("AXIOMS_STORE").as_deref() == Ok("spacetime") {
        let cfg = axioms_live::Config::from_env("server");
        let mirror = axioms_live::Mirror::start(cfg.clone());
        if std::env::var("AXIOMS_WORKER").as_deref() != Ok("0") {
            let worker = Arc::new(learner()?);
            let m = mirror.clone();
            tokio::spawn(axioms_live::worker::run(
                m,
                worker,
                axioms_live::worker::pace(),
            ));
        }
        let public_uri = std::env::var("AXIOMS_STDB_PUBLIC_URI").unwrap_or_default();
        tracing::info!(uri = %cfg.uri, db = %cfg.db, %public_uri, "starting live");
        let store = Arc::new(axioms_store::MemoryStore::new());
        let live = state::LiveState {
            mirror,
            public_uri,
            upstream: cfg.uri,
            db: cfg.db,
        };
        return Ok(state::AppState::new(Default::default(), store, learner()?).with_live(live));
    }
    let store: Arc<dyn axioms_store::Store> =
        axioms_store::open(axioms_store::default_path())?.into();
    let lib = store.load()?;
    let learner = learner()?;
    tracing::info!(store = %store.describe(), provider = %learner.label(), concepts = lib.graph.len(), "starting");
    Ok(state::AppState::new(lib, store, learner))
}
