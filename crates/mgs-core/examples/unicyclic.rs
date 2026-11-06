//! Survey of acyclic quivers whose underlying graph has exactly one cycle C:
//! an Ã subquiver with trees attached, multiple arrows allowed.
//!
//!     cargo run --release -p mgs-core --example unicyclic -- 5 1 [cap]
//!
//! Arguments: vertex count, largest multiplicity, and the most MGS to list
//! per quiver (default 300 000). For each quiver it prints:
//! - the search status and the number of MGS;
//! - the maximal step c-vectors, their δ_C-depth (the most copies of the
//!   cycle's null root δ_C below one: min of the entries on C), and how many
//!   cover C (support ⊇ C);
//! - whether the cycle is of type Ã(p, 1), and whether every Q ∖ v for v on
//!   C is a Dynkin forest.
//!
//! For quivers whose MGS are listed in full, it also checks four maps:
//! - restriction to C and to every Q ∖ v (keep the steps whose c-vector is
//!   supported there). This is the lattice quotient tors kQ → tors kQ/⟨e⟩
//!   (Muller; Demonet–Iyama–Reading–Reiten–Thomas), so every image should be
//!   an MGS;
//! - the product of the restrictions to Q ∖ v over v on C, and its fibres;
//! - for each arrow α of C, the cut map to Q ∖ α: delete the steps whose
//!   c-vector has both ends of α in its support. Is the image an MGS of
//!   Q ∖ α, and how big are the fibres?
//! - the full-support (sincere) step c-vectors, the only ones no Q ∖ v sees.
//!
//! A summary line at the end totals the exceptions.

use std::collections::HashMap;

use mgs_core::classify::{classify, Kind};
use mgs_core::engine::{search, SearchOptions, Stop};
use mgs_core::enumerate::acyclic_quivers;
use mgs_core::framed::Framed;
use mgs_core::Exchange;

/// The vertices on the unique cycle: strip leaves until none are left.
fn cycle(q: &Exchange) -> Vec<usize> {
    let n = q.n();
    let mut alive = vec![true; n];
    loop {
        let leaf = (0..n).find(|&v| alive[v] && (0..n).filter(|&w| alive[w] && q.get(v, w) != 0).count() <= 1);
        match leaf {
            Some(v) => alive[v] = false,
            None => return (0..n).filter(|&v| alive[v]).collect(),
        }
    }
}

fn edges(q: &Exchange) -> usize {
    let n = q.n();
    (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j))).filter(|&(i, j)| q.get(i, j) != 0).count()
}

/// (p, q) of the cycle's orientation: arrows one way round and the other.
fn cycle_type(q: &Exchange, c: &[usize]) -> (usize, usize) {
    // Walk the cycle: from c[0], repeatedly step to an unvisited cycle neighbour.
    let mut order = vec![c[0]];
    while order.len() < c.len() {
        let v = *order.last().unwrap();
        let next = c.iter().copied().find(|&w| !order.contains(&w) && q.get(v, w) != 0).unwrap();
        order.push(next);
    }
    let mut fwd = 0;
    for t in 0..order.len() {
        let (a, b) = (order[t], order[(t + 1) % order.len()]);
        if q.get(a, b) > 0 {
            fwd += 1;
        }
    }
    (fwd.max(order.len() - fwd), fwd.min(order.len() - fwd))
}

fn sub(q: &Exchange, vs: &[usize]) -> Exchange {
    Exchange::from_rows(vs.iter().map(|&i| vs.iter().map(|&j| q.get(i, j)).collect()).collect()).unwrap()
}

/// Step c-vectors of a sequence (0-based vertices).
fn steps(q: &Exchange, seq: &[usize]) -> Vec<Vec<i64>> {
    let mut f = Framed::new(q);
    let mut out = Vec::new();
    for &k in seq {
        out.push(f.c_vector(k).to_vec());
        f = f.mutate(k).unwrap();
    }
    out
}

/// Replays a sequence of c-vectors on q: at each step mutate the green vertex
/// with that c-vector. True if every step exists and the end is all red.
fn is_mgs_by_c_vectors(q: &Exchange, cs: &[Vec<i64>]) -> bool {
    let mut f = Framed::new(q);
    for c in cs {
        let Some(k) = (0..q.n()).find(|&k| f.c_vector(k) == c.as_slice()) else { return false };
        if !f.is_green(k) {
            return false;
        }
        f = f.mutate(k).unwrap();
    }
    f.all_red()
}

fn vec_text(v: &[i64]) -> String {
    v.iter().map(i64::to_string).collect::<Vec<_>>().join("")
}

/// The cycle's sources and sinks (along the cycle), and its two path lengths when it has one of each.
fn shape(q: &Exchange, c: &[usize]) -> (Vec<usize>, Vec<usize>, Option<(usize, usize)>) {
    let nb = |v: usize| c.iter().copied().filter(move |&w| q.get(v, w) != 0);
    let sources: Vec<usize> = c.iter().copied().filter(|&v| nb(v).all(|w| q.get(v, w) > 0)).collect();
    let sinks: Vec<usize> = c.iter().copied().filter(|&v| nb(v).all(|w| q.get(v, w) < 0)).collect();
    let paths = (sources.len() == 1 && sinks.len() == 1).then(|| {
        let (p, q2) = cycle_type(q, c);
        (p, q2)
    });
    (sources, sinks, paths)
}

/// Every connected component is Dynkin.
fn dynkin_forest(q: &Exchange) -> bool {
    fn ok(k: &Kind) -> bool {
        match k {
            Kind::Dynkin { .. } => true,
            Kind::Disconnected { parts } => parts.iter().all(ok),
            _ => false,
        }
    }
    ok(&classify(q))
}

#[derive(Default)]
struct Tally {
    /// Quivers whose Q ∖ v, for every v on the cycle, is a Dynkin forest.
    dynkin_attached: usize,
    dynkin_depth_over_1: usize,
    /// Cycles with one arrow against the rest (type Ã(p, 1)) that still have a step c-vector covering C.
    p1_cover: usize,
    p1: usize,
    p1_product_fibre: usize,
    other_product_fibre: usize,
    quivers: usize,
    listed: usize,
    depth_over_1: usize,
    bound_fails: usize,
    cover_mismatch: usize,
    restrict_bad: usize,
    cut_bad: usize,
    cut_not_onto: usize,
    max_fibre: usize,
    max_deleted: usize,
    incomplete: usize,
    full_support: usize,
    full_entry: i64,
}

fn main() {
    let args: Vec<u64> = std::env::args().skip(1).map(|a| a.parse().expect("numbers")).collect();
    let (n, m, cap) =
        (args.first().copied().unwrap_or(4) as usize, args.get(1).copied().unwrap_or(1) as i64, args.get(2).copied().unwrap_or(300_000));
    let opts = SearchOptions { max_gap: 0, max_length: 80, max_states: 2_000_000, prune: true };
    let quivers = acyclic_quivers(n, m, |q| edges(q) == q.n()).unwrap();
    println!("{} connected acyclic {n}-vertex quivers with one cycle, multiplicities ≤ {m}", quivers.len());
    let mut t = Tally { quivers: quivers.len(), ..Tally::default() };
    for q in quivers {
        let c = cycle(&q);
        let (sources, sinks, paths) = shape(&q, &c);
        let (engine, out) = search(&q, &opts);
        let status = match out.stop {
            Stop::Exhausted => "exhausted",
            Stop::Gap => "gap",
            _ => {
                t.incomplete += 1;
                "INCOMPLETE"
            }
        };
        let start = Framed::new(&q);
        let bound = out.searched_to;
        let total = out.root.as_ref().map_or(0, |r| r.counts.total_within(bound).to_u128().unwrap_or(u128::MAX));
        let seeds = engine.seeds(&start, bound);
        let maximal: Vec<Vec<i64>> = seeds.iter().flat_map(|s| s.maximal.iter().map(|m| m.vector.clone())).collect();
        // δ_C-depth: the most copies of δ_C under a step c-vector (monotone, so the maximal ones suffice).
        let depth = maximal.iter().map(|b| c.iter().map(|&v| b[v]).min().unwrap()).max().unwrap_or(0);
        let covers: Vec<&Vec<i64>> = maximal.iter().filter(|b| c.iter().all(|&v| b[v] > 0)).collect();
        // Predicted: one source and one sink on C, both paths of length ≥ 2, and no multiple arrow on C.
        let simple_cycle = c.iter().all(|&u| c.iter().all(|&v| q.get(u, v).abs() <= 1));
        let expected_cover = simple_cycle && paths.is_some_and(|(p, q2)| p >= 2 && q2 >= 2);
        // Bound: on C, a covering step c-vector is at most δ_C + e_s + e_t.
        let bound_ok = covers.iter().all(|b| {
            c.iter().all(|&v| b[v] <= 1 + i64::from(sources.contains(&v) || sinks.contains(&v)) && (b[v] <= 1 || sources.len() == 1))
        });
        let (pa, qa) = cycle_type(&q, &c);
        let attached_dynkin = c.iter().all(|&v| dynkin_forest(&sub(&q, &(0..n).filter(|&u| u != v).collect::<Vec<_>>())));
        t.dynkin_attached += usize::from(attached_dynkin);
        t.dynkin_depth_over_1 += usize::from(attached_dynkin && depth > 1);
        t.p1 += usize::from(qa == 1);
        t.p1_cover += usize::from(qa == 1 && !covers.is_empty());
        t.depth_over_1 += usize::from(depth > 1);
        t.bound_fails += usize::from(!bound_ok);
        t.cover_mismatch += usize::from(covers.is_empty() == expected_cover);
        let mut line = format!(
            "{:<40} C={} Ã({pa},{qa}) {} src {} snk {} {status:<10} MGS {total:>9} steps {:>4} depth {depth} cover {}{} | maximal {}",
            q.arrow_text(),
            c.len(),
            if attached_dynkin { "dyn" } else { "WILD-ATT" },
            sources.len(),
            sinks.len(),
            seeds.as_ref().map_or(0, |s| s.step_c_vectors),
            covers.len(),
            if covers.is_empty() == expected_cover { " MISMATCH" } else { "" },
            maximal.iter().map(|b| vec_text(b)).collect::<Vec<_>>().join(" ")
        );
        if total > 0 && total <= cap as u128 {
            t.listed += 1;
            let all: Vec<Vec<Vec<i64>>> =
                engine.sample(&start, bound, usize::MAX).iter().map(|s| steps(&q, &s.iter().map(|v| v - 1).collect::<Vec<_>>())).collect();
            // Restriction to the cycle: keep the steps supported on C.
            let qc = sub(&q, &c);
            let restrict_bad = all
                .iter()
                .filter(|cs| {
                    let kept: Vec<Vec<i64>> = cs
                        .iter()
                        .filter(|b| (0..n).all(|v| c.contains(&v) || b[v] == 0))
                        .map(|b| c.iter().map(|&v| b[v]).collect())
                        .collect();
                    !is_mgs_by_c_vectors(&qc, &kept)
                })
                .count();
            // And to every vertex-deleted subquiver Q ∖ v: keep the steps with c_v = 0.
            let mut vertex_bad = 0;
            for v in 0..n {
                let rest: Vec<usize> = (0..n).filter(|&u| u != v).collect();
                let qv = sub(&q, &rest);
                let mut seen = std::collections::HashSet::new();
                for cs in &all {
                    let kept: Vec<Vec<i64>> = cs.iter().filter(|b| b[v] == 0).map(|b| rest.iter().map(|&u| b[u]).collect()).collect();
                    if seen.insert(kept.clone()) && !is_mgs_by_c_vectors(&qv, &kept) {
                        vertex_bad += 1;
                    }
                }
            }
            t.restrict_bad += restrict_bad + vertex_bad;
            // The product of the restrictions to Q ∖ v over the cycle vertices v.
            let mut tuple: HashMap<Vec<Vec<Vec<i64>>>, usize> = HashMap::new();
            for cs in &all {
                let key: Vec<Vec<Vec<i64>>> = c.iter().map(|&v| cs.iter().filter(|b| b[v] == 0).cloned().collect()).collect();
                *tuple.entry(key).or_default() += 1;
            }
            let pfibre = tuple.values().max().copied().unwrap_or(0);
            if qa == 1 {
                t.p1_product_fibre = t.p1_product_fibre.max(pfibre);
            } else {
                t.other_product_fibre = t.other_product_fibre.max(pfibre);
            }
            line += &format!(" | product over C: fibre ≤{pfibre}");
            // Full-support step c-vectors: the only ones no Q ∖ v sees.
            let mut full: Vec<&Vec<i64>> = all.iter().flatten().filter(|b| b.iter().all(|&x| x > 0)).collect();
            full.sort();
            full.dedup();
            t.full_support = t.full_support.max(full.len());
            t.full_entry = t.full_entry.max(full.iter().flat_map(|b| b.iter().copied()).max().unwrap_or(0));
            line += &format!(
                " | restrict{} | full support {}: {}",
                if restrict_bad + vertex_bad > 0 { format!(" {} BAD", restrict_bad + vertex_bad) } else { " ok".into() },
                full.len(),
                full.iter().map(|b| vec_text(b)).collect::<Vec<_>>().join(" ")
            );
            for (i, j, _) in q.arrows().into_iter().filter(|&(i, j, _)| c.contains(&i) && c.contains(&j)) {
                let mut cut = q.clone();
                cut.set_pair(i, j, 0);
                let mut count: HashMap<Vec<Vec<i64>>, usize> = HashMap::new();
                let mut deleted = 0;
                for cs in &all {
                    let kept: Vec<Vec<i64>> = cs.iter().filter(|b| b[i] == 0 || b[j] == 0).cloned().collect();
                    deleted = deleted.max(cs.len() - kept.len());
                    *count.entry(kept).or_default() += 1;
                }
                let bad: usize = count.iter().filter(|(k, _)| !is_mgs_by_c_vectors(&cut, k)).map(|(_, v)| v).sum();
                let (_, ocut) = search(&cut, &opts);
                let cut_total = ocut.root.as_ref().map_or(0, |r| r.counts.total_within(ocut.searched_to).to_u128().unwrap_or(u128::MAX));
                let fibre = count.values().max().copied().unwrap_or(0);
                t.cut_bad += bad;
                t.cut_not_onto += usize::from(count.len() as u128 != cut_total);
                t.max_fibre = t.max_fibre.max(fibre);
                t.max_deleted = t.max_deleted.max(deleted);
                line += &format!(
                    " | cut {}->{}: {}/{cut_total} fibre ≤{fibre} del ≤{deleted}{}",
                    i + 1,
                    j + 1,
                    count.len(),
                    if bad > 0 { format!(" {bad} BAD") } else { String::new() }
                );
            }
        }
        println!("{line}");
    }
    println!(
        "summary: {} quivers ({} listed in full, {} incomplete); δ_C-depth > 1: {}; cover bound fails: {}; cover where not predicted or missing where predicted: {}; restriction to C or to some Q∖v not an MGS: {}; cut image not an MGS: {}; cut map not onto: {}; largest fibre {}; most deleted steps {}; most full-support step c-vectors {} (largest entry {}); Dynkin attachments: {} quivers, depth > 1 in {}; type Ã(p,1): {} quivers, covering C in {}; product of restrictions over C: largest fibre {} for Ã(p,1), {} otherwise",
        t.quivers, t.listed, t.incomplete, t.depth_over_1, t.bound_fails, t.cover_mismatch, t.restrict_bad, t.cut_bad, t.cut_not_onto, t.max_fibre, t.max_deleted, t.full_support, t.full_entry, t.dynkin_attached, t.dynkin_depth_over_1, t.p1, t.p1_cover, t.p1_product_fibre, t.other_product_fibre
    );
}
