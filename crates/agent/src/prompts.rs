//! The prompt contracts: what the model is asked, and the JSON schema its
//! answer must match. The decomposition schema is exactly
//! [`axioms_core::Decomposition`]'s serde shape. See docs/DESIGN.md.

use crate::source::DecomposeRequest;
use axioms_core::Collision;
use serde_json::{Value, json};
use std::fmt::Write;

pub const DECOMPOSE_SYSTEM: &str = "\
You build learning blueprints, like a Lean blueprint for a theorem: a target \
concept and the prerequisite concepts it needs, as a dependency graph. An edge \
\"A needs B\" means a learner must understand B before A can make sense. \
Prefer standard textbook names (short noun phrases, no parentheses). For every \
concept give: a one-line canonical statement (the precise definition or claim, \
as a textbook states it; plain-text math is fine); an explanation of 2-4 \
sentences that builds intuition assuming its prerequisites; and one check \
question answerable in a sentence or two. Answer only with the JSON object.";

pub fn decompose_prompt(r: &DecomposeRequest) -> String {
    let mut p = format!("Target concept: {}\n", r.target);
    if !r.statement.is_empty() {
        let _ = writeln!(p, "Its statement: {}", r.statement);
    }
    if let Some(goal) = r.goal.as_ref().filter(|g| **g != r.target) {
        let _ = writeln!(p, "Learning goal it serves: {goal}");
    }
    if !r.needed_for.is_empty() {
        let _ = writeln!(p, "It is needed for: {}", r.needed_for.join("; "));
    }
    let _ = write!(
        p,
        "\nList the 3-6 most important direct prerequisites of the target in \
         `target.needs`, and describe each in `concepts`. Also restate the \
         target itself in `target` (name, statement, explanation, check)."
    );
    if r.depth >= 2 {
        p.push_str(
            " Then go one level deeper: give each direct prerequisite its own 2-4 \
             prerequisites (in its `needs`, each also described in `concepts`). \
             Concepts may share prerequisites; list each concept once.",
        );
    } else {
        p.push_str(" Leave each prerequisite's `needs` empty.");
    }
    if !r.known.is_empty() {
        let _ = write!(
            p,
            "\n\nThe learner already knows these (their axioms). Do not decompose \
             them, but name one in a `needs` list, spelled exactly, where it is a \
             prerequisite: {}",
            r.known.join("; ")
        );
    }
    if !r.existing.is_empty() {
        let _ = write!(
            p,
            "\n\nConcepts already in the learner's graph. When you mean one of \
             these, use its name exactly as written: {}",
            r.existing.join("; ")
        );
    }
    p
}

pub fn decomposition_schema() -> Value {
    let concept = |needs_desc: &str| {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "statement": {"type": "string"},
                "explanation": {"type": "string"},
                "check": {"type": "string"},
                "needs": {"type": "array", "items": {"type": "string"}, "description": needs_desc}
            },
            "required": ["name", "statement", "explanation", "check", "needs"],
            "additionalProperties": false
        })
    };
    json!({
        "type": "object",
        "properties": {
            "target": concept("names of the target's direct prerequisites"),
            "concepts": {"type": "array", "items": concept("names of this concept's prerequisites")}
        },
        "required": ["target", "concepts"],
        "additionalProperties": false
    })
}

pub const SAME_SYSTEM: &str = "\
You deduplicate a concept graph. For each proposed concept, decide whether it \
is the same concept as one of the existing candidates (same idea, possibly \
worded differently) or a different concept that shares a name (e.g. \"ring\" \
in algebra vs. a ring buffer). Answer only with the JSON object.";

pub fn same_prompt(collisions: &[Collision]) -> String {
    let mut p = String::new();
    for (i, c) in collisions.iter().enumerate() {
        let _ = writeln!(
            p,
            "[{i}] proposed: {} — {}",
            c.proposal.name, c.proposal.statement
        );
        for e in &c.existing {
            let _ = writeln!(
                p,
                "    candidate id `{}`: {} — {}",
                e.id, e.name, e.statement
            );
        }
    }
    p.push_str("\nFor every index, answer `same_as` = the matching candidate id, or null.");
    p
}

pub fn same_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "answers": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "index": {"type": "integer"},
                        "same_as": {"type": ["string", "null"]}
                    },
                    "required": ["index", "same_as"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["answers"],
        "additionalProperties": false
    })
}
