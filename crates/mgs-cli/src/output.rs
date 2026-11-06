//! Plain-text rendering of reports for the terminal.

use std::fmt::Write;
use std::time::Duration;

use mgs_core::verify::{Verdict, Verification};
use mgs_core::walk::{BanReason, TailEnd};
use mgs_core::{Exchange, Report};

/// Maximal c-vectors printed before "… and N more".
const MAXIMAL_SHOWN: usize = 40;

fn seq(v: &[usize]) -> String {
    format!("({})", v.iter().map(usize::to_string).collect::<Vec<_>>().join(", "))
}

fn vector(v: &[i64]) -> String {
    format!("({})", v.iter().map(i64::to_string).collect::<Vec<_>>().join(", "))
}

pub fn report_text(r: &Report, took: Duration) -> String {
    let mut s = String::new();
    let o = &r.options.search;
    let _ = writeln!(s, "Quiver     {} vertices: {}", r.vertices, r.arrows);
    let _ = writeln!(s, "Type       {}", r.facts.kind_name);
    let _ = writeln!(
        s,
        "Search     lengths ≤ {}, {} classes, {:.2?}; max gap {}, BHIT 3.3.2 pruning {}",
        r.searched_to,
        r.states,
        took,
        o.max_gap,
        if o.prune { "on" } else { "off" }
    );
    let _ = writeln!(s, "Status     {}", r.explanation);
    if r.by_length.is_empty() {
        let none = if r.status.proved() { "none: this quiver has no maximal green sequence" } else { "none found" };
        let _ = writeln!(s, "MGS        {none}");
    } else {
        let _ = writeln!(s, "MGS        {} in all; shortest {}, longest {}", r.total, r.shortest.unwrap_or(0), r.longest.unwrap_or(0));
        let width = r.by_length.iter().map(|c| c.count.len()).max().unwrap_or(1);
        for c in &r.by_length {
            let _ = writeln!(s, "  length {:>3}: {:>width$}", c.length, c.count);
        }
    }
    for a in &r.agreements {
        let verdict = if a.agrees { "agrees" } else { "DISAGREES" };
        let found = a.found.map_or("none".to_string(), |v| v.to_string());
        let _ = writeln!(s, "Literature {} MGS {}, by {}: found {found}, {verdict}", a.what, a.expected, a.source);
    }
    if let Some(sd) = &r.seeds {
        let _ = writeln!(s, "Seeds      the MGS pass through {} classes; the most parallel arrows met is {}", sd.classes, sd.most_arrows);
        let _ = writeln!(
            s,
            "C-vectors  {} distinct at the steps of the MGS, largest entry {}; maximal under the entrywise order ({}):",
            sd.step_c_vectors,
            sd.largest_entry,
            sd.maximal.len()
        );
        let shown: Vec<(String, &Vec<usize>)> = sd.maximal.iter().take(MAXIMAL_SHOWN).map(|m| (vector(&m.vector), &m.prefix)).collect();
        let width = shown.iter().map(|(v, _)| v.chars().count()).max().unwrap_or(0);
        for (v, prefix) in shown {
            let _ = writeln!(s, "  {v:<width$}  at the last step of {}", seq(prefix));
        }
        if sd.maximal.len() > MAXIMAL_SHOWN {
            let _ = writeln!(s, "  … and {} more (all of them are in --json)", sd.maximal.len() - MAXIMAL_SHOWN);
        }
    }
    if !r.sample.is_empty() {
        let _ = writeln!(s, "Sample     {} of them:", r.sample.len());
        for m in &r.sample {
            let _ = writeln!(s, "  {}", seq(m));
        }
    }
    if !r.banned.is_empty() {
        let _ = writeln!(s, "Banned     prefixes that can never be finished (every shorter prefix can):");
        for b in &r.banned {
            let why = match &b.reason {
                BanReason::Bhit332 { vertex, from, arrows } => {
                    format!("proved: BHIT 3.3.2, vertex {vertex} is the target of {arrows} arrows from {from}")
                }
                BanReason::DeadEnd { within } => format!("proved: every continuation reaches a BHIT 3.3.2 step within {within} more steps"),
                BanReason::LongerMgs { length } => {
                    format!("NOT banned: it finishes as an MGS of length {length}, so the gap assumption fails")
                }
                BanReason::NoGapTheorem => "proved via the No Gap theorem: no continuation within the bound".into(),
                BanReason::Tentative { max_gap } => format!("tentative (max gap {max_gap}): empirically infinite"),
                BanReason::Unresolved => "unresolved: entries outgrew 64 bits".into(),
            };
            let tail = match b.tail_end {
                _ if b.tail.is_empty() => String::new(),
                TailEnd::StillGreen => format!("; runs on green: {}…", seq(&b.tail)),
                TailEnd::Dead => format!("; then {} and every green step breaks BHIT 3.3.2", seq(&b.tail)),
                TailEnd::AllRed => format!("; then {} ends all red", seq(&b.tail)),
                TailEnd::Overflow => format!("; then {} and entries outgrow 64 bits", seq(&b.tail)),
            };
            let _ = writeln!(s, "  {:<14} {why}{tail}", seq(&b.prefix));
        }
    }
    s
}

pub fn verification_text(q: &Exchange, v: &Verification) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "Quiver  {}", q.arrow_text());
    for (t, step) in v.steps.iter().enumerate() {
        let colour = if step.green { "green" } else { "RED" };
        let ban = step.bhit.map_or(String::new(), |(from, m)| format!("  breaks BHIT 3.3.2 ({m} arrows from {from}), so no MGS does this"));
        let c = step.c_vector.iter().map(i64::to_string).collect::<Vec<_>>().join(", ");
        let _ = writeln!(s, "  step {:>2}: mutate {} ({colour}, c = ({c})){ban}", t + 1, step.vertex);
    }
    let verdict = match v.verdict {
        Verdict::Maximal => "a maximal green sequence".to_string(),
        Verdict::GreenNotMaximal => format!("green but not maximal: still green at the end: {}", seq(&v.green_at_end)),
        Verdict::NotGreen => "not a green sequence: some step mutates a red vertex".to_string(),
    };
    let _ = writeln!(s, "Verdict {verdict}");
    s
}
