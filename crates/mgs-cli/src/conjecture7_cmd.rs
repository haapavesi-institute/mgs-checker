//! `mgs conjecture7`: Quiver Quest conjecture #7 on every connected acyclic
//! quiver with a multiple arrow, up to a number of vertices.

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{Args, ValueEnum};
use mgs_core::conjecture7::{plan, run as run_item, Class, Row, Spec};
use mgs_core::CheckOptions;

use crate::batch::{parallel, render, write, Format};
use crate::SearchArgs;

#[derive(Args)]
pub struct Conjecture7Args {
    /// Check quivers with up to this many vertices.
    #[arg(long, default_value_t = 4)]
    max_vertices: usize,
    /// And at least this many.
    #[arg(long, default_value_t = 2)]
    min_vertices: usize,
    /// The most parallel arrows between two vertices (at least 2).
    #[arg(long, default_value_t = 2)]
    max_multiplicity: i64,
    /// Which quivers with a multiple arrow to check.
    #[arg(long, value_enum, default_value_t = Which::All)]
    class: Which,
    #[command(flatten)]
    search: SearchArgs,
    /// Quivers checked at once.
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
    /// Every quiver with a multiple arrow.
    All,
    /// A multiple arrow on a cycle of the underlying graph.
    OnCycle,
    /// That, and no Igusa–Zhou cut.
    OnCycleNoCut,
}

const HEADER: [&str; 12] = [
    "id",
    "vertices",
    "arrows",
    "type",
    "on a cycle",
    "IZ cut",
    "MGS",
    "longest",
    "verdict",
    "detail",
    "labels avoid multiple arrows",
    "status",
];

fn cells(r: &Row) -> Vec<String> {
    vec![
        r.id.to_string(),
        r.vertices.to_string(),
        r.arrows.clone(),
        r.kind.clone(),
        if r.on_cycle { "yes".into() } else { "no".into() },
        r.iz_cut.map_or("none".into(), |k| format!("k = {k}")),
        r.total.clone(),
        r.longest.map_or(String::new(), |l| l.to_string()),
        if r.verdict == "holds" && !r.proved { "holds if no gap".into() } else { r.verdict.clone() },
        r.detail.clone(),
        r.labels_avoid_multiple.map_or(String::new(), |ok| if ok { "yes".into() } else { "NO".into() }),
        r.status.clone(),
    ]
}

pub fn run(a: &Conjecture7Args) -> Result<(), String> {
    let class = match a.class {
        Which::All => Class::All,
        Which::OnCycle => Class::OnCycle,
        Which::OnCycleNoCut => Class::OnCycleNoCut,
    };
    let spec = Spec { min_vertices: a.min_vertices, max_vertices: a.max_vertices, max_multiplicity: a.max_multiplicity, class };
    let items = plan(&spec)?;
    let opts = CheckOptions { search: a.search.options(), ..CheckOptions::default() };
    eprintln!("mgs: conjecture #7 on {} connected acyclic quivers with a multiple arrow", items.len());
    let rows: Vec<Row> = parallel(&items, a.jobs, |_, item| run_item(item, &opts).expect("planned quivers are acyclic"));
    eprint!("{}", summary(&rows));
    write(a.out.as_deref(), &render(&rows, &HEADER, cells, a.format)?)
}

/// Verdict counts by vertex count, then the failures.
fn summary(rows: &[Row]) -> String {
    let mut by_n: BTreeMap<usize, BTreeMap<String, usize>> = BTreeMap::new();
    for r in rows {
        let v = if r.verdict == "holds" && !r.proved { "holds if no gap".to_string() } else { r.verdict.clone() };
        *by_n.entry(r.vertices).or_default().entry(v).or_default() += 1;
    }
    let mut s = String::new();
    for (n, counts) in &by_n {
        let total: usize = counts.values().sum();
        let parts: Vec<String> = counts.iter().map(|(v, c)| format!("{c} {v}")).collect();
        s += &format!("mgs: {n} vertices: {total} quivers: {}\n", parts.join(", "));
    }
    let fails: Vec<&Row> = rows.iter().filter(|r| r.verdict == "fails").collect();
    if fails.is_empty() {
        s += "mgs: no counterexample\n";
    }
    for r in fails {
        s += &format!("mgs: COUNTEREXAMPLE {} — {}\n", r.arrows, r.detail);
    }
    let covers = rows.iter().filter(|r| r.labels_avoid_multiple == Some(false)).count();
    if covers > 0 {
        s += &format!("mgs: on {covers} quivers some step c-vector has both ends of a multiple arrow in its support\n");
    }
    s
}
