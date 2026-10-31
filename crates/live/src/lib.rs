//! axioms over SpacetimeDB (docs/DESIGN.md → SpacetimeDB): a mirror of the
//! shared graph for the server, the worker that answers decompose requests
//! with the model, and [`SpacetimeStore`] for the `Store` contract.

#[allow(clippy::all, clippy::pedantic, unused, rustdoc::all)]
mod bindings;
mod conn;
mod mirror;
pub mod rows;
mod store;
pub mod worker;

pub use conn::{Config, Live};
pub use mirror::Mirror;
pub use spacetimedb_sdk::Identity;
pub use store::SpacetimeStore;

/// The store the environment names: `AXIOMS_STORE=spacetime` here, any
/// other through [`axioms_store::open`].
pub fn open_store(
    jsonl: std::path::PathBuf,
    role: &str,
) -> anyhow::Result<Box<dyn axioms_store::Store>> {
    if std::env::var("AXIOMS_STORE").as_deref() == Ok("spacetime") {
        return Ok(Box::new(SpacetimeStore::new(Config::from_env(role))));
    }
    axioms_store::open(jsonl)
}
