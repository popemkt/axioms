//! A long-lived, self-healing connection: the server and the worker read
//! the subscribed cache through it, and it reconnects when SpacetimeDB
//! restarts.

use crate::conn::{Config, Live, connect};
use std::sync::{Arc, RwLock, mpsc};
use std::time::Duration;

pub struct Mirror {
    pub cfg: Config,
    live: RwLock<Option<Arc<Live>>>,
}

impl Mirror {
    /// Start connecting in the background; [`Mirror::get`] is `None` until
    /// the first subscription is applied, and while reconnecting.
    pub fn start(cfg: Config) -> Arc<Mirror> {
        let mirror = Arc::new(Mirror {
            cfg,
            live: RwLock::new(None),
        });
        let m = mirror.clone();
        std::thread::Builder::new()
            .name("spacetimedb-mirror".into())
            .spawn(move || m.keep_connected())
            .expect("spawn mirror thread");
        mirror
    }

    pub fn get(&self) -> Option<Arc<Live>> {
        self.live.read().unwrap_or_else(|p| p.into_inner()).clone()
    }

    fn set(&self, live: Option<Arc<Live>>) {
        *self.live.write().unwrap_or_else(|p| p.into_inner()) = live;
    }

    fn keep_connected(&self) {
        loop {
            let (lost, gone) = mpsc::channel::<()>();
            match connect(&self.cfg, move || {
                let _ = lost.send(());
            }) {
                Ok(live) => {
                    tracing::info!(me = %live.me, uri = %self.cfg.uri, "spacetimedb connected");
                    self.set(Some(Arc::new(live)));
                    let _ = gone.recv();
                    self.set(None);
                }
                Err(e) => tracing::warn!("{e:#}"),
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }
}
