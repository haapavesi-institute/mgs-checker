//! `mgs sincere`: a survey of sincere step c-vectors over a family of acyclic
//! quivers. It tests the inheritance conjecture (a quiver containing one with
//! no sincere c-vector has none), lists the minimal quivers with none, and
//! flags known patterns that a c-vector covers. See `mgs_core::sincere`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{Args, ValueEnum};
use mgs_core::sincere::{plan, run as run_item, Family, Row, Spec};
use mgs_core::SearchOptions;

use crate::batch::{parallel, render, write, Format};

#[derive(Args)]
pub struct SincereArgs {
    /// Survey quivers with up to this many vertices.
    #[arg(long, default_value_t = 5)]
    max_vertices: usize,
    /// And at least this many.
    #[arg(long, default_value_t = 2)]
    min_vertices: usize,
    /// The most parallel arrows between two vertices (cycles and tournaments have single arrows).
    #[arg(long, default_value_t = 1)]
    max_multiplicity: i64,
    #[arg(long, value_enum, default_value_t = Which::All)]
    family: Which,
    /// Longest run of empty lengths tolerated below the longest MGS. 0 is BDP's No Gap Conjecture.
    #[arg(long, default_value_t = 0)]
    max_gap: u32,
    #[arg(long, default_value_t = 200)]
    max_length: u32,
    /// State budget per search; each quiver runs one search per vertex and one for itself.
    #[arg(long, default_value_t = 1_000_000)]
    max_states: usize,
    /// Quivers surveyed at once.
    #[arg(long, default_value_t = 1)]
    jobs: usize,
    #[arg(long, value_enum, default_value_t = Format::Markdown)]
    format: Format,
    /// Write the table here instead of standard output. The summary always goes to standard error.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Which {
    /// Every connected acyclic quiver.
    All,
    Trees,
    /// One cycle in the underlying graph.
    OneCycle,
    /// Bare cycles, every orientation.
    Cycles,
    /// The transitive tournament.
    Tournament,
}

const HEADER: [&str; 11] =
    ["id", "vertices", "arrows", "type", "MGS", "step c-vectors", "sincere", "patterns", "Q∖v without sincere", "finding", "settled"];

fn vector(v: &[i64]) -> String {
    format!("({})", v.iter().map(i64::to_string).collect::<Vec<_>>().join(","))
}

/// What the row shows, most important first.
fn finding(r: &Row) -> String {
    if !r.settled {
        "unsettled".into()
    } else if r.inheritance_fails {
        "INHERITANCE FAILS".into()
    } else if !r.covered.is_empty() {
        format!("COVERED: {}", r.covered.join("; "))
    } else if r.minimal && r.known_minimal {
        "minimal (a known pattern)".into()
    } else if r.minimal {
        "MINIMAL, NEW".into()
    } else {
        String::new()
    }
}

fn cells(r: &Row) -> Vec<String> {
    let shown: Vec<String> = r.sincere.iter().take(6).map(|v| vector(v)).collect();
    let more = if r.sincere.len() > 6 { format!(" … {} in all", r.sincere.len()) } else { String::new() };
    vec![
        r.id.to_string(),
        r.vertices.to_string(),
        r.arrows.clone(),
        r.kind.clone(),
        r.total.clone(),
        r.step_c_vectors.to_string(),
        if r.sincere.is_empty() { "none".into() } else { format!("{}{more}", shown.join(" ")) },
        r.patterns.join("; "),
        r.bare_below.iter().map(usize::to_string).collect::<Vec<_>>().join(" "),
        finding(r),
        if r.proved {
            "proved".into()
        } else if r.settled {
            "if no gap".into()
        } else {
            "no".into()
        },
    ]
}

pub fn run(a: &SincereArgs) -> Result<(), String> {
    let family = match a.family {
        Which::All => Family::All,
        Which::Trees => Family::Trees,
        Which::OneCycle => Family::OneCycle,
        Which::Cycles => Family::Cycles,
        Which::Tournament => Family::Tournament,
    };
    let spec = Spec { min_vertices: a.min_vertices, max_vertices: a.max_vertices, max_multiplicity: a.max_multiplicity, family };
    let items = plan(&spec)?;
    let opts = SearchOptions { max_gap: a.max_gap, max_length: a.max_length, max_states: a.max_states, prune: true };
    eprintln!("mgs: surveying sincere c-vectors of {} quivers", items.len());
    let rows: Vec<Row> = parallel(&items, a.jobs, |_, item| run_item(item, &opts).expect("planned quivers are valid"));
    eprint!("{}", summary(&rows));
    write(a.out.as_deref(), &render(&rows, &HEADER, cells, a.format)?)
}

/// Counts by vertex number, then every counterexample and every minimal quiver.
fn summary(rows: &[Row]) -> String {
    let mut by_n: BTreeMap<usize, [usize; 5]> = BTreeMap::new();
    for r in rows {
        let c = by_n.entry(r.vertices).or_default();
        c[0] += 1;
        c[1] += usize::from(!r.sincere.is_empty());
        c[2] += r.bare_below.len();
        c[3] += r.patterns.len();
        c[4] += usize::from(!r.settled);
    }
    let mut s = String::new();
    for (n, c) in &by_n {
        s += &format!(
            "mgs: {n} vertices: {} quivers, {} with a sincere c-vector; {} one-vertex extensions of a quiver with none checked; {} known patterns checked; {} unsettled\n",
            c[0], c[1], c[2], c[3], c[4]
        );
    }
    let fails: Vec<&Row> = rows.iter().filter(|r| r.inheritance_fails).collect();
    let covered: Vec<&Row> = rows.iter().filter(|r| r.settled && !r.covered.is_empty()).collect();
    s += &format!("mgs: inheritance fails on {} quivers; a known pattern is covered on {}\n", fails.len(), covered.len());
    for r in fails {
        s += &format!(
            "mgs: INHERITANCE FAILS {}: Q∖{:?} has no sincere c-vector, but Q has {}\n",
            r.arrows,
            r.bare_below,
            vector(&r.sincere[0])
        );
    }
    for r in covered {
        s += &format!("mgs: COVERED {}: {}\n", r.arrows, r.covered.join("; "));
    }
    for r in rows.iter().filter(|r| r.minimal) {
        let what = if r.known_minimal { "a known pattern" } else { "NEW" };
        s += &format!("mgs: minimal without a sincere c-vector ({what}): {} [{}]\n", r.arrows, r.kind);
    }
    s
}
