//! What to write to a kb store to make it hold the codec's nodes: pure
//! diffs from the nodes wanted and the nodes kb has, run by [`super::sync`].
//!
//! Only what axioms owns is diffed. A concept node may carry the person's
//! own tags, fields and children in kb; those are never unset or moved.

use crate::node::{KbNode, NodeId, PropValue};
use crate::schema::{FIELD, FIELDS, TAG, TYPE};
use std::collections::{BTreeSet, HashMap};

/// Fields axioms adds to but never takes from: a node's tags, a tag's
/// templated fields. Every other field it writes is its own.
const ADDITIVE: [&str; 2] = [TYPE, FIELDS];

/// How a missing node is minted: a field or tag through kb's own define
/// actions, anything else as a plain node carrying its tags.
#[derive(Debug, PartialEq)]
pub enum Create {
    Field {
        id: NodeId,
        name: String,
    },
    Tag {
        id: NodeId,
        name: String,
    },
    Node {
        id: NodeId,
        text: String,
        types: Vec<NodeId>,
        parent: Option<NodeId>,
    },
}

#[derive(Debug, Default, PartialEq)]
pub struct Update {
    pub id: NodeId,
    pub text: Option<String>,
    pub set: Vec<(NodeId, PropValue)>,
    pub unset: Vec<(NodeId, PropValue)>,
}

/// The nodes to mint, in the wanted order (so a parent precedes its child).
pub fn creates(want: &[KbNode], have: &HashMap<NodeId, KbNode>) -> Vec<Create> {
    let parents: HashMap<&str, &str> = want
        .iter()
        .flat_map(|p| p.children.iter().map(move |c| (c.as_str(), p.id.as_str())))
        .collect();
    want.iter()
        .filter(|n| !have.contains_key(&n.id))
        .map(|n| {
            let types: Vec<NodeId> = n.refs(TYPE).map(str::to_owned).collect();
            let (id, name) = (n.id.clone(), n.text.clone());
            if types == [FIELD] {
                Create::Field { id, name }
            } else if types == [TAG] {
                Create::Tag { id, name }
            } else {
                let parent = parents.get(n.id.as_str()).map(|p| (*p).to_owned());
                Create::Node {
                    id,
                    text: name,
                    types,
                    parent,
                }
            }
        })
        .collect()
}

/// The prop and text edits that bring `have` to `want`, if any.
pub fn update(want: &KbNode, have: &KbNode) -> Option<Update> {
    let mut u = Update {
        id: want.id.clone(),
        ..Update::default()
    };
    if want.text != have.text {
        u.text = Some(want.text.clone());
    }
    let fields: BTreeSet<&str> = want
        .props
        .keys()
        .chain(have.props.keys().filter(|k| k.starts_with("ax.")))
        .map(String::as_str)
        .collect();
    for f in fields {
        let (w, h) = (want.values(f), have.values(f));
        for v in w.iter().filter(|v| !h.contains(v)) {
            u.set.push((f.to_owned(), v.clone()));
        }
        if !ADDITIVE.contains(&f) {
            for v in h.iter().filter(|v| !w.contains(v)) {
                u.unset.push((f.to_owned(), v.clone()));
            }
        }
    }
    let changed = u.text.is_some() || !u.set.is_empty() || !u.unset.is_empty();
    changed.then_some(u)
}

/// Whether a parent's children, read from kb, are out of the wanted order.
/// Children that are not axioms' (the person's own notes) are ignored.
pub fn misplaced(want: &KbNode, have_children: &[NodeId]) -> bool {
    let ours: Vec<&NodeId> = have_children
        .iter()
        .filter(|c| want.children.contains(c))
        .collect();
    ours.len() != want.children.len() || ours.iter().zip(&want.children).any(|(a, b)| *a != b)
}

/// Owned nodes kb has that are no longer wanted.
pub fn deletes(want: &[KbNode], owned: &[NodeId]) -> Vec<NodeId> {
    let keep: BTreeSet<&str> = want.iter().map(|n| n.id.as_str()).collect();
    owned
        .iter()
        .filter(|id| !keep.contains(id.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{CONCEPT, KNOWLEDGE, STATEMENT};

    fn r(id: &str) -> PropValue {
        PropValue::Ref(id.into())
    }

    fn concept(id: &str) -> KbNode {
        KbNode::new(id, id).with(TYPE, r(CONCEPT))
    }

    #[test]
    fn mints_fields_and_tags_by_their_define_actions() {
        let want = [
            KbNode::new("ax.f.x", "x").with(TYPE, r(FIELD)),
            KbNode::new("ax.tag.t", "t").with(TYPE, r(TAG)),
            concept("a"),
        ];
        let made = creates(&want, &HashMap::new());
        assert!(matches!(&made[0], Create::Field { name, .. } if name == "x"));
        assert!(matches!(&made[1], Create::Tag { name, .. } if name == "t"));
        assert!(
            matches!(&made[2], Create::Node { types, parent: None, .. } if types == &[CONCEPT])
        );
    }

    #[test]
    fn a_child_is_minted_under_its_parent() {
        let mut topics = KbNode::new("ax.topics", "topics");
        topics.children = vec!["a".into()];
        let made = creates(&[topics, concept("a")], &HashMap::new());
        assert!(matches!(&made[1], Create::Node { parent: Some(p), .. } if p == "ax.topics"));
    }

    #[test]
    fn keeps_the_persons_tags_and_fields_but_owns_its_own() {
        let want = concept("a")
            .with(STATEMENT, PropValue::Str("new".into()))
            .with(KNOWLEDGE, r("ax.k.known"));
        let have = concept("a")
            .with(TYPE, r("their-tag"))
            .with("their-field", PropValue::Str("note".into()))
            .with(STATEMENT, PropValue::Str("old".into()))
            .with(KNOWLEDGE, r("ax.k.unknown"))
            .with("ax.f.needs", r("b"));
        let u = update(&want, &have).expect("changed");
        assert_eq!(u.text, None);
        assert_eq!(
            u.set,
            [
                (KNOWLEDGE.into(), r("ax.k.known")),
                (STATEMENT.into(), PropValue::Str("new".into()))
            ]
        );
        assert_eq!(
            u.unset,
            [
                (KNOWLEDGE.into(), r("ax.k.unknown")),
                ("ax.f.needs".into(), r("b")),
                (STATEMENT.into(), PropValue::Str("old".into()))
            ]
        );
        assert_eq!(update(&want, &want), None);
    }

    #[test]
    fn order_ignores_the_persons_children() {
        let mut want = KbNode::new("ax.topics", "topics");
        want.children = vec!["a".into(), "b".into()];
        let ids = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(!misplaced(&want, &ids(&["a", "note", "b"])));
        assert!(misplaced(&want, &ids(&["b", "a"])));
        assert!(misplaced(&want, &ids(&["a"])));
    }

    #[test]
    fn deletes_only_what_is_no_longer_wanted() {
        let gone = deletes(&[concept("a")], &["a".into(), "b".into()]);
        assert_eq!(gone, ["b"]);
    }
}
