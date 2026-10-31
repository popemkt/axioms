//! Plain-text rendering of a view (as a tree) and a roadmap.

use axioms_core::{ConceptId, Roadmap, Status, View, ViewNode};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

fn mark(status: Status) -> &'static str {
    match status {
        Status::Goal => "◎",
        Status::Unknown => "○",
        Status::Learning => "◐",
        Status::Known => "●",
    }
}

/// The view as an indented tree. A concept reached twice is printed once;
/// later mentions point back with `↑`.
pub fn tree(view: &View) -> String {
    let nodes: BTreeMap<&ConceptId, &ViewNode> =
        view.nodes.iter().map(|n| (&n.concept.id, n)).collect();
    let mut needs: BTreeMap<&ConceptId, Vec<&ConceptId>> = BTreeMap::new();
    for (a, b) in &view.edges {
        needs.entry(a).or_default().push(b);
    }
    let mut out = String::new();
    let mut seen = BTreeSet::new();
    walk(
        &view.goal, "", true, true, &nodes, &needs, &mut seen, &mut out,
    );
    out.push_str("\n◎ goal  ○ unknown  ◐ learning  ● known (an axiom; ⋯ = hidden below)\n");
    out
}

#[allow(clippy::too_many_arguments)]
fn walk<'v>(
    id: &'v ConceptId,
    prefix: &str,
    last: bool,
    root: bool,
    nodes: &BTreeMap<&ConceptId, &'v ViewNode>,
    needs: &BTreeMap<&ConceptId, Vec<&'v ConceptId>>,
    seen: &mut BTreeSet<&'v ConceptId>,
    out: &mut String,
) {
    let n = nodes[id];
    let branch = if root {
        ""
    } else if last {
        "└─ "
    } else {
        "├─ "
    };
    let again = !seen.insert(id);
    let hidden = if n.hidden_below > 0 {
        format!(" ⋯{}", n.hidden_below)
    } else {
        String::new()
    };
    let tail = if again {
        " ↑".to_string()
    } else {
        format!("  — {}", n.concept.statement)
    };
    let _ = writeln!(
        out,
        "{prefix}{branch}{} {}{hidden}{tail}",
        mark(n.status),
        n.concept.name
    );
    if again {
        return;
    }
    let child_prefix = if root {
        String::new()
    } else {
        format!("{prefix}{}", if last { "   " } else { "│  " })
    };
    let kids = needs.get(id).cloned().unwrap_or_default();
    for (i, k) in kids.iter().enumerate() {
        walk(
            k,
            &child_prefix,
            i + 1 == kids.len(),
            false,
            nodes,
            needs,
            seen,
            out,
        );
    }
}

pub fn roadmap(r: &Roadmap) -> String {
    let mut out = String::from("Roadmap (from what you know, up to the goal):\n");
    if r.steps.is_empty() {
        out.push_str("  nothing to learn: you know the goal.\n");
    }
    for (i, s) in r.steps.iter().enumerate() {
        let _ = writeln!(out, "\n{:>2}. {}  [{}]", i + 1, s.name, s.id);
        let _ = writeln!(out, "    {}", s.statement);
        if !s.builds_on.is_empty() {
            let names: Vec<_> = s.builds_on.iter().map(|i| i.as_str()).collect();
            let _ = writeln!(out, "    builds on: {}", names.join(", "));
        }
        if !s.explanation.is_empty() {
            let _ = writeln!(out, "    {}", s.explanation);
        }
        if !s.check.is_empty() {
            let _ = writeln!(out, "    check: {}", s.check);
        }
    }
    out
}
