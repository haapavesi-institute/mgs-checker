//! Where sincere step c-vectors sit in the Auslander–Reiten quiver: a record
//! of two hypotheses that the six-vertex trees refute. The inheritance survey
//! itself is `mgs sincere` (`mgs_core::sincere`); this example repeats its
//! check and adds the τ tallies.
//!
//!     cargo run --release -p mgs-core --example tau_positions -- N M [gap] [trees|sparse] [list]
//!
//! Arguments: vertex count, largest multiplicity, max gap (default 0), an
//! optional filter (`trees`: n − 1 edges; `sparse`: at most n edges), and
//! `list` to print every non-Dynkin quiver's sincere c-vectors.
//!
//! Each step c-vector is a real Schur root, so it is the dimension vector of
//! one exceptional module: preprojective τ^{-k} P_i, preinjective τ^{k} I_i,
//! or regular. Tallied:
//! - **First sincere.** k, and how far k is past the first sincere module of
//!   its τ-orbit. On 5 vertices every sincere preprojective or preinjective
//!   step c-vector of a wild quiver is the first sincere one of its orbit;
//!   on 6-vertex wild trees some are up to 3 steps past it.
//! - **τ-neighbours.** For a sincere step c-vector c = dim M, each c falls in
//!   the first class that applies: A, M is projective or injective; B, one of
//!   τM, τ⁻¹M (dimension vectors Φc, Φ⁻¹c) is not sincere and is itself a
//!   step c-vector of Q; C, one of them is not sincere; D, both are sincere.
//!   "Always A or B" would bound the sincere c-vectors by induction and so
//!   prove finiteness. It holds on 5 vertices (wild: 10 A, 8 B) and fails on
//!   6-vertex wild trees (12 A, 566 B, 286 C, 48 D).

#![allow(clippy::needless_range_loop)]

use std::collections::{BTreeMap, HashMap};

use mgs_core::canonical::canonical_key;
use mgs_core::classify::{classify, components, Kind};
use mgs_core::engine::{search, SearchOptions, Stop};
use mgs_core::enumerate::acyclic_quivers;
use mgs_core::framed::Framed;
use mgs_core::Exchange;

type V = Vec<i64>;

fn sub(q: &Exchange, keep: &[usize]) -> Exchange {
    Exchange::from_rows(keep.iter().map(|&i| keep.iter().map(|&j| q.get(i, j)).collect()).collect()).unwrap()
}

fn step_vectors(q: &Exchange, opts: &SearchOptions) -> (Vec<V>, bool) {
    let (engine, out) = search(q, opts);
    (engine.step_c_vectors(&Framed::new(q), out.searched_to), matches!(out.stop, Stop::Exhausted | Stop::Gap))
}

fn sincere(v: &[i64]) -> bool {
    v.iter().all(|&x| x > 0)
}

fn text(v: &[i64]) -> String {
    v.iter().map(i64::to_string).collect::<Vec<_>>().join("")
}

/// Vertices in an order where every arrow goes from earlier to later.
fn topological(q: &Exchange) -> Vec<usize> {
    let n = q.n();
    let mut order = Vec::new();
    let mut placed = vec![false; n];
    while order.len() < n {
        let v = (0..n).find(|&v| !placed[v] && (0..n).all(|u| placed[u] || q.get(u, v) <= 0)).expect("acyclic");
        placed[v] = true;
        order.push(v);
    }
    order
}

/// dim τ^{-k} P_i for k = 0, 1, … (as `table[k][i]`), until every total passes `cap`
/// or a vector stops being positive (a Dynkin quiver). With `dual`, dim τ^{k} I_i.
fn orbits(q: &Exchange, cap: i64, dual: bool) -> Vec<Vec<V>> {
    let n = q.n();
    // arrows(i, j) = number of arrows i → j, in the quiver or (dual) its opposite.
    let arrows = |i: usize, j: usize| if dual { q.get(j, i).max(0) } else { q.get(i, j).max(0) };
    let mut order = topological(q);
    if dual {
        order.reverse();
    }
    // P_i (or I_i): paths from i, counted from the sinks back.
    let mut first = vec![vec![0i64; n]; n];
    for &i in order.iter().rev() {
        first[i][i] = 1;
        for j in 0..n {
            for x in 0..n {
                first[i][x] += arrows(i, j) * first[j][x];
            }
        }
    }
    let mut table = vec![first];
    for _ in 0..400 {
        let prev = table.last().unwrap();
        let mut next = vec![vec![0i64; n]; n];
        let mut alive = false;
        for &i in order.iter().rev() {
            let mut v: V = prev[i].iter().map(|x| -x).collect();
            for j in 0..n {
                for x in 0..n {
                    v[x] += arrows(j, i) * prev[j][x] + arrows(i, j) * next[j][x];
                }
            }
            if v.iter().any(|&x| x < 0) {
                return table; // Dynkin: the orbit has ended
            }
            alive |= v.iter().sum::<i64>() <= cap;
            next[i] = v;
        }
        table.push(next);
        if !alive {
            break;
        }
    }
    table
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Place {
    /// τ^{-k} P_i, with `past` = k minus the first sincere index of that orbit (None: not sincere yet).
    Preprojective {
        k: usize,
        past: Option<usize>,
    },
    Preinjective {
        k: usize,
        past: Option<usize>,
    },
    Regular,
}

fn place(v: &[i64], pre: &[Vec<V>], inj: &[Vec<V>]) -> Place {
    let find = |table: &[Vec<V>]| {
        for (k, row) in table.iter().enumerate() {
            if let Some(i) = row.iter().position(|w| w == v) {
                let first = table.iter().position(|r| sincere(&r[i]));
                return Some((k, first.filter(|&f| k >= f).map(|f| k - f)));
            }
        }
        None
    };
    if let Some((k, past)) = find(pre) {
        return Place::Preprojective { k, past };
    }
    match find(inj) {
        Some((k, past)) => Place::Preinjective { k, past },
        None => Place::Regular,
    }
}

/// Φc and Φ⁻¹c: the dimension vectors of τM and τ⁻¹M when c = dim M (a
/// negative entry means M is projective, respectively injective).
fn translates(q: &Exchange, c: &[i64]) -> (V, V) {
    let n = q.n();
    let arrows = |i: usize, j: usize| q.get(i, j).max(0);
    // paths[i][j] = number of paths i → j: the inverse of the Euler matrix E = 1 − (arrows).
    let mut paths = vec![vec![0i64; n]; n];
    for &i in topological(q).iter().rev() {
        paths[i][i] = 1;
        for j in 0..n {
            for x in 0..n {
                paths[i][x] += arrows(i, j) * paths[j][x];
            }
        }
    }
    let e = |i: usize, j: usize| i64::from(i == j) - arrows(i, j);
    let et_c: V = (0..n).map(|i| (0..n).map(|j| e(j, i) * c[j]).sum()).collect();
    let e_c: V = (0..n).map(|i| (0..n).map(|j| e(i, j) * c[j]).sum()).collect();
    let tau: V = (0..n).map(|i| -(0..n).map(|j| paths[i][j] * et_c[j]).sum::<i64>()).collect();
    let tau_inverse: V = (0..n).map(|i| -(0..n).map(|j| paths[j][i] * e_c[j]).sum::<i64>()).collect();
    (tau, tau_inverse)
}

#[derive(Default)]
struct Class {
    /// Sincere c-vectors by τ-neighbour class A, B, C, D (see the module comment).
    neighbours: [usize; 4],
    quivers: usize,
    with_sincere: usize,
    most_sincere: usize,
    largest_entry: i64,
    regular: usize,
    sincere_regular: usize,
    largest_k: usize,
    /// How far past the first sincere module of its orbit: distance → c-vectors.
    past: BTreeMap<usize, usize>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let n: usize = args.first().map_or(4, |a| a.parse().expect("vertex count"));
    let m: i64 = args.get(1).map_or(1, |a| a.parse().expect("multiplicity"));
    let gap: u32 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(0);
    let edges = |q: &Exchange| q.arrows().len();
    let filter = |q: &Exchange| {
        if args.iter().any(|a| a == "trees") {
            edges(q) == n - 1
        } else if args.iter().any(|a| a == "sparse") {
            edges(q) <= n
        } else {
            true
        }
    };
    let list = args.iter().any(|a| a == "list");
    let opts = SearchOptions { max_gap: gap, max_length: 160, max_states: 3_000_000, prune: true };
    let quivers = acyclic_quivers(n, m, filter).unwrap();
    // Whether a connected quiver has a sincere step c-vector, by relabelling class.
    let mut memo: HashMap<Vec<i64>, Option<bool>> = HashMap::new();
    let mut has_sincere = |q: &Exchange| {
        *memo.entry(canonical_key(q, false)).or_insert_with(|| {
            let (vs, ok) = step_vectors(q, &opts);
            ok.then(|| vs.iter().any(|v| sincere(v)))
        })
    };
    let mut classes: BTreeMap<&'static str, Class> = BTreeMap::new();
    let (mut unsettled, mut inherit_checked, mut inherit_fails, mut minimal) = (0, 0, 0, Vec::new());
    for q in &quivers {
        let (vectors, settled) = step_vectors(q, &opts);
        if !settled {
            unsettled += 1;
            println!("UNSETTLED {}", q.arrow_text());
            continue;
        }
        let sinc: Vec<&V> = vectors.iter().filter(|v| sincere(v)).collect();
        let kind = classify(q);
        let name = match kind {
            Kind::Dynkin { .. } => "Dynkin",
            ref k if k.is_tame() => "tame",
            _ => "wild",
        };
        // Inheritance: a connected Q ∖ v without a sincere c-vector should leave Q without one.
        let mut below_all_sincere = true;
        for v in 0..n {
            let keep: Vec<usize> = (0..n).filter(|&u| u != v).collect();
            let part = sub(q, &keep);
            if keep.is_empty() || components(&part).len() != 1 {
                continue;
            }
            match has_sincere(&part) {
                Some(true) => {}
                Some(false) => {
                    below_all_sincere = false;
                    inherit_checked += 1;
                    if !sinc.is_empty() {
                        inherit_fails += 1;
                        println!("INHERITANCE FAILS {}: Q∖{} has no sincere c-vector but Q has {}", q.arrow_text(), v + 1, text(sinc[0]));
                    }
                }
                None => below_all_sincere = false,
            }
        }
        if sinc.is_empty() && below_all_sincere {
            minimal.push(format!("{} [{}]", q.arrow_text(), kind.name()));
        }
        let c = classes.entry(name).or_default();
        c.quivers += 1;
        c.with_sincere += usize::from(!sinc.is_empty());
        c.most_sincere = c.most_sincere.max(sinc.len());
        c.largest_entry = c.largest_entry.max(sinc.iter().flat_map(|v| v.iter().copied()).max().unwrap_or(0));
        if name == "Dynkin" {
            continue;
        }
        let cap = vectors.iter().map(|v| v.iter().sum::<i64>()).max().unwrap_or(0);
        let (pre, inj) = (orbits(q, cap, false), orbits(q, cap, true));
        if pre.len() > 1 {
            assert_eq!(translates(q, &pre[0][0]).1, pre[1][0], "Φ⁻¹ must send P to τ⁻¹P");
        }
        let mut shown = Vec::new();
        for v in &vectors {
            let p = place(v, &pre, &inj);
            match p {
                Place::Regular => {
                    c.regular += 1;
                    c.sincere_regular += usize::from(sincere(v));
                }
                Place::Preprojective { k, past } | Place::Preinjective { k, past } => {
                    c.largest_k = c.largest_k.max(k);
                    if let Some(d) = past {
                        *c.past.entry(d).or_default() += 1;
                    }
                }
            }
            if sincere(v) {
                let (up, down) = translates(q, v);
                let gone = |w: &V| w.iter().any(|&x| x < 0);
                let thin = |w: &V| !gone(w) && !sincere(w);
                let class = if gone(&up) || gone(&down) {
                    0
                } else if [&up, &down].into_iter().any(|w| thin(w) && vectors.contains(w)) {
                    1
                } else if thin(&up) || thin(&down) {
                    2
                } else {
                    3
                };
                c.neighbours[class] += 1;
                shown.push(format!("{} {:?} {}(τ {}, τ⁻ {})", text(v), p, ["A", "B", "C", "D"][class], text(&up), text(&down)));
            }
        }
        if list && !shown.is_empty() {
            println!("{name} {}: {}", q.arrow_text(), shown.join("; "));
        }
    }
    println!("== {} connected acyclic {n}-vertex quivers, multiplicities ≤ {m}, max gap {gap}; {unsettled} unsettled", quivers.len());
    println!("inheritance: {inherit_checked} (Q, v) pairs with Q∖v connected and without a sincere c-vector; Q has one anyway in {inherit_fails}");
    println!("minimal quivers without a sincere c-vector ({}):", minimal.len());
    for q in &minimal {
        println!("  {q}");
    }
    for (name, c) in &classes {
        println!(
            "{name}: {} quivers, {} with a sincere c-vector (at most {} each, largest entry {}); regular c-vectors {}, sincere ones {}; largest τ-power {}; sincere c-vectors by steps past the orbit's first sincere module: {:?}; τ-neighbour classes A/B/C/D: {:?}",
            c.quivers, c.with_sincere, c.most_sincere, c.largest_entry, c.regular, c.sincere_regular, c.largest_k, c.past, c.neighbours
        );
    }
}
