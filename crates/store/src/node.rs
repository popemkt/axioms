//! kb's node shape (kb DESIGN.md → Data model): everything is a node, and a
//! relationship is a prop whose key is a field node's id.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type NodeId = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "lowercase")]
pub enum PropValue {
    Str(String),
    Num(f64),
    Bool(bool),
    Ref(NodeId),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KbNode {
    pub id: NodeId,
    pub text: String,
    #[serde(default)]
    pub props: BTreeMap<NodeId, Vec<PropValue>>,
    #[serde(default)]
    pub children: Vec<NodeId>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

impl KbNode {
    pub fn new(id: impl Into<NodeId>, text: impl Into<String>) -> Self {
        KbNode {
            id: id.into(),
            text: text.into(),
            props: BTreeMap::new(),
            children: Vec::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    pub fn with(mut self, field: &str, value: PropValue) -> Self {
        self.props.entry(field.to_owned()).or_default().push(value);
        self
    }

    pub fn values(&self, field: &str) -> &[PropValue] {
        self.props.get(field).map_or(&[], Vec::as_slice)
    }

    /// The first string value of `field`, or "".
    pub fn str(&self, field: &str) -> &str {
        self.values(field)
            .iter()
            .find_map(|v| match v {
                PropValue::Str(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("")
    }

    pub fn refs(&self, field: &str) -> impl Iterator<Item = &str> {
        self.values(field).iter().filter_map(|v| match v {
            PropValue::Ref(id) => Some(id.as_str()),
            _ => None,
        })
    }

    /// Same content, ignoring timestamps.
    pub fn same_content(&self, other: &KbNode) -> bool {
        self.id == other.id
            && self.text == other.text
            && self.props == other.props
            && self.children == other.children
    }
}
