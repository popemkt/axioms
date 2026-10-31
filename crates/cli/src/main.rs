//! `axioms learn "<topic>"`: decompose a topic and print its blueprint tree
//! and roadmap. Shares its library with the server.

mod render;

use anyhow::Context;
use axioms_agent::{Learner, LlmSource, provider};
use axioms_core::{ConceptId, Knowledge, Library, roadmap, view};

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Parser)]
#[command(version, about = "Learn any topic starting from what you already know")]
struct Cli {
    /// The library file (kb-style JSONL nodes). With AXIOMS_STORE=kb the
    /// library is kept in the kb store at AXIOMS_KB_ROOT instead.
    #[arg(long, env = "AXIOMS_DATA", default_value_os_t = axioms_store::default_path())]
    data: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// State a topic: decompose it (if new) and print its tree and roadmap.
    Learn {
        topic: String,
        /// Levels to decompose a new topic (1 or 2).
        #[arg(long, default_value_t = 2)]
        depth: u8,
    },
    /// Print a stated topic's tree and roadmap without calling the model.
    Show { topic: String },
    /// Decompose one concept a level deeper, then print the topic.
    Decompose { concept: String, topic: String },
    /// Mark concepts known (your axioms), learning, or unknown.
    Mark {
        #[arg(value_parser = parse_knowledge)]
        knowledge: Knowledge,
        concepts: Vec<String>,
    },
    /// List stated topics, most recent first.
    Topics,
}

fn parse_knowledge(s: &str) -> Result<Knowledge, String> {
    match s {
        "known" => Ok(Knowledge::Known),
        "learning" => Ok(Knowledge::Learning),
        "unknown" => Ok(Knowledge::Unknown),
        _ => Err("expected known | learning | unknown".into()),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "warn".into()))
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();
    let store = axioms_store::open(cli.data.clone())?;
    let lib = Mutex::new(store.load()?);
    let learner = || -> anyhow::Result<Learner> {
        Ok(Learner::new(Arc::new(
            LlmSource::new(provider::from_env()?),
        )))
    };
    let topic = match cli.command {
        Command::Learn { topic, depth } => {
            let learner = learner()?;
            eprintln!("decomposing with {} …", learner.label());
            Some(learner.learn(&lib, &topic, depth).await?)
        }
        Command::Show { topic } => Some(find(&lib.lock().unwrap(), &topic)?),
        Command::Decompose { concept, topic } => {
            let (id, goal) = {
                let l = lib.lock().unwrap();
                (find(&l, &concept)?, find(&l, &topic)?)
            };
            learner()?.decompose(&lib, &id, Some(&goal), 1).await?;
            Some(goal)
        }
        Command::Mark {
            knowledge,
            concepts,
        } => {
            let mut l = lib.lock().unwrap();
            for name in &concepts {
                let id = find(&l, name)?;
                l.graph.get_mut(&id).expect("found").knowledge = knowledge;
            }
            None
        }
        Command::Topics => {
            for t in &lib.lock().unwrap().topics {
                println!("{t}");
            }
            None
        }
    };
    let l = lib.into_inner().unwrap();
    store.save(&l)?;
    if let Some(goal) = topic {
        let v = view(&l.graph, &goal).context("topic vanished")?;
        print!("{}", render::tree(&v));
        println!();
        print!("{}", render::roadmap(&roadmap(&v)));
    }
    Ok(())
}

/// A concept by id or by name.
fn find(lib: &Library, key: &str) -> anyhow::Result<ConceptId> {
    let id = ConceptId::from(key);
    if lib.graph.get(&id).is_some() {
        return Ok(id);
    }
    match lib.graph.named(key).as_slice() {
        [one] => Ok(one.id.clone()),
        [] => anyhow::bail!("no concept `{key}`"),
        many => anyhow::bail!(
            "`{key}` is ambiguous: {}",
            many.iter()
                .map(|c| c.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
