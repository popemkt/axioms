//! [`SpacetimeStore`]: the [`Store`] contract over SpacetimeDB, so the CLI
//! (and the server, if asked) keep a library there like in a file. Load is
//! the shared graph with this client's knowledge; save writes what the
//! library adds or changes. It never deletes: the graph is shared.

use crate::conn::{Config, connect};
use crate::rows::{library, send};
use axioms_core::{Library, diff};
use axioms_store::Store;

pub struct SpacetimeStore {
    cfg: Config,
}

impl SpacetimeStore {
    pub fn new(cfg: Config) -> Self {
        SpacetimeStore { cfg }
    }

    /// Run `f` on a plain thread: the SDK runs its own runtime there, and a
    /// caller's async runtime (the CLI's, the server's) is never blocked in.
    fn off_runtime<T: Send>(
        &self,
        f: impl FnOnce() -> anyhow::Result<T> + Send,
    ) -> anyhow::Result<T> {
        std::thread::scope(|s| s.spawn(f).join().expect("spacetimedb store thread"))
    }
}

impl Store for SpacetimeStore {
    fn describe(&self) -> String {
        format!(
            "spacetimedb {} ({}) as {}",
            self.cfg.uri, self.cfg.db, self.cfg.creds
        )
    }

    fn load(&self) -> anyhow::Result<Library> {
        self.off_runtime(|| {
            let live = connect(&self.cfg, || {})?;
            Ok(library(&live, Some(live.me)))
        })
    }

    fn save(&self, lib: &Library) -> anyhow::Result<()> {
        self.off_runtime(|| {
            let live = connect(&self.cfg, || {})?;
            let before = library(&live, Some(live.me));
            for w in send(&live, &diff(&before, lib))? {
                w.blocking_recv()
                    .map_err(|_| anyhow::anyhow!("spacetimedb connection dropped a write"))?
                    .map_err(|e| anyhow::anyhow!("spacetimedb refused a write: {e}"))?;
            }
            Ok(())
        })
    }
}
