//! An in-memory store, for tests and throwaway sessions.

use crate::node::KbNode;
use crate::{Store, codec};
use axioms_core::Library;
use std::sync::Mutex;

#[derive(Default)]
pub struct MemoryStore {
    nodes: Mutex<Vec<KbNode>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn nodes(&self) -> Vec<KbNode> {
        self.nodes.lock().expect("store lock").clone()
    }
}

impl Store for MemoryStore {
    fn describe(&self) -> String {
        "memory".into()
    }

    fn load(&self) -> anyhow::Result<Library> {
        codec::from_nodes(&self.nodes.lock().expect("store lock"))
    }

    fn save(&self, library: &Library) -> anyhow::Result<()> {
        *self.nodes.lock().expect("store lock") = codec::to_nodes(library);
        Ok(())
    }
}
