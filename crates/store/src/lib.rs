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
