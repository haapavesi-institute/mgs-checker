//! `mgs conjecture7`: Quiver Quest conjecture #7, on one quiver or on every
//! small connected quiver with a multiple arrow in a scope (acyclic, open or
//! control). The open scope is the one place this command accepts oriented
//! cycles: none may pass through a multiple arrow.

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{Args, ValueEnum};
use mgs_core::conjecture7::{check_one, plan, run as run_item, Class, Row, Scope, Spec};
use mgs_core::{CheckOptions, SearchOptions};

use crate::batch::{parallel, render, write, Format};
use crate::read_quiver;

#[derive(Args)]
pub struct Conjecture7Args {
    /// Check just this quiver (CSV file; `-` for standard input) instead of a survey.
    file: Option<PathBuf>,
    /// Check just this quiver, given as arrows, e.g. "1->2*2, 1->3, 3->2, 2->4, 4->3".
    #[arg(long, conflicts_with = "file")]
    arrows: Option<String>,
    /// Survey quivers with up to this many vertices.
    #[arg(long, default_value_t = 4)]
    max_vertices: usize,
    /// And at least this many.
    #[arg(long, default_value_t = 2)]
    min_vertices: usize,
    /// The most parallel arrows between two vertices (at least 2).
    #[arg(long, default_value_t = 2)]
    max_multiplicity: i64,
    /// acyclic: Igusa–Zhou's theorem; open: an oriented cycle, none through a multiple arrow; control: a multiple arrow on an oriented cycle.
    #[arg(long, value_enum, default_value_t = WhichScope::Acyclic)]
    scope: WhichScope,
    /// Which quivers with a multiple arrow to check, within the scope.
    #[arg(long, value_enum, default_value_t = WhichClass::All)]
    class: WhichClass,
    /// Draw this many random matrices per vertex count instead of taking every one.
    #[arg(long)]
    sample: Option<u64>,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Longest run of empty lengths tolerated below the longest MGS. 0 is BDP's No Gap Conjecture.
    #[arg(long, default_value_t = 0)]
    max_gap: u32,
    #[arg(long, default_value_t = 80)]
    max_length: u32,
    /// State budget per quiver; lower than `mgs check`'s, since many quivers run at once.
    #[arg(long, default_value_t = 200_000)]
    max_states: usize,
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
enum WhichScope {
    Acyclic,
    Open,
    Control,
}

#[derive(Clone, Copy, ValueEnum)]
enum WhichClass {
    /// Every quiver with a multiple arrow.
    All,
    /// A multiple arrow on a cycle of the underlying graph that is not oriented.
    OnCycle,
    /// That, and no Igusa–Zhou cut.
    OnCycleNoCut,
}

const HEADER: [&str; 14] = [
    "id",
    "vertices",
    "arrows",
    "scope",
    "oriented cycle",
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

fn verdict(r: &Row) -> String {
    if r.verdict == "holds" && !r.proved {
        "holds if no gap".into()
    } else {
        r.verdict.clone()
    }
}

fn cells(r: &Row) -> Vec<String> {
    vec![
        r.id.to_string(),
        r.vertices.to_string(),
        r.arrows.clone(),
        r.scope.to_string(),
        r.oriented_cycle.clone(),
        r.kind.clone(),
        if r.on_cycle { "yes".into() } else { "no".into() },
        r.iz_cut.map_or("none".into(), |k| format!("k = {k}")),
        r.total.clone(),
        r.longest.map_or(String::new(), |l| l.to_string()),
        verdict(r),
        r.detail.clone(),
        r.labels_avoid_multiple.map_or(String::new(), |ok| if ok { "yes".into() } else { "NO".into() }),
        r.status.clone(),
    ]
}

pub fn run(a: &Conjecture7Args) -> Result<(), String> {
    let search = SearchOptions { max_gap: a.max_gap, max_length: a.max_length, max_states: a.max_states, prune: true };
    let opts = CheckOptions { search, ..CheckOptions::default() };
    if a.file.is_some() || a.arrows.is_some() {
        let q = read_quiver(a.file.as_deref(), a.arrows.as_deref())?;
        let row = check_one(&q, 0, &opts)?;
        let text = match a.format {
            Format::Json => serde_json::to_string_pretty(&row).map_err(|e| e.to_string())? + "\n",
            _ => one_text(&row),
        };
        return write(a.out.as_deref(), &text);
    }
    let scope = match a.scope {
        WhichScope::Acyclic => Scope::Acyclic,
        WhichScope::Open => Scope::Open,
        WhichScope::Control => Scope::Control,
    };
    let class = match a.class {
        WhichClass::All => Class::All,
        WhichClass::OnCycle => Class::OnCycle,
        WhichClass::OnCycleNoCut => Class::OnCycleNoCut,
    };
    let spec = Spec {
        min_vertices: a.min_vertices,
        max_vertices: a.max_vertices,
        max_multiplicity: a.max_multiplicity,
        scope,
        class,
        sample: a.sample,
        seed: a.seed,
    };
    let items = plan(&spec)?;
    eprintln!("mgs: conjecture #7 on {} connected quivers with a multiple arrow, scope {}", items.len(), scope.name());
    let rows: Vec<Row> = parallel(&items, a.jobs, |_, item| run_item(item, &opts).expect("planned quivers have a multiple arrow"));
    eprint!("{}", summary(&rows));
    write(a.out.as_deref(), &render(&rows, &HEADER, cells, a.format)?)
}

fn one_text(r: &Row) -> String {
    let cycle = if r.oriented_cycle.is_empty() { "none".to_string() } else { r.oriented_cycle.clone() };
    let mut s = format!("Quiver     {} vertices: {}\nType       {}; oriented cycle: {cycle}\n", r.vertices, r.arrows, r.kind);
    let scope = match r.scope {
        "acyclic" => "acyclic: Igusa–Zhou's Theorem 3.12 covers it",
        "open" => "open: an oriented cycle, none through a multiple arrow",
        _ => "control: a multiple arrow lies on an oriented cycle, so conjecture #7 claims nothing",
    };
    s += &format!("Scope      {scope}\nStatus     {}\nMGS        {}\n", r.status, r.total);
    s += &format!("Q′         {}\nConj. #7   {}", r.q_prime, verdict(r));
    if !r.detail.is_empty() {
        s += &format!(": {}", r.detail);
    }
    s += "\n";
    if r.labels_avoid_multiple == Some(false) {
        s += "           some step c-vector has both ends of a multiple arrow in its support\n";
    }
    s
}

/// Verdict counts by vertex count, then the counterexamples.
fn summary(rows: &[Row]) -> String {
    let mut by_n: BTreeMap<usize, BTreeMap<String, usize>> = BTreeMap::new();
    for r in rows {
        *by_n.entry(r.vertices).or_default().entry(verdict(r)).or_default() += 1;
    }
    let mut s = String::new();
    for (n, counts) in &by_n {
        let total: usize = counts.values().sum();
        let parts: Vec<String> = counts.iter().map(|(v, c)| format!("{c} {v}")).collect();
        s += &format!("mgs: {n} vertices: {total} quivers: {}\n", parts.join(", "));
    }
    let fails: Vec<&Row> = rows.iter().filter(|r| r.counterexample()).collect();
    if rows.iter().any(|r| r.scope == "control") {
        s += "mgs: these are controls (a multiple arrow on an oriented cycle): failures are expected, not counterexamples\n";
    } else if fails.is_empty() {
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
