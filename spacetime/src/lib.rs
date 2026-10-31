//! axioms in SpacetimeDB. The concept graph (concepts, their needs, the
//! stated topics) is shared; what each person knows is theirs, keyed by
//! their identity. Model calls cannot run in a reducer, so decomposing is a
//! request row a worker client claims, answers through `put`, and finishes.
//! The graph maths (views, roadmaps, merging) stays in `axioms-core`, in the
//! clients. See docs/DESIGN.md → SpacetimeDB.

mod tables;

use spacetimedb::{Identity, ReducerContext, Table, reducer};
use tables::*;

const KNOWLEDGE: [&str; 3] = ["unknown", "learning", "known"];
const MAX_ID: usize = 160;
const MAX_TEXT: usize = 8_000;

fn check(cond: bool, msg: &str) -> Result<(), String> {
    if cond { Ok(()) } else { Err(msg.to_owned()) }
}

/// Add or update concepts, needs and topics in one transaction. Needs may
/// name concepts added in the same call; a need on no concept is refused.
#[reducer]
pub fn put(
    ctx: &ReducerContext,
    concepts: Vec<ConceptIn>,
    needs: Vec<NeedIn>,
    topics: Vec<String>,
) -> Result<(), String> {
    for c in concepts {
        check(!c.id.is_empty() && c.id.len() <= MAX_ID, "bad concept id")?;
        let long = [&c.name, &c.statement, &c.explanation, &c.check];
        check(long.iter().all(|t| t.len() <= MAX_TEXT), "text too long")?;
        let row = Concept {
            id: c.id,
            name: c.name,
            statement: c.statement,
            explanation: c.explanation,
            check: c.check,
            by: ctx.sender(),
            at: ctx.timestamp,
        };
        match ctx.db.concept().id().find(&row.id) {
            Some(_) => {
                ctx.db.concept().id().update(row);
            }
            None => {
                ctx.db.concept().insert(row);
            }
        }
    }
    let exists = |id: &String| ctx.db.concept().id().find(id).is_some();
    for n in needs {
        check(
            exists(&n.from_id) && exists(&n.to_id),
            "a need on no concept",
        )?;
        check(n.from_id != n.to_id, "a concept cannot need itself")?;
        let key = format!("{}>{}", n.from_id, n.to_id);
        if ctx.db.need().key().find(&key).is_none() {
            ctx.db.need().insert(Need {
                key,
                from_id: n.from_id,
                to_id: n.to_id,
            });
        }
    }
    for t in topics {
        check(exists(&t), "a topic that is no concept")?;
        if ctx.db.topic().concept().find(&t).is_none() {
            ctx.db.topic().insert(Topic {
                concept: t,
                by: ctx.sender(),
                at: ctx.timestamp,
            });
        }
    }
    Ok(())
}

/// Record what the caller knows of a concept.
#[reducer]
pub fn mark(ctx: &ReducerContext, concept: String, state: String) -> Result<(), String> {
    check(
        KNOWLEDGE.contains(&state.as_str()),
        "state is unknown, learning or known",
    )?;
    check(
        ctx.db.concept().id().find(&concept).is_some(),
        "no such concept",
    )?;
    let key = format!("{}:{}", ctx.sender().to_hex(), concept);
    let row = Knowledge {
        key: key.clone(),
        person: ctx.sender(),
        concept,
        state,
    };
    if ctx.db.knowledge().key().find(&key).is_some() {
        ctx.db.knowledge().key().update(row);
    } else {
        ctx.db.knowledge().insert(row);
    }
    Ok(())
}

/// Ask for a topic to be stated and decomposed `depth` levels.
#[reducer]
pub fn request_topic(ctx: &ReducerContext, name: String, depth: u8) -> Result<(), String> {
    let name = name.trim().to_owned();
    check(
        !name.is_empty() && name.len() <= 200,
        "state a topic (at most 200 characters)",
    )?;
    open_request(ctx, "topic", name, String::new(), depth.clamp(1, 2))
}

/// Ask for one concept to be decomposed a level deeper, for `goal`.
#[reducer]
pub fn request_decompose(
    ctx: &ReducerContext,
    concept: String,
    goal: String,
) -> Result<(), String> {
    check(
        ctx.db.concept().id().find(&concept).is_some(),
        "no such concept",
    )?;
    open_request(ctx, "decompose", concept, goal, 1)
}

fn open_request(
    ctx: &ReducerContext,
    kind: &str,
    text: String,
    goal: String,
    depth: u8,
) -> Result<(), String> {
    let open = |r: &Request| r.kind == kind && r.text.eq_ignore_ascii_case(&text);
    for status in ["pending", "working"] {
        if ctx.db.request().status().filter(status).any(|r| open(&r)) {
            return Ok(());
        }
    }
    ctx.db.request().insert(Request {
        id: 0,
        kind: kind.to_owned(),
        text,
        goal,
        depth,
        by: ctx.sender(),
        status: "pending".into(),
        concept: String::new(),
        worker: None,
        note: String::new(),
        at: ctx.timestamp,
    });
    Ok(())
}

/// A worker takes a pending request (or resumes its own after a restart).
#[reducer]
pub fn claim(ctx: &ReducerContext, id: u64) -> Result<(), String> {
    let mut r = ctx.db.request().id().find(id).ok_or("no such request")?;
    let mine = r.status == "working" && r.worker == Some(ctx.sender());
    check(r.status == "pending" || mine, "already taken")?;
    r.status = "working".into();
    r.worker = Some(ctx.sender());
    ctx.db.request().id().update(r);
    Ok(())
}

/// The worker names the concept a request is about (a topic's new id).
#[reducer]
pub fn resolve(ctx: &ReducerContext, id: u64, concept: String) -> Result<(), String> {
    let mut r = own(ctx, id)?;
    r.concept = concept;
    ctx.db.request().id().update(r);
    Ok(())
}

#[reducer]
pub fn finish(ctx: &ReducerContext, id: u64, ok: bool, note: String) -> Result<(), String> {
    let mut r = own(ctx, id)?;
    r.status = if ok { "done" } else { "failed" }.into();
    r.note = note.chars().take(500).collect();
    ctx.db.request().id().update(r);
    Ok(())
}

fn own(ctx: &ReducerContext, id: u64) -> Result<Request, String> {
    let r = ctx.db.request().id().find(id).ok_or("no such request")?;
    let me: Option<Identity> = Some(ctx.sender());
    check(r.worker == me && r.status == "working", "not your request")?;
    Ok(r)
}
