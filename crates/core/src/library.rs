//! Everything a person has: the concept graph (axioms included) and the
//! topics they have stated, most recent first. The unit a store persists.

use crate::concept::{Concept, ConceptId};
use crate::graph::ConceptGraph;

#[derive(Debug, Clone, Default)]
pub struct Library {
    pub graph: ConceptGraph,
    pub topics: Vec<ConceptId>,
}

impl Library {
    /// The concept for a stated topic: reuse one with the same name, or
    /// create it. Either way it becomes the most recent topic.
    pub fn state_topic(&mut self, name: &str) -> ConceptId {
        let id = match self.graph.named(name).first() {
            Some(c) => c.id.clone(),
            None => {
                let id = self.graph.fresh_id(name);
                self.graph.upsert(Concept::new(id.clone(), name.trim()));
                id
            }
        };
        self.topics.retain(|t| t != &id);
        self.topics.insert(0, id.clone());
        id
    }
}
