//! `mgs explore`: the fixed explorations around tame quivers, run in
//! parallel, written as a Markdown table, CSV or JSON.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use clap::{Args, ValueEnum};
use mgs_core::explore::{plan, run as run_item, ExploreSpec, Row};
use mgs_core::families::{Family, Orientations};
use mgs_core::modify::Mods;
use mgs_core::CheckOptions;

use crate::SearchArgs;

#[derive(Args)]
pub struct ExploreArgs {
    /// Include the Kronecker quiver 1 ⇉ 2 (Ã1).
    #[arg(long)]
    kronecker: bool,
    /// Ã ranks, e.g. 2-4 or 2,3 (Ã_n has n + 1 vertices).
    #[arg(long, value_parser = ranks)]
    atilde: Option<Ranks>,
    /// D̃ ranks, e.g. 4-5 (D̃_n has n + 1 vertices, n ≥ 4).
    #[arg(long, value_parser = ranks)]
    dtilde: Option<Ranks>,
    /// Ẽ ranks: any of 6, 7, 8.
    #[arg(long, value_parser = ranks)]
    etilde: Option<Ranks>,
    /// Every acyclic orientation (up to relabelling), or one standard one.
    #[arg(long, value_enum, default_value_t = Which::All)]
    orientations: Which,
    /// Which edge changes to make, comma-separated.
    #[arg(long, value_enum, value_delimiter = ',', default_values_t = [Change::Multiplicity, Change::NewEdge])]
    mods: Vec<Change>,
    /// How many changes to apply in a row (0, 1 or 2).
    #[arg(long, default_value_t = 1)]
    steps: u32,
    /// Leave the tame starting quivers out of the table.
    #[arg(long)]
    no_base: bool,
    /// Treat a quiver and its opposite as one (same counts by length, BDP Prop. 2.15).
    #[arg(long)]
    up_to_opposite: bool,
    #[command(flatten)]
    search: SearchArgs,
    /// Quivers checked at once; each uses up to --max-states of memory.
    #[arg(long, default_value_t = 1)]
    jobs: usize,
    #[arg(long, value_enum, default_value_t = Format::Markdown)]
    format: Format,
    /// Write here instead of standard output.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Which {
    All,
    One,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Change {
    Multiplicity,
    NewEdge,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Markdown,
    Csv,
    Json,
}

/// Ranks given as "2-4" or "2,3,5".
#[derive(Clone, Debug)]
pub struct Ranks(Vec<usize>);

fn ranks(s: &str) -> Result<Ranks, String> {
    let mut out = Vec::new();
    for part in s.split(',') {
        let bad = || format!("{part:?} is not a rank or range like 2-4");
        match part.split_once('-') {
            Some((a, b)) => out.extend(a.trim().parse::<usize>().map_err(|_| bad())?..=b.trim().parse::<usize>().map_err(|_| bad())?),
            None => out.push(part.trim().parse().map_err(|_| bad())?),
        }
    }
    Ok(Ranks(out))
}

fn spec(a: &ExploreArgs) -> ExploreSpec {
    let mut families = Vec::new();
    if a.kronecker {
        families.push(Family::Kronecker);
    }
    families.extend(a.atilde.iter().flat_map(|r| &r.0).map(|&rank| Family::ATilde { rank }));
    families.extend(a.dtilde.iter().flat_map(|r| &r.0).map(|&rank| Family::DTilde { rank }));
    families.extend(a.etilde.iter().flat_map(|r| &r.0).map(|&rank| Family::ETilde { rank }));
    ExploreSpec {
        families,
        orientations: match a.orientations {
            Which::All => Orientations::All,
            Which::One => Orientations::One,
        },
        include_base: !a.no_base,
        mods: Mods { multiplicity: a.mods.contains(&Change::Multiplicity), new_edge: a.mods.contains(&Change::NewEdge) },
        steps: a.steps,
        up_to_opposite: a.up_to_opposite,
    }
}

pub fn run(a: &ExploreArgs) -> Result<(), String> {
    let spec = spec(a);
    if spec.families.is_empty() {
        return Err("choose at least one family: --kronecker, --atilde, --dtilde or --etilde".into());
    }
    let items = plan(&spec)?;
    let opts = CheckOptions { search: a.search.options(), ..CheckOptions::default() };
    eprintln!("mgs: checking {} quivers", items.len());
    let next = AtomicUsize::new(0);
    let rows: Mutex<Vec<Option<Row>>> = Mutex::new(vec![None; items.len()]);
    std::thread::scope(|scope| {
        for _ in 0..a.jobs.max(1) {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(i) else { break };
                let row = run_item(item, &opts).expect("planned quivers are valid");
                eprintln!("mgs: [{}/{}] {} {} — {} MGS", i + 1, items.len(), item.base, item.change, row.total);
                rows.lock().expect("no panics while holding the lock")[i] = Some(row);
            });
        }
    });
    let rows: Vec<Row> = rows.into_inner().expect("threads finished").into_iter().map(|r| r.expect("every item ran")).collect();
    let text = match a.format {
        Format::Markdown => markdown(&rows),
        Format::Csv => csv(&rows),
        Format::Json => serde_json::to_string_pretty(&rows).map_err(|e| e.to_string())? + "\n",
    };
    match &a.out {
        Some(path) => std::fs::write(path, text).map_err(|e| format!("writing {}: {e}", path.display())),
        None => {
            print!("{text}");
            Ok(())
        }
    }
}

const HEADER: [&str; 18] = [
    "id",
    "parent",
    "base",
    "change",
    "arrows",
    "type",
    "settled",
    "proved",
    "total",
    "shortest",
    "longest",
    "lit. longest",
    "by length",
    "classes on MGS",
    "step c-vectors",
    "maximal c-vectors",
    "largest c entry",
    "most arrows",
];

fn cells(r: &Row) -> [String; 18] {
    let opt = |x: Option<u32>| x.map_or(String::new(), |v| v.to_string());
    let num = |x: Option<i64>| x.map_or(String::new(), |v| v.to_string());
    [
        r.id.to_string(),
        r.parent.map_or(String::new(), |p| p.to_string()),
        r.base.clone(),
        r.change.clone(),
        r.arrows.clone(),
        r.kind.clone(),
        if r.settled { "yes".into() } else { format!("no (≤{})", r.searched_to) },
        if r.proved { "yes".into() } else { "no".into() },
        r.total.clone(),
        opt(r.shortest),
        opt(r.longest),
        opt(r.predicted_longest),
        r.by_length.clone(),
        r.classes_on_mgs.map_or(String::new(), |v| v.to_string()),
        r.step_c_vectors.map_or(String::new(), |v| v.to_string()),
        r.maximal_c.clone(),
        num(r.largest_c_entry),
        num(r.most_arrows),
    ]
}

fn markdown(rows: &[Row]) -> String {
    let mut s = format!("| {} | banned first moves | status |\n|{}\n", HEADER.join(" | "), "---|".repeat(HEADER.len() + 2));
    for r in rows {
        s += &format!("| {} | {} | {} |\n", cells(r).join(" | "), r.banned_first, r.status);
    }
    s
}

fn csv(rows: &[Row]) -> String {
    let quote = |c: &str| {
        if c.contains([',', '"', '\n']) {
            format!("\"{}\"", c.replace('"', "\"\""))
        } else {
            c.to_string()
        }
    };
    let mut s = HEADER.iter().map(|h| quote(h)).collect::<Vec<_>>().join(",") + ",banned first moves,status\n";
    for r in rows {
        let mut line: Vec<String> = cells(r).iter().map(|c| quote(c)).collect();
        line.push(quote(&r.banned_first));
        line.push(quote(&r.status));
        s += &(line.join(",") + "\n");
    }
    s
}
