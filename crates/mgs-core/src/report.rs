//! A complete check of one acyclic quiver: the search, what it proves and on
//! what assumption, the literature cross-checks, what the MGS pass through
//! (step c-vectors and their maximal elements), a sample of MGS and the
//! banned prefixes.

use serde::{Deserialize, Serialize};

use crate::acyclic::{admit, OrientedCycle};
use crate::cutting::CutCheck;
use crate::engine::{search, SearchOptions, Stop};
use crate::framed::Framed;
use crate::known::{facts, Facts};
use crate::matrix::Exchange;
use crate::seeds::Seeds;
use crate::walk::{BanOptions, BanReason, Banned, Bound};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CheckOptions {
    #[serde(flatten)]
    pub search: SearchOptions,
    /// How many MGS to list.
    pub sample: usize,
    /// List banned prefixes up to this length (0: none).
    pub banned_depth: usize,
    pub banned_limit: usize,
    /// Length of the green continuation shown for each banned prefix.
    pub tail: usize,
    /// State budget for trying to prove each tentative ban outright (0: don't try).
    pub prove_states: usize,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self { search: SearchOptions::default(), sample: 20, banned_depth: 3, banned_limit: 200, tail: 12, prove_states: 20_000 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Status {
    /// Every green sequence ended (all red, or broken by BHIT 3.3.2): exact, no assumption.
    Exhausted,
    /// Stopped at an empty length after the longest MGS, and a No Gap theorem covers this quiver: exact.
    NoGapTheorem { source: String },
    /// Stopped after `max_gap + 1` empty lengths: exact if the lengths of MGS have no gap longer than `max_gap`.
    Conditional { max_gap: u32 },
    /// No MGS of length ≤ the cap. Says nothing about longer ones.
    NoneFound,
    /// Still finding MGS near the length cap.
    LengthCap,
    /// The state budget ran out; counts are exact only up to `searched_to`.
    Budget,
    /// A matrix entry outgrew 64 bits below some state; counts are lower bounds.
    Overflow,
    /// The gap assumption is false here: a prefix the search thought dead finishes as an MGS of this length.
    AssumptionFails { max_gap: u32, length: u32 },
}

impl Status {
    /// Whether the counts are the full answer (possibly on an assumption).
    pub fn settled(&self) -> bool {
        matches!(self, Self::Exhausted | Self::NoGapTheorem { .. } | Self::Conditional { .. })
    }

    /// Settled with no unproved assumption.
    pub fn proved(&self) -> bool {
        matches!(self, Self::Exhausted | Self::NoGapTheorem { .. })
    }

    pub fn explain(&self, searched_to: u32) -> String {
        match self {
            Self::Exhausted => {
                format!("exact: every green sequence ended (all red, or at a step BHIT 3.3.2 rules out) by length {searched_to}")
            }
            Self::NoGapTheorem { source } => format!("exact: no MGS of length {searched_to}, so none longer, by {source}"),
            Self::Conditional { max_gap } => {
                let rule = if *max_gap == 0 {
                    "BDP's No Gap Conjecture (2.22)".to_string()
                } else {
                    format!("no gap longer than {max_gap} in the lengths of MGS")
                };
                let empty = if *max_gap == 0 {
                    format!("length {searched_to}")
                } else {
                    format!("lengths {} to {searched_to}", searched_to - max_gap)
                };
                format!("exact if {rule} holds here: no MGS of {empty}; every green sequence still going is tentatively infinite")
            }
            Self::NoneFound => format!("no MGS of length ≤ {searched_to}; nothing is claimed about longer ones"),
            Self::LengthCap => format!("incomplete: MGS were still being found near the length cap {searched_to}"),
            Self::Budget => format!("incomplete: the state budget ran out; counts are exact for lengths ≤ {searched_to} only"),
            Self::Overflow => {
                format!("incomplete: matrix entries outgrew 64 bits on some branch; counts up to {searched_to} are lower bounds")
            }
            Self::AssumptionFails { max_gap, length } => {
                format!("the gap assumption (max gap {max_gap}) FAILS here: there is an MGS of length {length}, beyond {searched_to}; counts are lower bounds")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LengthCount {
    pub length: u32,
    /// Decimal, since counts can exceed what JSON numbers hold exactly.
    pub count: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Agreement {
    pub what: &'static str,
    pub expected: u32,
    pub found: Option<u32>,
    pub agrees: bool,
    pub source: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Report {
    pub vertices: usize,
    pub matrix: Vec<Vec<i64>>,
    pub arrows: String,
    pub facts: Facts,
    pub options: CheckOptions,
    pub status: Status,
    pub explanation: String,
    pub searched_to: u32,
    pub by_length: Vec<LengthCount>,
    pub total: String,
    pub shortest: Option<u32>,
    pub longest: Option<u32>,
    pub states: usize,
    pub agreements: Vec<Agreement>,
    /// What the MGS of length ≤ `searched_to` pass through; None if there are none.
    pub seeds: Option<Seeds>,
    /// Conjecture #7 on those MGS: are they MGS of Q with its multiple arrows deleted? None without a multiple arrow or an MGS.
    pub cut: Option<CutCheck>,
    pub sample: Vec<Vec<usize>>,
    /// Empty unless the status is settled.
    pub banned: Vec<Banned>,
}

/// Checks an acyclic quiver; a quiver with an oriented cycle is refused.
pub fn check(q: &Exchange, opts: &CheckOptions) -> Result<Report, OrientedCycle> {
    admit(q)?;
    let facts = facts(q);
    let (engine, outcome) = search(q, &opts.search);
    let status = match outcome.stop {
        Stop::Exhausted => Status::Exhausted,
        Stop::Gap => match facts.no_gap {
            Some(source) => Status::NoGapTheorem { source: source.into() },
            None => Status::Conditional { max_gap: opts.search.max_gap },
        },
        Stop::NoneFound => Status::NoneFound,
        Stop::LengthCap => Status::LengthCap,
        Stop::Budget => Status::Budget,
        Stop::Overflow => Status::Overflow,
    };
    let bound = outcome.searched_to;
    let root = outcome.root.as_ref();
    let counts = root.map(|e| e.counts.nonzero_within(bound)).unwrap_or_default();
    let total = root.map(|e| e.counts.total_within(bound)).unwrap_or_default();
    let range = root.and_then(|e| e.counts.range_within(bound));
    let (shortest, longest) = (range.map(|r| r.0), range.map(|r| r.1));
    let mut agreements = Vec::new();
    for (what, fact, found) in [("shortest", &facts.shortest, shortest), ("longest", &facts.longest, longest)] {
        if let (Some(f), true) = (fact, status.settled()) {
            agreements.push(Agreement { what, expected: f.value, found, agrees: found == Some(f.value), source: f.source });
        }
    }
    let start = Framed::new(q);
    let sample = if root.is_some() { engine.sample(&start, bound, opts.sample) } else { Vec::new() };
    let seeds = engine.seeds(&start, bound);
    let cut = engine.cut_check(q, bound, opts.search.max_states);
    let ban = BanOptions {
        depth: opts.banned_depth,
        limit: opts.banned_limit,
        tail: opts.tail,
        prove_states: opts.prove_states,
        prove_length: opts.search.max_length,
    };
    let banned = match &status {
        Status::Exhausted | Status::NoGapTheorem { .. } => engine.banned(&start, bound, Bound::Theorem, &ban),
        Status::Conditional { max_gap } => engine.banned(&start, bound, Bound::Assumed { max_gap: *max_gap }, &ban),
        _ => Vec::new(),
    };
    let longer = banned.iter().filter_map(|b| if let BanReason::LongerMgs { length } = b.reason { Some(length) } else { None }).min();
    let status = match longer {
        Some(length) => Status::AssumptionFails { max_gap: opts.search.max_gap, length },
        None => status,
    };
    Ok(Report {
        vertices: q.n(),
        matrix: q.rows(),
        arrows: q.arrow_text(),
        facts,
        options: opts.clone(),
        explanation: status.explain(bound),
        status,
        searched_to: bound,
        by_length: counts.into_iter().map(|(length, c)| LengthCount { length, count: c.to_string() }).collect(),
        total: total.to_string(),
        shortest,
        longest,
        states: engine.states(),
        agreements,
        seeds,
        cut,
        sample,
        banned,
    })
}
