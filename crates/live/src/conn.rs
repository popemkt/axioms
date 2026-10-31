//! One connection to the axioms database, subscribed to every table, with
//! this client's identity kept in a credentials file so it stays the same
//! person across runs.

use crate::bindings::DbConnection;
use anyhow::Context;
use spacetimedb_sdk::{DbContext, Identity, credentials};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

/// Where the database is and who this client is.
///
/// - `AXIOMS_STDB_URI` (default `http://127.0.0.1:4420`)
/// - `AXIOMS_STDB_DB` (default `axioms`)
/// - `AXIOMS_STDB_CREDS`: the credentials file name under
///   `~/.spacetimedb_client_credentials/` (default `axioms-<role>`)
#[derive(Clone, Debug)]
pub struct Config {
    pub uri: String,
    pub db: String,
    pub creds: String,
}

impl Config {
    pub fn from_env(role: &str) -> Self {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        Config {
            uri: var("AXIOMS_STDB_URI").unwrap_or_else(|| "http://127.0.0.1:4420".into()),
            db: var("AXIOMS_STDB_DB").unwrap_or_else(|| "axioms".into()),
            creds: var("AXIOMS_STDB_CREDS").unwrap_or_else(|| format!("axioms-{role}")),
        }
    }
}

const TABLES: [&str; 5] = [
    "SELECT * FROM concept",
    "SELECT * FROM need",
    "SELECT * FROM topic",
    "SELECT * FROM knowledge",
    "SELECT * FROM request",
];

pub struct Live {
    pub conn: DbConnection,
    pub me: Identity,
    /// Set when we close the connection ourselves, so it is not "lost".
    closing: Arc<AtomicBool>,
}

/// Connect and wait until the subscription is applied (the cache is full).
/// `on_lost` runs if the connection later drops.
pub fn connect(cfg: &Config, on_lost: impl FnOnce() + Send + 'static) -> anyhow::Result<Live> {
    let token = credentials::File::new(cfg.creds.clone())
        .load()
        .ok()
        .flatten();
    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let failed = tx.clone();
    let creds = cfg.creds.clone();
    let closing = Arc::new(AtomicBool::new(false));
    let ours = closing.clone();
    let conn = DbConnection::builder()
        .with_uri(cfg.uri.as_str())
        .with_database_name(cfg.db.clone())
        .with_token(token)
        .on_connect(move |ctx, _, token| {
            if let Err(e) = credentials::File::new(creds).save(token) {
                tracing::warn!("keeping the spacetimedb token: {e}");
            }
            let (ok, err) = (tx.clone(), tx);
            ctx.subscription_builder()
                .on_applied(move |_| {
                    let _ = ok.send(Ok(()));
                })
                .on_error(move |_, e| {
                    let _ = err.send(Err(e.to_string()));
                })
                .subscribe(TABLES);
        })
        .on_connect_error(move |_, e| {
            let _ = failed.send(Err(e.to_string()));
        })
        .on_disconnect(move |_, e| {
            if !ours.load(Ordering::SeqCst) {
                tracing::warn!(?e, "spacetimedb connection lost");
                on_lost();
            }
        })
        .build()
        .with_context(|| format!("connecting to spacetimedb at {} ({})", cfg.uri, cfg.db))?;
    conn.run_threaded();
    match rx.recv_timeout(Duration::from_secs(20)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => anyhow::bail!("spacetimedb subscription: {e}"),
        Err(_) => anyhow::bail!("spacetimedb at {} did not answer in 20 s", cfg.uri),
    }
    let me = conn.identity();
    Ok(Live { conn, me, closing })
}

impl Drop for Live {
    fn drop(&mut self) {
        self.closing.store(true, Ordering::SeqCst);
        let _ = self.conn.disconnect();
    }
}
