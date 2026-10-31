//! axioms core: the concept graph, its view projection, the roadmap and the
//! merge of a decomposition into the graph. Pure: no IO, no async, no LLM.

pub mod concept;
pub mod graph;
pub mod merge;
pub mod roadmap;
pub mod view;

pub use concept::{Concept, ConceptId, Knowledge, normalize};
pub use graph::{ConceptGraph, GraphError};
pub use merge::{Collision, Decomposition, Merged, Proposal, Resolution};
pub use roadmap::{Roadmap, Step, roadmap};
pub use view::{Status, View, ViewNode, view};
