//! The quivers this checker accepts: acyclic ones. A quiver with an oriented
//! cycle is refused, and the refusal names one of its cycles.
//!
//! Everything the checker leans on is stated for acyclic quivers: c-vectors
//! are real Schur roots, the shortest MGS has length n (BDP 2.20), and the
//! Dynkin, tame and No Gap theorems all concern path algebras.

use std::fmt;

use serde::Serialize;

use crate::matrix::Exchange;

/// The quiver has an oriented cycle, so the checker refuses it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OrientedCycle {
    /// 1-based, with the first vertex repeated at the end: [1, 2, 3, 1].
    pub cycle: Vec<usize>,
}

impl fmt::Display for OrientedCycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = self.cycle.iter().map(usize::to_string).collect::<Vec<_>>().join(" → ");
        write!(f, "the quiver has an oriented cycle {path}; this checker accepts only acyclic quivers")
    }
}

impl std::error::Error for OrientedCycle {}

/// One oriented cycle of q (0-based, first vertex repeated at the end), if it has any.
pub fn oriented_cycle(q: &Exchange) -> Option<Vec<usize>> {
    // Depth-first search: an arrow back to a vertex still on the stack closes a cycle.
    fn visit(q: &Exchange, v: usize, state: &mut [u8], stack: &mut Vec<usize>) -> Option<Vec<usize>> {
        state[v] = 1;
        stack.push(v);
        for w in (0..q.n()).filter(|&w| q.get(v, w) > 0) {
            if state[w] == 1 {
                let at = stack.iter().position(|&x| x == w).expect("w is on the stack");
                let mut cycle = stack[at..].to_vec();
                cycle.push(w);
                return Some(cycle);
            }
            if state[w] == 0 {
                if let Some(c) = visit(q, w, state, stack) {
                    return Some(c);
                }
            }
        }
        stack.pop();
        state[v] = 2;
        None
    }
    let mut state = vec![0u8; q.n()];
    (0..q.n()).find_map(|v| if state[v] == 0 { visit(q, v, &mut state, &mut Vec::new()) } else { None })
}

/// Ok for an acyclic quiver; otherwise the cycle that rules it out.
pub fn admit(q: &Exchange) -> Result<(), OrientedCycle> {
    match oriented_cycle(q) {
        Some(c) => Err(OrientedCycle { cycle: c.into_iter().map(|v| v + 1).collect() }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::is_acyclic;

    fn q(text: &str) -> Exchange {
        Exchange::from_arrow_text(text, None).unwrap()
    }

    #[test]
    fn names_a_cycle_or_admits() {
        assert_eq!(admit(&q("1->2, 2->3, 3->1")).unwrap_err().cycle, vec![1, 2, 3, 1]);
        let e = admit(&q("1->2, 2->3, 3->4, 4->2")).unwrap_err();
        assert_eq!(e.cycle, vec![2, 3, 4, 2]);
        assert_eq!(e.to_string(), "the quiver has an oriented cycle 2 → 3 → 4 → 2; this checker accepts only acyclic quivers");
        assert!(admit(&q("1->2*2, 2->3, 1->3")).is_ok());
        assert!(admit(&q("1->2, 3->2, 3->4, 1->4")).is_ok());
    }

    #[test]
    fn agrees_with_the_topological_sort_on_every_small_quiver() {
        // Every orientation (with multiplicities 0..=1) of the complete graph on 4 vertices.
        let pairs: Vec<(usize, usize)> = (0..4).flat_map(|i| (i + 1..4).map(move |j| (i, j))).collect();
        for code in 0..3u32.pow(pairs.len() as u32) {
            let mut r = Exchange::zero(4);
            let mut c = code;
            for &(i, j) in &pairs {
                r.set_pair(i, j, (c % 3) as i64 - 1);
                c /= 3;
            }
            match oriented_cycle(&r) {
                Some(cycle) => {
                    assert!(!is_acyclic(&r));
                    assert!(cycle.windows(2).all(|w| r.get(w[0], w[1]) > 0), "{cycle:?} in {}", r.arrow_text());
                }
                None => assert!(is_acyclic(&r)),
            }
        }
    }
}
