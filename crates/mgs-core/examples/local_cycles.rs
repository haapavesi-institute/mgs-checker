//! Survey of every connected acyclic quiver: which full subquivers that are
//! cycles can a step c-vector of a maximal green sequence cover?
//!
//!     cargo run --release -p mgs-core --example local_cycles -- N M [gap] [lift]
//!
//! Arguments: vertex count, largest multiplicity, max gap for the stopping
//! rule (default 0), and `lift` to also check the decomposition below.
//!
//! A *cycle* here is a vertex set C, |C| ≥ 3, whose full subquiver has a
//! cycle as underlying graph (so it has no chord). Its type is (p, q): p
//! arrows one way round and q the other, p ≥ q ≥ 1. A c-vector *covers* C if
//! its support contains C.
//!
//! What is tallied:
//! - multiple arrows i ⇉ j: a c-vector with i and j in its support (none, by
//!   Igusa–Zhou Theorem 3.12; a check on the engine);
//! - cycles by type: how many (quiver, cycle) pairs have a covering c-vector.
//!   The conjecture under test: none when q = 1;
//! - for q ≥ 2, what the covering c-vectors look like on C;
//! - sincere step c-vectors per quiver;
//! - with `lift`: S(Q) = Sinc(Q) ∪ ⋃_v S(Q ∖ v), where S is the set of step
//!   c-vectors and Sinc the sincere ones. "⊆" is restriction (Muller,
//!   DIRRT); "⊇" says every MGS c-vector of a full subquiver occurs in Q.

#![allow(clippy::type_complexity)]

use std::collections::{BTreeMap, BTreeSet};

use mgs_core::engine::{search, SearchOptions, Stop};
use mgs_core::enumerate::acyclic_quivers;
use mgs_core::framed::Framed;
use mgs_core::patterns::{cycles, Cycle};
use mgs_core::Exchange;

/// A covering c-vector as seen on the cycle: entries in walking order with
/// sources marked `s` and sinks `t`, least over rotations and reflections.
fn pattern(c: &Cycle, v: &[i64]) -> String {
    let len = c.order.len();
    let cell = |u: usize| {
        format!(
            "{}{}",
            v[u],
            if c.sources.contains(&u) {
                "s"
            } else if c.sinks.contains(&u) {
                "t"
            } else {
                ""
            }
        )
    };
    let mut best: Option<String> = None;
    for start in 0..len {
        for dir in [1, len - 1] {
            let s: Vec<String> = (0..len).map(|t| cell(c.order[(start + t * dir) % len])).collect();
            let s = s.join(" ");
            if best.as_ref().is_none_or(|b| s < *b) {
                best = Some(s);
            }
        }
    }
    best.unwrap()
}

fn sub(q: &Exchange, keep: &[usize]) -> Exchange {
    Exchange::from_rows(keep.iter().map(|&i| keep.iter().map(|&j| q.get(i, j)).collect()).collect()).unwrap()
}

/// The step c-vectors of q's MGS, and whether the search settled.
fn step_vectors(q: &Exchange, opts: &SearchOptions) -> (Vec<Vec<i64>>, bool) {
    let (engine, out) = search(q, opts);
    (engine.step_c_vectors(&Framed::new(q), out.searched_to), matches!(out.stop, Stop::Exhausted | Stop::Gap))
}

fn text(v: &[i64]) -> String {
    v.iter().map(i64::to_string).collect::<Vec<_>>().join("")
}

#[derive(Default)]
struct Tally {
    quivers: usize,
    unsettled: usize,
    me_pairs: usize,
    me_covered: usize,
    /// (length, p, q, sources on the cycle, multiple arrow on it) → (pairs, pairs with a covering c-vector)
    by_type: BTreeMap<(usize, usize, usize, usize, bool), (usize, usize)>,
    /// (length, p, q, sources) → patterns of covering c-vectors on the cycle, with counts
    patterns: BTreeMap<(usize, usize, usize, usize), BTreeMap<String, usize>>,
    /// Quivers with a cycle, all of whose cycles have q = 1: (count, with a sincere c-vector)
    all_p1: (usize, usize),
    /// Other quivers with a cycle: sincere c-vector counts → quivers
    sincere_counts: BTreeMap<usize, usize>,
    largest_sincere_entry: i64,
    lift_checked: usize,
    lift_missing: usize,
    restrict_extra: usize,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let n: usize = args.first().map_or(4, |a| a.parse().expect("vertex count"));
    let m: i64 = args.get(1).map_or(1, |a| a.parse().expect("multiplicity"));
    let gap: u32 = args.get(2).map_or(0, |a| a.parse().expect("gap"));
    let lift = args.iter().any(|a| a == "lift");
    let opts = SearchOptions { max_gap: gap, max_length: 120, max_states: 3_000_000, prune: true };
    let quivers = acyclic_quivers(n, m, |_| true).unwrap();
    let mut t = Tally { quivers: quivers.len(), ..Tally::default() };
    for q in &quivers {
        let (vectors, settled) = step_vectors(q, &opts);
        if !settled {
            t.unsettled += 1;
            println!("UNSETTLED {}", q.arrow_text());
            continue;
        }
        let covers = |c: &[usize]| vectors.iter().filter(|v| c.iter().all(|&u| v[u] > 0)).collect::<Vec<_>>();
        for (i, j, _) in q.arrows().into_iter().filter(|&(_, _, k)| k >= 2) {
            t.me_pairs += 1;
            if !covers(&[i, j]).is_empty() {
                t.me_covered += 1;
                println!("ME COVERED {} at {}⇉{}", q.arrow_text(), i + 1, j + 1);
            }
        }
        let cs = cycles(q);
        for c in &cs {
            let hit = covers(&c.order);
            let e = t.by_type.entry((c.order.len(), c.p, c.q, c.sources.len(), c.multi)).or_default();
            e.0 += 1;
            if hit.is_empty() {
                continue;
            }
            e.1 += 1;
            if c.q == 1 || c.multi {
                println!(
                    "EXCEPTION {}: cycle {:?} of type ({}, {}){} is covered by {}",
                    q.arrow_text(),
                    c.order.iter().map(|v| v + 1).collect::<Vec<_>>(),
                    c.p,
                    c.q,
                    if c.multi { " with a multiple arrow" } else { "" },
                    hit.iter().map(|v| text(v)).collect::<Vec<_>>().join(" ")
                );
            }
            let pats = t.patterns.entry((c.order.len(), c.p, c.q, c.sources.len())).or_default();
            for v in BTreeSet::from_iter(hit.iter().map(|v| pattern(c, v))) {
                *pats.entry(v).or_default() += 1;
            }
        }
        let sincere: Vec<&Vec<i64>> = vectors.iter().filter(|v| v.iter().all(|&x| x > 0)).collect();
        if !cs.is_empty() {
            if cs.iter().all(|c| c.q == 1 || c.multi) {
                t.all_p1.0 += 1;
                t.all_p1.1 += usize::from(!sincere.is_empty());
            } else {
                *t.sincere_counts.entry(sincere.len()).or_default() += 1;
                t.largest_sincere_entry = t.largest_sincere_entry.max(sincere.iter().flat_map(|v| v.iter().copied()).max().unwrap_or(0));
            }
        }
        if lift {
            // S(Q) against Sinc(Q) ∪ ⋃_v S(Q ∖ v), each padded with a zero at v.
            let mut below: BTreeSet<Vec<i64>> = BTreeSet::new();
            let mut ok = true;
            for v in 0..n {
                let keep: Vec<usize> = (0..n).filter(|&u| u != v).collect();
                let (part, fine) = step_vectors(&sub(q, &keep), &opts);
                ok &= fine;
                for w in part {
                    let mut full = vec![0; n];
                    for (k, &u) in keep.iter().enumerate() {
                        full[u] = w[k];
                    }
                    below.insert(full);
                }
            }
            if ok {
                t.lift_checked += 1;
                let have: BTreeSet<Vec<i64>> = vectors.iter().filter(|v| v.contains(&0)).cloned().collect();
                let missing: Vec<&Vec<i64>> = below.difference(&have).collect();
                let extra: Vec<&Vec<i64>> = have.difference(&below).collect();
                if !missing.is_empty() {
                    t.lift_missing += 1;
                    println!(
                        "LIFT FAILS {}: c-vectors of some Q∖v absent from Q: {}",
                        q.arrow_text(),
                        missing.iter().map(|v| text(v)).collect::<Vec<_>>().join(" ")
                    );
                }
                if !extra.is_empty() {
                    t.restrict_extra += 1;
                    println!("RESTRICTION FAILS {}: {}", q.arrow_text(), extra.iter().map(|v| text(v)).collect::<Vec<_>>().join(" "));
                }
            }
        }
    }
    println!("== {} connected acyclic {n}-vertex quivers, multiplicities ≤ {m}, max gap {gap}; {} unsettled", t.quivers, t.unsettled);
    println!("multiple arrows: {} pairs, {} with a c-vector over both ends (Igusa–Zhou: 0)", t.me_pairs, t.me_covered);
    for ((len, p, q, sources, multi), (pairs, hit)) in &t.by_type {
        println!(
            "cycle of length {len}, type ({p}, {q}), {sources} source(s){}: {pairs} (quiver, cycle) pairs, {hit} covered",
            if *multi { ", multiple arrow on it" } else { "" }
        );
    }
    println!(
        "quivers with a cycle, all of type (p, 1) or carrying a multiple arrow: {}; with a sincere c-vector: {}",
        t.all_p1.0, t.all_p1.1
    );
    println!(
        "other quivers with a cycle, by number of sincere c-vectors: {:?}; largest sincere entry {}",
        t.sincere_counts, t.largest_sincere_entry
    );
    if lift {
        println!(
            "decomposition checked on {} quivers: lifting fails on {}, restriction fails on {}",
            t.lift_checked, t.lift_missing, t.restrict_extra
        );
    }
    for ((len, p, q, sources), pats) in &t.patterns {
        let mut list: Vec<(&String, &usize)> = pats.iter().collect();
        list.sort_by(|a, b| b.1.cmp(a.1));
        let shown: Vec<String> = list.iter().take(14).map(|(s, c)| format!("[{s}]×{c}")).collect();
        println!("covering patterns, length {len} type ({p}, {q}), {sources} source(s): {} distinct: {}", pats.len(), shown.join(" "));
    }
}
