//! `mgs`: check an acyclic quiver's maximal green sequences from its exchange
//! matrix, verify one sequence, or run the fixed explorations around tame
//! quivers. A quiver with an oriented cycle is refused.

mod batch;
mod conjecture7_cmd;
mod explore_cmd;
mod output;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use mgs_core::{check, csv::parse_csv, verify::verify, CheckOptions, Exchange, SearchOptions};

#[derive(Parser)]
#[command(name = "mgs", version, about = "Maximal green sequences of quivers, counted exactly by length")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Count every MGS of an acyclic quiver by length, list banned prefixes and the maximal c-vectors.
    Check {
        #[command(flatten)]
        input: Input,
        #[command(flatten)]
        search: SearchArgs,
        /// How many MGS to list.
        #[arg(long, default_value_t = 20)]
        sample: usize,
        /// List banned prefixes up to this length (0 for none).
        #[arg(long, default_value_t = 3)]
        banned_depth: usize,
        /// Length of the green continuation shown for each banned prefix.
        #[arg(long, default_value_t = 12)]
        tail: usize,
        /// Print the full report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Check one mutation sequence (1-based, comma-separated): green? maximal?
    Verify {
        #[command(flatten)]
        input: Input,
        /// For example 1,3,2,3.
        #[arg(long, value_delimiter = ',', required = true)]
        sequence: Vec<usize>,
        #[arg(long)]
        json: bool,
    },
    /// Check every quiver one or two edge changes away from chosen tame quivers.
    Explore(explore_cmd::ExploreArgs),
    /// Quiver Quest conjecture #7 on every connected acyclic quiver with a multiple
    /// arrow, up to --max-vertices: is each MGS an MGS of the quiver with its
    /// multiple arrows deleted, with the same c-vectors?
    Conjecture7(conjecture7_cmd::Conjecture7Args),
}

#[derive(Args)]
struct Input {
    /// CSV file with the exchange matrix (b_ij = #(i→j) − #(j→i)); `-` reads standard input.
    #[arg(required_unless_present = "arrows")]
    file: Option<PathBuf>,
    /// The quiver as arrows instead, e.g. "1->2*2, 2->3".
    #[arg(long, conflicts_with = "file")]
    arrows: Option<String>,
}

#[derive(Args, Clone)]
pub struct SearchArgs {
    /// Longest run of empty lengths tolerated below the longest MGS. 0 is BDP's No Gap Conjecture.
    #[arg(long, default_value_t = 0)]
    max_gap: u32,
    /// Never search past this length.
    #[arg(long, default_value_t = 80)]
    max_length: u32,
    /// Stop after storing this many exchange-graph classes (about 0.5–1 kB each).
    #[arg(long, default_value_t = 1_000_000)]
    max_states: usize,
    /// Do not skip the steps BHIT 3.3.2 rules out (slower; for checking the pruning).
    #[arg(long)]
    no_prune: bool,
}

impl SearchArgs {
    pub fn options(&self) -> SearchOptions {
        SearchOptions { max_gap: self.max_gap, max_length: self.max_length, max_states: self.max_states, prune: !self.no_prune }
    }
}

impl Input {
    fn read(&self) -> Result<Exchange, String> {
        if let Some(text) = &self.arrows {
            return Exchange::from_arrow_text(text, None).map_err(|e| e.to_string());
        }
        let path = self.file.as_ref().expect("clap requires a file or --arrows");
        let text = if path.as_os_str() == "-" {
            std::io::read_to_string(std::io::stdin()).map_err(|e| format!("reading standard input: {e}"))?
        } else {
            std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?
        };
        parse_csv(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mgs: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Check { input, search, sample, banned_depth, tail, json } => {
            let q = input.read()?;
            let opts = CheckOptions { search: search.options(), sample, banned_depth, tail, ..CheckOptions::default() };
            let started = std::time::Instant::now();
            let report = check(&q, &opts).map_err(|e| e.to_string())?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?);
            } else {
                print!("{}", output::report_text(&report, started.elapsed()));
            }
        }
        Command::Verify { input, sequence, json } => {
            let q = input.read()?;
            let v = verify(&q, &sequence)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?);
            } else {
                print!("{}", output::verification_text(&q, &v));
            }
        }
        Command::Explore(args) => explore_cmd::run(&args)?,
        Command::Conjecture7(args) => conjecture7_cmd::run(&args)?,
    }
    Ok(())
}
