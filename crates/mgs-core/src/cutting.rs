//! Deleting the multiple arrows: Quiver Quest conjecture #7.
//!
//! Let Q′ be Q with every bundle of two or more parallel arrows deleted. The
//! conjecture: if no multiple arrow lies on an oriented cycle, every MGS of Q
//! is an MGS of Q′, with the same c-vector at every step. For acyclic Q this
//! is Igusa–Zhou, arXiv:1902.07375v1, Theorem 3.12: "MGSs of an acyclic
//! quiver Q are a subset of the set of Q-ME-free MGSs of its ME-free
//! version, Q′", with the same C-matrices. Q-ME-free means no c-vector has
//! both ends of a multiple arrow in its support. The open part of #7 is
//! quivers with oriented cycles, which this checker refuses. So here the
//! check tests the engine against that theorem.
//!
//! The check walks every MGS of Q counted by a finished search, playing each
//! step on Q′ at the same time, and stops at the first step where Q′
//! disagrees. It also watches for a step c-vector whose support holds both
//! ends of a multiple arrow.

use rustc_hash::FxHashSet;
use serde::Serialize;

use crate::classify::components;
use crate::engine::Engine;
use crate::framed::Framed;
use crate::matrix::Exchange;

/// Q′ and the bundles deleted, as (from, to, arrows), 0-based.
pub fn delete_multiple(q: &Exchange) -> (Exchange, Vec<(usize, usize, i64)>) {
    let mut out = q.clone();
    let deleted: Vec<(usize, usize, i64)> = q.arrows().into_iter().filter(|&(_, _, m)| m >= 2).collect();
    for &(i, j, _) in &deleted {
        out.set_pair(i, j, 0);
    }
    (out, deleted)
}

/// Some multiple arrow lies on a cycle of the underlying graph: its ends stay
/// connected once its bundle is removed.
pub fn multiple_on_cycle(q: &Exchange) -> bool {
    q.arrows().into_iter().filter(|&(_, _, m)| m >= 2).any(|(i, j, _)| {
        let mut r = q.clone();
        r.set_pair(i, j, 0);
        components(&r).iter().any(|c| c.contains(&i) && c.contains(&j))
    })
}

/// The least k > 1 for an Igusa–Zhou partition cut (their Theorem 4.3, the
/// tool they have for quivers with oriented cycles): a split of the vertices in which
/// every crossing multiplicity is divisible by k. For connected q, one exists
/// exactly when dropping the edges whose multiplicity k doesn't divide
/// disconnects the graph.
pub fn iz_cut(q: &Exchange) -> Option<i64> {
    let n = q.n();
    let top = q.entries().iter().map(|b| b.abs()).max().unwrap_or(0);
    (2..=top).find(|&k| {
        let mut g = Exchange::zero(n);
        for (i, j, m) in q.arrows() {
            if m % k != 0 {
                g.set_pair(i, j, 1);
            }
        }
        components(&g).len() > 1
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CutFailure {
    /// The last step mutates a vertex that is red in Q′.
    RedInPrime { c_in_q: Vec<i64>, c_in_prime: Vec<i64> },
    /// The last step is green in both, with different c-vectors.
    DifferentCVector { c_in_q: Vec<i64>, c_in_prime: Vec<i64> },
    /// The prefix is an MGS of Q, but these vertices are still green in Q′.
    NotMaximalInPrime { still_green: Vec<usize> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "verdict", rename_all = "kebab-case")]
pub enum CutVerdict {
    /// Every MGS walked is an MGS of Q′ with the same c-vector at every step.
    Holds,
    /// The first failure met, at a shortest prefix (1-based).
    Fails { prefix: Vec<usize>, failure: CutFailure },
    /// The walk stopped at its state budget before finishing.
    Unfinished,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CutCheck {
    /// The bundles deleted, (from, to, arrows), 1-based.
    pub deleted: Vec<(usize, usize, i64)>,
    pub q_prime: String,
    pub verdict: CutVerdict,
    /// A step whose c-vector has both ends of a multiple arrow in its support: (prefix, c-vector), 1-based.
    pub label_covers_multiple: Option<(Vec<usize>, Vec<i64>)>,
    /// Pairs (state of Q, state of Q′) walked, up to relabelling both at once.
    pub joint_states: usize,
}

fn joint_key(s: &Framed, t: &Framed) -> Box<[i64]> {
    let mut rows: Vec<Vec<i64>> = (0..s.n()).map(|i| s.c_vector(i).iter().chain(t.c_vector(i)).copied().collect()).collect();
    rows.sort_unstable();
    rows.concat().into_boxed_slice()
}

impl Engine {
    /// Conjecture #7 on the MGS of length ≤ `bound` of q. None if q has no
    /// multiple arrow (then Q′ = Q) or no MGS. Reads the memo of a finished
    /// search; walks at most `budget` joint states.
    pub fn cut_check(&self, q: &Exchange, bound: u32, budget: usize) -> Option<CutCheck> {
        let (prime, deleted) = delete_multiple(q);
        let start = Framed::new(q);
        if deleted.is_empty() || !self.lookup(&start)?.counts.any_within(bound) {
            return None;
        }
        let covers = |c: &[i64]| deleted.iter().any(|&(i, j, _)| c[i] != 0 && c[j] != 0);
        let one_based = |p: &[u8]| p.iter().map(|&k| k as usize + 1).collect::<Vec<usize>>();
        let (mut verdict, mut label) = (CutVerdict::Holds, None);
        let first = Framed::new(&prime);
        let mut seen: FxHashSet<Box<[i64]>> = FxHashSet::default();
        seen.insert(joint_key(&start, &first));
        // Breadth first, as in `seeds`: a pair is met first where it has the most length left.
        let mut layer: Vec<(Framed, Framed, Vec<u8>)> = vec![(start, first, Vec::new())];
        'walk: for depth in 0..bound {
            let mut next = Vec::new();
            for (s, t, path) in &layer {
                for k in self.moves(s) {
                    let Ok(child) = s.mutate(k) else { continue };
                    if !self.lookup(&child).is_some_and(|e| e.counts.any_within(bound - depth - 1)) {
                        continue;
                    }
                    let walked: Vec<u8> = path.iter().copied().chain([k as u8]).collect();
                    let (c, c2) = (s.c_vector(k), t.c_vector(k));
                    if label.is_none() && covers(c) {
                        label = Some((one_based(&walked), c.to_vec()));
                    }
                    if c != c2 {
                        let (c_in_q, c_in_prime) = (c.to_vec(), c2.to_vec());
                        let failure = if t.is_green(k) {
                            CutFailure::DifferentCVector { c_in_q, c_in_prime }
                        } else {
                            CutFailure::RedInPrime { c_in_q, c_in_prime }
                        };
                        verdict = CutVerdict::Fails { prefix: one_based(&walked), failure };
                        break 'walk;
                    }
                    let Ok(child2) = t.mutate(k) else { continue };
                    if child.all_red() && !child2.all_red() {
                        let still_green = child2.greens().into_iter().map(|v| v + 1).collect();
                        verdict = CutVerdict::Fails { prefix: one_based(&walked), failure: CutFailure::NotMaximalInPrime { still_green } };
                        break 'walk;
                    }
                    if !child.all_red() && seen.insert(joint_key(&child, &child2)) {
                        if seen.len() > budget {
                            verdict = CutVerdict::Unfinished;
                            break 'walk;
                        }
                        next.push((child, child2, walked));
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            layer = next;
        }
        Some(CutCheck {
            deleted: deleted.into_iter().map(|(i, j, m)| (i + 1, j + 1, m)).collect(),
            q_prime: prime.arrow_text(),
            verdict,
            label_covers_multiple: label,
            joint_states: seen.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{search, SearchOptions};

    fn q(text: &str) -> Exchange {
        Exchange::from_arrow_text(text, None).unwrap()
    }

    #[test]
    fn structure() {
        let (p, d) = delete_multiple(&q("1->2*2, 2->3, 1->3*3"));
        assert_eq!((p.arrow_text(), d), ("2->3".to_string(), vec![(0, 1, 2), (0, 2, 3)]));
        assert!(multiple_on_cycle(&q("1->2*2, 2->3, 1->3")));
        assert!(!multiple_on_cycle(&q("1->2*2, 2->3, 3->4")));
        assert_eq!(iz_cut(&q("1->2*2, 2->3, 3->4")), Some(2)); // split {1} | {2, 3, 4}
        assert_eq!(iz_cut(&q("1->2*2, 2->3*2, 1->3*4")), Some(2));
        assert_eq!(iz_cut(&q("1->2*2, 2->3, 1->3")), None); // the cycle crosses any split twice
        assert_eq!(iz_cut(&q("1->2*3, 2->3*3, 1->3*2")), Some(3)); // split {2} | {1, 3}
    }

    fn check(text: &str) -> Option<CutCheck> {
        let q = q(text);
        let (engine, out) = search(&q, &SearchOptions::default());
        engine.cut_check(&q, out.searched_to, 1_000_000)
    }

    #[test]
    fn holds_on_acyclic_examples() {
        for text in ["1->2*2, 2->3", "1->2*2, 2->3, 1->3", "1->2*2, 1->3, 3->2", "1->2*2, 2->3, 3->4, 1->4", "1->2*3, 2->3"] {
            let c = check(text).unwrap();
            assert_eq!(c.verdict, CutVerdict::Holds, "{text}");
            assert_eq!(c.label_covers_multiple, None, "{text}");
        }
        assert_eq!(check("1->2, 2->3"), None); // no multiple arrow: Q′ = Q
    }

    #[test]
    fn finds_a_failure_when_the_engine_is_given_a_cyclic_quiver() {
        // BHIT's Figure 3, 1 → 2 → 3 ⇉ 1: the multiple arrow is on an oriented cycle,
        // outside the conjecture (and refused by `check`), but the walk itself still works.
        let c = check("1->2, 2->3, 3->1*2").unwrap();
        assert!(matches!(c.verdict, CutVerdict::Fails { .. }), "{:?}", c.verdict);
    }
}
