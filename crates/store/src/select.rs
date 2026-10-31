//! Which store to use, from the environment:
//!
//! - `AXIOMS_STORE=jsonl` (default): the JSONL file at the given path.
//! - `AXIOMS_STORE=kb`: the kb store at `AXIOMS_KB_ROOT` (the directory
//!   holding `.kb/`), through the `kb` binary (`AXIOMS_KB_BIN`, default `kb`).
//!   `KB_HOME`, if set, reaches kb as usual.

use crate::{JsonlStore, KbStore, Store};
use anyhow::{Context, bail};
use std::path::PathBuf;

pub fn open(jsonl: PathBuf) -> anyhow::Result<Box<dyn Store>> {
    let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    match var("AXIOMS_STORE").as_deref().unwrap_or("jsonl") {
        "jsonl" => Ok(Box::new(JsonlStore::new(jsonl))),
        "kb" => {
            let root = var("AXIOMS_KB_ROOT").context("AXIOMS_STORE=kb needs AXIOMS_KB_ROOT")?;
            let bin = var("AXIOMS_KB_BIN").unwrap_or_else(|| "kb".into());
            Ok(Box::new(KbStore::new(bin, root)))
        }
        other => bail!("AXIOMS_STORE={other}: expected jsonl or kb"),
    }
}
