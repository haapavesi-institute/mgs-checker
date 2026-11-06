//! Follow-ups to `cycle_cut`:
//! 1. acyclic Q, Γ with a multiple edge: does C = (the multiple edges of Γ) always work?
//! 2. Ã(n−1,1), the n-cycle with one arrow against the rest: does one edge always suffice, and which?
//! 3. which acyclic quivers fail for a triangle Γ of single edges?
//!
//!     cargo run --release -p mgs-core --example cycle_cut_detail

use std::collections::{BTreeMap, HashSet};

use mgs_core::canonical::canonical_key;
use mgs_core::classify::{components, is_acyclic};
use mgs_core::engine::{search, Stop};
use mgs_core::framed::Framed;
use mgs_core::{Exchange, SearchOptions};

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

fn all_mgs(q: &Exchange, cap: u128) -> Option<Vec<Vec<usize>>> {
    let (engine, out) = search(q, &SearchOptions { max_gap: 1, max_length: 40, max_states: 4_000_000, prune: true });
    let root = out.root.as_ref()?;
    let fits = root.counts.total_within(out.searched_to).to_u128().is_some_and(|t| t <= cap);
    (fits && matches!(out.stop, Stop::Exhausted | Stop::Gap)).then(|| engine.sample(&Framed::new(q), out.searched_to, usize::MAX))
}

fn cut(q: &Exchange, edges: &[(usize, usize)]) -> Exchange {
    let mut c = q.clone();
    for &(i, j) in edges {
        c.set_pair(i, j, 0);
    }
    c
}

/// Triangles of the underlying graph whose arrows are not an oriented 3-cycle.
fn triangles(q: &Exchange) -> Vec<[(usize, usize); 3]> {
    let n = q.n();
    let mut out = Vec::new();
    for a in 0..n {
        for b in a + 1..n {
            for c in b + 1..n {
                if q.get(a, b) != 0 && q.get(b, c) != 0 && q.get(a, c) != 0 {
                    let oriented = (q.get(a, b) > 0) == (q.get(b, c) > 0) && (q.get(b, c) > 0) == (q.get(c, a) > 0);
                    if !oriented {
                        out.push([(a, b), (b, c), (a, c)]);
                    }
                }
            }
        }
    }
    out
}

fn main() {
    // 1 and 3, over all connected acyclic 4-vertex quivers with multiplicities ≤ 2 (triangles as Γ).
    let n = 4;
    let pairs: Vec<(usize, usize)> = (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j))).collect();
    let mut seen = HashSet::new();
    let (mut me_pairs, mut me_ok, mut single_fail) = (0, 0, Vec::new());
    for code in 0..5u64.pow(6) {
        let mut q = Exchange::zero(n);
        let mut c = code;
        for &(i, j) in &pairs {
            q.set_pair(i, j, (c % 5) as i64 - 2);
            c /= 5;
        }
        if components(&q).len() != 1 || !is_acyclic(&q) || !seen.insert(canonical_key(&q, false)) {
            continue;
        }
        let Some(all) = all_mgs(&q, 200_000) else {
            continue;
        };
        for tri in triangles(&q) {
            let multi: Vec<(usize, usize)> = tri.iter().copied().filter(|&(i, j)| q.get(i, j).abs() >= 2).collect();
            if multi.is_empty() {
                let survives = |s: &Vec<usize>| {
                    (1u32..8).any(|m| {
                        is_mgs(&cut(&q, &tri.iter().enumerate().filter(|(b, _)| m >> b & 1 == 1).map(|(_, &e)| e).collect::<Vec<_>>()), s)
                    })
                };
                if let Some(bad) = all.iter().find(|s| !survives(s)) {
                    single_fail.push(format!("{}  Γ = {:?}  σ = {bad:?}", q.arrow_text(), tri.map(|(i, j)| (i + 1, j + 1))));
                }
            } else {
                me_pairs += 1;
                let reduced = cut(&q, &multi);
                me_ok += usize::from(all.iter().all(|s| is_mgs(&reduced, s)));
            }
        }
    }
    println!("1. acyclic, triangle Γ with a multiple edge: C = Γ's multiple edges works for every MGS in {me_ok} of {me_pairs} pairs");
    println!("3. acyclic, triangle Γ of single edges, failures ({}):", single_fail.len());
    for f in &single_fail {
        println!("   {f}");
    }
    // 2. Ã(n−1,1): the path 1 → ⋯ → n with 1 → n.
    for n in 3..=6 {
        let mut text: Vec<String> = (1..n).map(|i| format!("{i}->{}", i + 1)).collect();
        text.push(format!("1->{n}"));
        let q = Exchange::from_arrow_text(&text.join(", "), None).unwrap();
        let Some(all) = all_mgs(&q, 3_000_000) else {
            continue;
        };
        let edges: Vec<(usize, usize)> = q.arrows().into_iter().map(|(i, j, _)| (i.min(j), i.max(j))).collect();
        let mut which: BTreeMap<String, usize> = BTreeMap::new();
        let mut none = 0;
        for s in &all {
            let good: Vec<String> =
                edges.iter().filter(|&&e| is_mgs(&cut(&q, &[e]), s)).map(|&(i, j)| format!("{}-{}", i + 1, j + 1)).collect();
            if good.is_empty() {
                none += 1;
            }
            for g in good {
                *which.entry(g).or_insert(0) += 1;
            }
        }
        println!(
            "2. Ã({},1), {} MGS: {} need more than one edge cut; MGS surviving the cut of each single edge: {which:?}",
            n - 1,
            all.len(),
            none
        );
    }
}
