//! Survey for the cycle-cutting conjecture: for a cycle Γ of the underlying
//! graph whose arrows are not an oriented cycle, is every MGS σ of Q also an
//! MGS of Q ∖ C for some nonempty set C of Γ's edges? (Deleting an edge
//! deletes every arrow between its two vertices.)
//!
//!     cargo run --release -p mgs-core --example cycle_cut -- 4 2
//!
//! Arguments: vertex count, largest multiplicity. Also runs the tame cycles
//! Ã(a, b) on 4 and 5 vertices. Like the rest of this repository it looks
//! only at acyclic quivers.

use std::collections::HashSet;

use mgs_core::canonical::canonical_key;
use mgs_core::classify::{components, is_acyclic};
use mgs_core::engine::search;
use mgs_core::framed::Framed;
use mgs_core::{Exchange, SearchOptions};

/// Simple cycles of length ≥ 3 in the underlying graph, as edge lists, each once.
fn cycles(q: &Exchange) -> Vec<Vec<(usize, usize)>> {
    let n = q.n();
    let mut out = HashSet::new();
    fn dfs(q: &Exchange, start: usize, v: usize, path: &mut Vec<usize>, out: &mut HashSet<Vec<(usize, usize)>>) {
        for w in 0..q.n() {
            if q.get(v, w) == 0 {
                continue;
            }
            if w == start && path.len() >= 3 {
                let mut e: Vec<(usize, usize)> = path.windows(2).map(|p| (p[0].min(p[1]), p[0].max(p[1]))).collect();
                e.push((v.min(start), v.max(start)));
                e.sort_unstable();
                out.insert(e);
            } else if w > start && !path.contains(&w) {
                path.push(w);
                dfs(q, start, w, path, out);
                path.pop();
            }
        }
    }
    for s in 0..n {
        dfs(q, s, s, &mut vec![s], &mut out);
    }
    let mut v: Vec<_> = out.into_iter().collect();
    v.sort();
    v
}

fn oriented(q: &Exchange, cycle: &[(usize, usize)]) -> bool {
    // Oriented iff every vertex of the cycle has exactly one arrow out along it.
    let mut outs = std::collections::HashMap::new();
    for &(i, j) in cycle {
        let from = if q.get(i, j) > 0 { i } else { j };
        *outs.entry(from).or_insert(0) += 1;
    }
    outs.len() == cycle.len() && outs.values().all(|&c| c == 1)
}

/// σ (1-based) is a maximal green sequence of q.
fn is_mgs(q: &Exchange, sigma: &[usize]) -> bool {
    let mut f = Framed::new(q);
    for &v in sigma {
        if !f.is_green(v - 1) {
            return false;
        }
        let Ok(next) = f.mutate(v - 1) else {
            return false;
        };
        f = next;
    }
    f.all_red()
}

/// (quiver, cycle, failing MGS)
type Failure = (String, Vec<(usize, usize)>, Vec<usize>);

struct Tally {
    pairs: usize,
    pairs_ok: usize,
    mgs: usize,
    mgs_bad: usize,
    skipped: usize,
    bad_lengths: std::collections::BTreeMap<usize, usize>,
    /// (Γ has a multiple edge, length of Γ) → (pairs, pairs that satisfy it)
    by_class: std::collections::BTreeMap<(bool, usize), (usize, usize)>,
    worst: Option<Failure>,
}

fn survey(q: &Exchange, t: &mut Tally) {
    let (engine, outcome) = search(q, &SearchOptions { max_gap: 1, max_length: 40, max_states: 2_000_000, prune: true });
    let Some(root) = outcome.root.as_ref() else {
        return;
    };
    let too_many = root.counts.total_within(outcome.searched_to).to_u128().is_none_or(|t| t > 200_000);
    if too_many || !matches!(outcome.stop, mgs_core::engine::Stop::Exhausted | mgs_core::engine::Stop::Gap) {
        t.skipped += 1;
        return;
    }
    let all = engine.sample(&Framed::new(q), outcome.searched_to, usize::MAX);
    for cycle in cycles(q).into_iter().filter(|c| !oriented(q, c)) {
        t.pairs += 1;
        let mut ok = true;
        for sigma in &all {
            t.mgs += 1;
            let survives = (1u32..1 << cycle.len()).any(|mask| {
                let mut cut = q.clone();
                for (b, &(i, j)) in cycle.iter().enumerate() {
                    if mask >> b & 1 == 1 {
                        cut.set_pair(i, j, 0);
                    }
                }
                is_mgs(&cut, sigma)
            });
            if !survives {
                t.mgs_bad += 1;
                *t.bad_lengths.entry(sigma.len()).or_insert(0) += 1;
                ok = false;
                let shorter = t.worst.as_ref().is_none_or(|(_, _, w)| sigma.len() < w.len());
                if shorter {
                    t.worst = Some((q.arrow_text(), cycle.iter().map(|&(i, j)| (i + 1, j + 1)).collect(), sigma.clone()));
                }
            }
        }
        t.pairs_ok += usize::from(ok);
        let multiple = cycle.iter().any(|&(i, j)| q.get(i, j).abs() >= 2);
        let e = t.by_class.entry((multiple, cycle.len())).or_insert((0, 0));
        e.0 += 1;
        e.1 += usize::from(ok);
    }
}

fn report(label: &str, t: &Tally) {
    println!("{label}: {} of {} (quiver, cycle) pairs satisfy it; {} of {} (MGS, cycle) checks fail; {} quivers skipped (too many MGS or unsettled)", t.pairs_ok, t.pairs, t.mgs_bad, t.mgs, t.skipped);
    for ((multiple, len), (pairs, ok)) in &t.by_class {
        println!(
            "  Γ of length {len} {}: {ok} of {pairs} pairs satisfy it",
            if *multiple { "with a multiple edge" } else { "all single edges" }
        );
    }
    if !t.bad_lengths.is_empty() {
        println!("  failing MGS by length: {:?}", t.bad_lengths);
    }
    if let Some((q, c, s)) = &t.worst {
        println!("  shortest failing MGS: Q = {q}, Γ = {c:?}, σ = {s:?} (length {})", s.len());
    }
}

fn main() {
    let args: Vec<usize> = std::env::args().skip(1).map(|a| a.parse().expect("numbers")).collect();
    let (n, m) = (args.first().copied().unwrap_or(4), args.get(1).copied().unwrap_or(2) as i64);
    let pairs: Vec<(usize, usize)> = (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j))).collect();
    let mut seen = HashSet::new();
    let (mut t, mut quivers) = (
        Tally {
            pairs: 0,
            pairs_ok: 0,
            mgs: 0,
            mgs_bad: 0,
            skipped: 0,
            bad_lengths: Default::default(),
            by_class: Default::default(),
            worst: None,
        },
        0,
    );
    let base = (2 * m + 1) as u64;
    for code in 0..base.pow(pairs.len() as u32) {
        let mut q = Exchange::zero(n);
        let mut c = code;
        for &(i, j) in &pairs {
            q.set_pair(i, j, (c % base) as i64 - m);
            c /= base;
        }
        if components(&q).len() != 1
            || !is_acyclic(&q)
            || !seen.insert(canonical_key(&q, false))
            || !cycles(&q).iter().any(|cy| !oriented(&q, cy))
        {
            continue;
        }
        quivers += 1;
        survey(&q, &mut t);
    }
    report(&format!("all {quivers} connected acyclic {n}-vertex quivers (multiplicity ≤ {m}) with a cycle in the underlying graph"), &t);
    if n != 4 {
        return;
    }
    for text in [
        "1->2, 2->3, 1->4, 4->3",
        "1->2, 3->2, 3->4, 1->4",
        "1->2, 2->3, 3->4, 1->4",
        "1->2, 2->3, 3->4, 1->5, 5->4",
        "1->2, 2->3, 3->4, 4->5, 1->5",
        "1->2, 3->2, 3->4, 5->4, 1->5",
    ] {
        let mut t = Tally {
            pairs: 0,
            pairs_ok: 0,
            mgs: 0,
            mgs_bad: 0,
            skipped: 0,
            bad_lengths: Default::default(),
            by_class: Default::default(),
            worst: None,
        };
        survey(&Exchange::from_arrow_text(text, None).unwrap(), &mut t);
        report(text, &t);
    }
}
