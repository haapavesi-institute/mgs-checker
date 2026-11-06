//! Full subquivers that are cycles, and the patterns no step c-vector of a
//! maximal green sequence is known to cover.
//!
//! A *cycle* is a vertex set C with |C| ≥ 3 whose full subquiver has a cycle
//! as its underlying graph, so it has no chord. In an acyclic quiver it is of
//! type (p, q): p arrows one way round and q the other, p ≥ q ≥ 1.
//!
//! A c-vector *covers* a vertex set if its support contains it. The patterns:
//! - a multiple arrow: no step c-vector covers its two ends (Igusa–Zhou,
//!   arXiv:1902.07375v1, Theorem 3.12). Proved.
//! - a cycle of type (p, 1), and the alternating square (two sources, two
//!   sinks): conjectured, from the surveys in [`crate::sincere`]. Among bare
//!   cycles of length 3 to 8 these are exactly the orientations with no
//!   sincere step c-vector.

use serde::Serialize;

use crate::matrix::Exchange;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cycle {
    /// Vertices in walking order, 0-based.
    pub order: Vec<usize>,
    pub p: usize,
    pub q: usize,
    /// Some edge of the cycle is a multiple arrow.
    pub multi: bool,
    pub sources: Vec<usize>,
    pub sinks: Vec<usize>,
}

/// Every vertex set of size ≥ 3 whose full subquiver is a cycle.
pub fn cycles(q: &Exchange) -> Vec<Cycle> {
    let n = q.n();
    let mut out = Vec::new();
    for mask in 0u64..1 << n {
        let vs: Vec<usize> = (0..n).filter(|&v| mask >> v & 1 == 1).collect();
        if vs.len() < 3 || vs.iter().any(|&v| vs.iter().filter(|&&w| q.get(v, w) != 0).count() != 2) {
            continue;
        }
        let mut order = vec![vs[0]];
        while let Some(next) = vs.iter().copied().find(|&w| !order.contains(&w) && q.get(*order.last().unwrap(), w) != 0) {
            order.push(next);
        }
        if order.len() != vs.len() {
            continue; // two disjoint cycles
        }
        let len = order.len();
        let fwd = (0..len).filter(|&t| q.get(order[t], order[(t + 1) % len]) > 0).count();
        let multi = (0..len).any(|t| q.get(order[t], order[(t + 1) % len]).abs() >= 2);
        let on = |v: usize| vs.iter().copied().filter(move |&w| q.get(v, w) != 0);
        let sources = vs.iter().copied().filter(|&v| on(v).all(|w| q.get(v, w) > 0)).collect();
        let sinks = vs.iter().copied().filter(|&v| on(v).all(|w| q.get(v, w) < 0)).collect();
        out.push(Cycle { order, p: fwd.max(len - fwd), q: fwd.min(len - fwd), multi, sources, sinks });
    }
    out
}

/// A pattern in a quiver. Vertices are 1-based.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Pattern {
    MultipleArrow {
        from: usize,
        to: usize,
        arrows: i64,
    },
    /// A full cycle of type (p, 1) with single arrows.
    CycleP1 {
        vertices: Vec<usize>,
        p: usize,
    },
    /// A full square with two sources and two sinks, single arrows.
    AlternatingSquare {
        vertices: Vec<usize>,
    },
}

impl Pattern {
    /// The vertices a c-vector must not cover, 1-based and sorted.
    pub fn vertices(&self) -> Vec<usize> {
        let mut v = match self {
            Self::MultipleArrow { from, to, .. } => vec![*from, *to],
            Self::CycleP1 { vertices, .. } | Self::AlternatingSquare { vertices } => vertices.clone(),
        };
        v.sort_unstable();
        v
    }

    /// Whether "no step c-vector covers it" is a theorem (only for multiple arrows).
    pub fn proved(&self) -> bool {
        matches!(self, Self::MultipleArrow { .. })
    }

    pub fn covered_by(&self, c: &[i64]) -> bool {
        self.vertices().iter().all(|&v| c[v - 1] > 0)
    }

    pub fn describe(&self) -> String {
        let set = |v: &[usize]| v.iter().map(usize::to_string).collect::<Vec<_>>().join(", ");
        match self {
            Self::MultipleArrow { from, to, arrows } => format!("{arrows} arrows {from} → {to}"),
            Self::CycleP1 { vertices, p } => format!("a full cycle of type ({p}, 1) on {{{}}}", set(vertices)),
            Self::AlternatingSquare { vertices } => format!("a full alternating square on {{{}}}", set(vertices)),
        }
    }
}

/// The known patterns in q.
pub fn patterns(q: &Exchange) -> Vec<Pattern> {
    let mut out: Vec<Pattern> = q
        .arrows()
        .into_iter()
        .filter(|&(_, _, m)| m >= 2)
        .map(|(i, j, m)| Pattern::MultipleArrow { from: i + 1, to: j + 1, arrows: m })
        .collect();
    for c in cycles(q).into_iter().filter(|c| !c.multi) {
        let vertices: Vec<usize> = c.order.iter().map(|v| v + 1).collect();
        if c.q == 1 {
            out.push(Pattern::CycleP1 { vertices, p: c.p });
        } else if c.order.len() == 4 && c.sources.len() == 2 {
            out.push(Pattern::AlternatingSquare { vertices });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(text: &str) -> Exchange {
        Exchange::from_arrow_text(text, None).unwrap()
    }

    #[test]
    fn cycle_types() {
        let c = cycles(&q("1->2, 2->3, 3->4, 1->4"));
        assert_eq!((c.len(), c[0].p, c[0].q, c[0].sources.len()), (1, 3, 1, 1));
        let alt = cycles(&q("1->3, 1->4, 2->3, 2->4"));
        assert_eq!((alt[0].p, alt[0].q, alt[0].sources.len(), alt[0].sinks.len()), (2, 2, 2, 2));
        // A chord splits the square into two triangles; the square itself is no longer a full cycle.
        let chord = cycles(&q("1->2, 2->3, 3->4, 1->4, 1->3"));
        assert_eq!(chord.iter().map(|c| c.order.len()).collect::<Vec<_>>(), vec![3, 3]);
        assert!(cycles(&q("1->2, 2->3, 3->4")).is_empty());
    }

    #[test]
    fn known_patterns() {
        assert_eq!(patterns(&q("1->2*2, 2->3")), vec![Pattern::MultipleArrow { from: 1, to: 2, arrows: 2 }]);
        assert_eq!(patterns(&q("1->2, 2->3, 1->3"))[0].describe(), "a full cycle of type (2, 1) on {1, 2, 3}");
        assert!(matches!(patterns(&q("1->3, 1->4, 2->3, 2->4"))[0], Pattern::AlternatingSquare { .. }));
        // The one-source square is not a pattern: its projective at the source is a sincere step c-vector.
        assert!(patterns(&q("1->2, 2->4, 1->3, 3->4")).is_empty());
        // Every triangle of the transitive tournament is of type (2, 1).
        assert_eq!(patterns(&q("1->2, 1->3, 1->4, 2->3, 2->4, 3->4")).len(), 4);
        let p = Pattern::CycleP1 { vertices: vec![3, 1, 2], p: 2 };
        assert!(p.covered_by(&[1, 1, 2, 0]) && !p.covered_by(&[1, 0, 2, 5]) && !p.proved());
    }
}
