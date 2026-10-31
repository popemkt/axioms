//! Persistence for a [`Library`]. One contract ([`Store`]), every
//! implementation goes through the same kb-shaped node codec, so the JSONL
//! file and the in-memory store can never disagree about the data model.

mod codec;
mod jsonl;
mod memory;
pub mod node;
pub mod schema;

pub use codec::{from_nodes, to_nodes};
pub use jsonl::JsonlStore;
pub use memory::MemoryStore;

use axioms_core::Library;

pub trait Store: Send + Sync {
    fn load(&self) -> anyhow::Result<Library>;
    fn save(&self, library: &Library) -> anyhow::Result<()>;
}

/// Where the library lives unless told otherwise: `$AXIOMS_DATA`, else
/// `$XDG_DATA_HOME/axioms/nodes.jsonl`, else `~/.local/share/axioms/nodes.jsonl`.
/// The CLI and the server share it, so axioms carry over between them.
pub fn default_path() -> std::path::PathBuf {
    if let Some(p) = std::env::var_os("AXIOMS_DATA") {
        return p.into();
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".local/share")))
        .unwrap_or_else(|| ".".into());
    base.join("axioms").join("nodes.jsonl")
}
