//! What the maximal green sequences pass through, read off a finished search.
//!
//! Each step of an MGS mutates a green vertex, whose c-vector is then a
//! positive real Schur root: the dimension vector of the brick that labels
//! the step. This collects those step c-vectors over every MGS of length
//! ≤ the bound, and keeps the maximal ones under the entrywise order. That
//! order is only partial, so there can be several maximal c-vectors, none
//! above another. For a Dynkin quiver every positive root is a step c-vector
//! of the longest MGS (BDP Thm 4.4), so there are |Φ⁺| of them, and the
//! only maximal one is the highest root.
//!
//! It also counts the exchange-graph classes the MGS pass through, and the
//! most parallel arrows any of their exchange matrices has.

use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;

use crate::engine::{Engine, Entry};
use crate::framed::Framed;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MaximalC {
    pub vector: Vec<i64>,
    /// A shortest green prefix whose last step has this c-vector (1-based). It finishes to an MGS.
    pub prefix: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Seeds {
    /// Exchange-graph classes some MGS passes through, the start and the all-red end included.
    pub classes: usize,
    /// Distinct c-vectors at the steps of the MGS.
    pub step_c_vectors: usize,
    /// The step c-vectors with no other step c-vector ≥ them in every entry: by total, largest first.
    pub maximal: Vec<MaximalC>,
    /// The largest entry of any step c-vector.
    pub largest_entry: i64,
    /// The largest |b_ij| in any exchange matrix along an MGS: the most parallel arrows met.
    pub most_arrows: i64,
}

fn most_arrows(f: &Framed) -> i64 {
    let n = f.n();
    (0..n).flat_map(|i| (0..n).map(move |j| (i, j))).map(|(i, j)| f.b(i, j).abs()).max().unwrap_or(0)
}

impl Engine {
    /// The MGS of length ≤ `bound` from `start`, or None if there are none.
    /// Call it after a search that reached `bound`: it only reads the memo.
    pub fn seeds(&self, start: &Framed, bound: u32) -> Option<Seeds> {
        let root = self.lookup(start)?;
        if !root.counts.any_within(bound) {
            return None;
        }
        // Breadth first, so each class is met first at its least depth, where
        // it has the most length left and so the most steps that can still finish.
        let mut seen: FxHashSet<*const Entry> = FxHashSet::default();
        seen.insert(Rc::as_ptr(&root));
        let mut steps: FxHashMap<Box<[i64]>, Vec<u8>> = FxHashMap::default();
        let mut most = most_arrows(start);
        let mut layer: Vec<(Framed, Vec<u8>)> = vec![(start.clone(), Vec::new())];
        for depth in 0..bound {
            let left = bound - depth - 1;
            let mut next = Vec::new();
            for (s, path) in &layer {
                for k in self.moves(s) {
                    let Ok(child) = s.mutate(k) else { continue };
                    let Some(e) = self.lookup(&child).filter(|e| e.counts.any_within(left)) else { continue };
                    let walked = || path.iter().copied().chain([k as u8]).collect::<Vec<u8>>();
                    steps.entry(s.c_vector(k).into()).or_insert_with(walked);
                    if seen.insert(Rc::as_ptr(&e)) {
                        most = most.max(most_arrows(&child));
                        if !child.all_red() {
                            next.push((child, walked()));
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            layer = next;
        }
        let maximal = maximal(steps.iter().map(|(v, p)| (&**v, p)));
        Some(Seeds {
            classes: seen.len(),
            step_c_vectors: steps.len(),
            largest_entry: maximal.iter().flat_map(|m| m.vector.iter().copied()).max().unwrap_or(0),
            maximal,
            most_arrows: most,
        })
    }
}

/// The maximal vectors under the entrywise order, largest total first.
fn maximal<'a>(vectors: impl Iterator<Item = (&'a [i64], &'a Vec<u8>)>) -> Vec<MaximalC> {
    let mut all: Vec<(&[i64], &Vec<u8>)> = vectors.collect();
    all.sort_by(|a, b| b.0.iter().sum::<i64>().cmp(&a.0.iter().sum::<i64>()).then_with(|| b.0.cmp(a.0)));
    // A vector below another has a smaller total, so it comes later; and if it
    // is below anything, it is below something already kept (by transitivity).
    let mut kept: Vec<(&[i64], &Vec<u8>)> = Vec::new();
    for (v, p) in all {
        if !kept.iter().any(|(w, _)| w.iter().zip(v).all(|(a, b)| a >= b)) {
            kept.push((v, p));
        }
    }
    kept.into_iter().map(|(v, p)| MaximalC { vector: v.to_vec(), prefix: p.iter().map(|&k| k as usize + 1).collect() }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{search, SearchOptions};
    use crate::matrix::Exchange;
    use crate::verify::{verify, Verdict};

    fn seeds(text: &str) -> Seeds {
        let q = Exchange::from_arrow_text(text, None).unwrap();
        let (engine, out) = search(&q, &SearchOptions::default());
        engine.seeds(&Framed::new(&q), out.searched_to).unwrap()
    }

    #[test]
    fn maximal_keeps_an_antichain() {
        let p = vec![];
        let vs: Vec<Vec<i64>> = vec![vec![1, 0, 0], vec![1, 1, 0], vec![0, 1, 1], vec![0, 0, 1], vec![0, 1, 0]];
        let m: Vec<Vec<i64>> = maximal(vs.iter().map(|v| (v.as_slice(), &p))).into_iter().map(|m| m.vector).collect();
        assert_eq!(m, vec![vec![1, 1, 0], vec![0, 1, 1]]);
    }

    #[test]
    fn dynkin_gives_every_positive_root_and_the_highest_root() {
        let a3 = seeds("1->2, 2->3");
        assert_eq!((a3.step_c_vectors, a3.largest_entry, a3.most_arrows), (6, 1, 1));
        assert_eq!(a3.maximal.iter().map(|m| m.vector.clone()).collect::<Vec<_>>(), vec![vec![1, 1, 1]]);
        let d4 = seeds("1->2, 3->2, 4->2");
        assert_eq!(d4.step_c_vectors, 12);
        assert_eq!(d4.maximal.iter().map(|m| m.vector.clone()).collect::<Vec<_>>(), vec![vec![1, 2, 1, 1]]);
    }

    #[test]
    fn the_kronecker_quiver_has_two_maximal_c_vectors() {
        // Its one MGS, (1, 2), mutates the simple roots, and neither is above the other.
        let k = seeds("1->2*2");
        assert_eq!((k.classes, k.step_c_vectors), (3, 2));
        assert_eq!(k.maximal.iter().map(|m| m.vector.clone()).collect::<Vec<_>>(), vec![vec![1, 0], vec![0, 1]]);
    }

    #[test]
    fn every_prefix_is_green_and_ends_on_its_vector() {
        let q = Exchange::from_arrow_text("1->2, 2->3, 1->3", None).unwrap();
        let (engine, out) = search(&q, &SearchOptions::default());
        for m in engine.seeds(&Framed::new(&q), out.searched_to).unwrap().maximal {
            let v = verify(&q, &m.prefix).unwrap();
            assert_ne!(v.verdict, Verdict::NotGreen);
            assert_eq!(v.steps.last().unwrap().c_vector, m.vector);
        }
    }
}
