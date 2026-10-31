//! A [`Store`] that keeps the library in a kb store, through kb's public
//! action registry (`kb mcp`), never its files. The codec spells the library
//! as nodes exactly as the JSONL store writes them; this store makes kb hold
//! those nodes with kb's own actions, so kb validates every write.
//! See docs/DESIGN.md → kb integration.

mod mcp;
mod plan;

use crate::node::{KbNode, NodeId};
use crate::schema::{CONCEPT, TOPICS};
use crate::{Store, codec};
use anyhow::Context;
use axioms_core::Library;
use mcp::Session;
use plan::Create;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct KbStore {
    bin: String,
    root: PathBuf,
    home: Option<PathBuf>,
}

impl KbStore {
    /// A kb store at `root` (the directory holding `.kb/`), driven by the
    /// `kb` binary `bin`.
    pub fn new(bin: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        KbStore {
            bin: bin.into(),
            root: root.into(),
            home: None,
        }
    }

    /// Run kb with this home store (`KB_HOME`), where kb keeps the list of
    /// roots a person opened, instead of the person's own.
    pub fn with_home(mut self, home: impl Into<PathBuf>) -> Self {
        self.home = Some(home.into());
        self
    }

    fn session(&self) -> anyhow::Result<Session> {
        Session::open(&self.bin, &self.root, self.home.as_deref())
            .with_context(|| format!("kb store at {}", self.root.display()))
    }
}

impl Store for KbStore {
    fn describe(&self) -> String {
        format!("kb store at {}", self.root.display())
    }

    fn load(&self) -> anyhow::Result<Library> {
        let mut kb = self.session()?;
        let mut nodes = Vec::new();
        for id in concept_ids(&mut kb)?.into_iter().chain([TOPICS.to_owned()]) {
            nodes.extend(fetch(&mut kb, &id)?);
        }
        codec::from_nodes(&nodes)
    }

    fn save(&self, library: &Library) -> anyhow::Result<()> {
        let want = codec::to_nodes(library);
        let mut kb = self.session()?;
        let owned = concept_ids(&mut kb)?;
        let mut have = HashMap::new();
        for id in want.iter().map(|n| &n.id).chain(&owned) {
            if let Some(n) = fetch(&mut kb, id)? {
                have.insert(n.id.clone(), n);
            }
        }
        // Mint first, so every ref a later write makes has its target.
        for c in plan::creates(&want, &have) {
            let made = create(&mut kb, c)?;
            have.insert(made.id.clone(), made);
        }
        for n in want.iter().filter(|n| !n.children.is_empty()) {
            let now = fetch(&mut kb, &n.id)?
                .map(|n| n.children)
                .unwrap_or_default();
            if plan::misplaced(n, &now) {
                for (i, child) in n.children.iter().enumerate() {
                    let at = json!({ "id": child, "parent": n.id, "position": i });
                    kb.must("node_update", at)?;
                }
            }
        }
        for n in &want {
            if let Some(u) = have.get(&n.id).and_then(|h| plan::update(n, h)) {
                let mut args = json!({ "id": u.id });
                if let Some(text) = u.text {
                    args["text"] = json!(text);
                }
                args["unsetProps"] = props(&u.unset);
                args["setProps"] = props(&u.set);
                kb.must("node_update", args)?;
            }
        }
        for id in plan::deletes(&want, &owned) {
            kb.must(
                "node_delete",
                json!({ "id": id, "descendants": "reparent" }),
            )?;
        }
        Ok(())
    }
}

/// The ids of every node tagged #concept.
fn concept_ids(kb: &mut Session) -> anyhow::Result<Vec<NodeId>> {
    let query = format!(
        r#"[:find ?id :where [?t :node/id "{CONCEPT}"] [?n :f/sys.f.type ?t] [?n :node/id ?id]]"#
    );
    let out = kb.must("graph_query", json!({ "query": query }))?;
    let rows = out["rows"].as_array().cloned().unwrap_or_default();
    Ok(rows
        .iter()
        .filter_map(|r| r[0].as_str().map(str::to_owned))
        .collect())
}

/// A node with its children's ids, or `None` if kb has no such node.
fn fetch(kb: &mut Session, id: &str) -> anyhow::Result<Option<KbNode>> {
    let mut node = match kb.call("node_get", json!({ "id": id, "depth": 1 }))? {
        Ok(out) => out["node"].clone(),
        Err(r) if r.code == "not_found" => return Ok(None),
        Err(r) => anyhow::bail!("kb node_get {id}: {}", r.message),
    };
    let children: Vec<Value> = node["children"]
        .as_array()
        .map(|cs| cs.iter().map(|c| c["id"].clone()).collect())
        .unwrap_or_default();
    node["children"] = Value::Array(children);
    serde_json::from_value(node).with_context(|| format!("reading kb node {id}"))
}

/// Mint a node with kb's action for its kind; answers it as kb now holds it.
fn create(kb: &mut Session, c: Create) -> anyhow::Result<KbNode> {
    let id = match c {
        Create::Field { id, name } => {
            kb.must("field_define", json!({ "id": id, "name": name }))?;
            id
        }
        Create::Tag { id, name } => {
            kb.must("tag_define", json!({ "id": id, "name": name }))?;
            id
        }
        Create::Node {
            id,
            text,
            types,
            parent,
        } => {
            let tags: Vec<Value> = types
                .iter()
                .map(|t| json!({ "field": crate::schema::TYPE, "value": { "t": "ref", "v": t } }))
                .collect();
            let mut args = json!({ "id": id, "text": text, "props": tags });
            if let Some(p) = parent {
                args["parent"] = json!(p);
            }
            kb.must("node_add", args)?;
            id
        }
    };
    fetch(kb, &id)?.with_context(|| format!("kb minted {id} but cannot read it"))
}

fn props(values: &[(NodeId, crate::node::PropValue)]) -> Value {
    values
        .iter()
        .map(|(field, value)| json!({ "field": field, "value": value }))
        .collect()
}
