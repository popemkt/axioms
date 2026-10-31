//! A JSONL file of kb nodes, one per line, rewritten atomically on save.
//! A node keeps its `createdAt`, and its `updatedAt` moves only when its
//! content changed.

use crate::node::KbNode;
use crate::{Store, codec};
use anyhow::Context;
use axioms_core::Library;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub struct JsonlStore {
    path: PathBuf,
}

impl JsonlStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        JsonlStore { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn read(&self) -> anyhow::Result<Vec<KbNode>> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e).with_context(|| format!("reading {}", self.path.display())),
        };
        text.lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
            .map(|(i, l)| {
                serde_json::from_str(l)
                    .with_context(|| format!("{}:{}", self.path.display(), i + 1))
            })
            .collect()
    }
}

impl Store for JsonlStore {
    fn load(&self) -> anyhow::Result<Library> {
        codec::from_nodes(&self.read()?)
    }

    fn save(&self, library: &Library) -> anyhow::Result<()> {
        let existing = self.read()?;
        let old: HashMap<String, KbNode> =
            existing.iter().map(|n| (n.id.clone(), n.clone())).collect();
        let now = humantime::format_rfc3339_millis(SystemTime::now()).to_string();
        let mut out = String::new();
        let ours = codec::to_nodes(library);
        // Nodes the codec does not own (another tool's, in a shared kb
        // store) are kept as they are, before ours.
        for n in existing.iter().filter(|n| !codec::owns(n)) {
            out.push_str(&serde_json::to_string(n)?);
            out.push('\n');
        }
        for mut n in ours {
            match old.get(&n.id) {
                Some(o) if o.same_content(&n) => {
                    n.created_at.clone_from(&o.created_at);
                    n.updated_at.clone_from(&o.updated_at);
                }
                Some(o) => {
                    n.created_at.clone_from(&o.created_at);
                    n.updated_at.clone_from(&now);
                }
                None => {
                    n.created_at.clone_from(&now);
                    n.updated_at.clone_from(&now);
                }
            }
            out.push_str(&serde_json::to_string(&n)?);
            out.push('\n');
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("jsonl.tmp");
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(out.as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&tmp, &self.path)
            .with_context(|| format!("writing {}", self.path.display()))
    }
}
