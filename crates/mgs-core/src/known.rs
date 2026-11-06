//! What the literature says about a quiver's maximal green sequences, so a
//! search can be checked against it and its stopping rule upgraded from a
//! conjecture to a theorem where one applies.
//!
//! - BDP = Brüstle, Dupont, Pérotin, "On maximal green sequences", IMRN 2014.
//!   Lemma 2.20 with Cor. 2.18: for acyclic Q the shortest MGS has length n.
//!   Thm 4.4: for a Dynkin quiver the longest has length |Φ⁺|. Thm 5.2:
//!   affine quivers have finitely many.
//! - Hermes, Igusa, "The No Gap Conjecture for tame hereditary algebras",
//!   arXiv:1601.04054, Cor. 4.0.2: "The No Gap Conjecture holds for any
//!   simply laced quiver of tame type."
//! - Kase, Nakashima, "Lengths of maximal green sequences for tame path
//!   algebras", arXiv:2010.13996 (preprint): the longest MGS of a tame quiver
//!   does not depend on the orientation, and has length (a+b−1)(a+b)/2 + ab for
//!   Ã(a,b), 2n² − 2n − 2 for D̃_n, and 78, 159, 390 for Ẽ6, Ẽ7, Ẽ8.

use serde::Serialize;

use crate::classify::{classify, is_acyclic, Kind};
use crate::matrix::Exchange;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Fact {
    pub value: u32,
    pub source: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Facts {
    pub kind: Kind,
    pub kind_name: String,
    pub acyclic: bool,
    pub shortest: Option<Fact>,
    pub longest: Option<Fact>,
    /// A theorem saying the lengths of MGS form an interval, if one covers this quiver.
    pub no_gap: Option<&'static str>,
    /// A theorem saying there are finitely many MGS, if one covers this quiver.
    pub finite: Option<&'static str>,
}

const BDP_SHORTEST: &str = "BDP Lemma 2.20 and Cor. 2.18 (acyclic: the shortest MGS has length n)";
const BDP_LONGEST: &str = "BDP Thm 4.4 (Dynkin: the longest MGS has length |Φ⁺|)";
const KN_LONGEST: &str = "Kase–Nakashima, arXiv:2010.13996 (tame: the longest MGS, any orientation)";
const HI_NO_GAP: &str = "Hermes–Igusa, arXiv:1601.04054, Cor. 4.0.2 (No Gap for tame quivers)";
const BDP_FINITE_TAME: &str = "BDP Thm 5.2 (affine quivers have finitely many MGS)";
const BDP_FINITE_DYNKIN: &str = "BDP Thm 4.1 (finite cluster type: finitely many MGS)";

/// Number of positive roots of a Dynkin type.
pub fn positive_roots(family: char, rank: usize) -> usize {
    match (family, rank) {
        ('A', n) => n * (n + 1) / 2,
        ('D', n) => n * (n - 1),
        ('E', 6) => 36,
        ('E', 7) => 63,
        ('E', 8) => 120,
        _ => unreachable!("not a Dynkin type: {family}{rank}"),
    }
}

/// The longest MGS of a tame quiver, by Kase–Nakashima.
pub fn tame_longest(kind: &Kind) -> Option<usize> {
    match *kind {
        Kind::ATilde { a, b } => Some((a + b - 1) * (a + b) / 2 + a * b),
        Kind::Euclidean { family: 'D', rank: n } => Some(2 * n * n - 2 * n - 2),
        Kind::Euclidean { family: 'E', rank: 6 } => Some(78),
        Kind::Euclidean { family: 'E', rank: 7 } => Some(159),
        Kind::Euclidean { family: 'E', rank: 8 } => Some(390),
        _ => None,
    }
}

pub fn facts(q: &Exchange) -> Facts {
    let kind = classify(q);
    let acyclic = is_acyclic(q);
    let n = q.n() as u32;
    let fact = |value: usize, source| Some(Fact { value: value as u32, source });
    let shortest = acyclic.then_some(Fact { value: n, source: BDP_SHORTEST });
    let (longest, no_gap, finite) = match &kind {
        Kind::Dynkin { family, rank } => (fact(positive_roots(*family, *rank), BDP_LONGEST), None, Some(BDP_FINITE_DYNKIN)),
        k if k.is_tame() => (tame_longest(k).and_then(|v| fact(v, KN_LONGEST)), Some(HI_NO_GAP), Some(BDP_FINITE_TAME)),
        _ => (None, None, None),
    };
    Facts { kind_name: kind.name(), kind, acyclic, shortest, longest, no_gap, finite }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn longest(text: &str) -> Option<u32> {
        facts(&Exchange::from_arrow_text(text, None).unwrap()).longest.map(|f| f.value)
    }

    #[test]
    fn literature_values() {
        assert_eq!(longest("1->2, 2->3"), Some(6));
        assert_eq!(longest("1->2*2"), Some(2));
        assert_eq!(longest("1->2, 2->3, 1->3"), Some(5)); // BDP Example 5.3
        assert_eq!(longest("1->2, 2->3, 1->4, 4->3"), Some(10));
        assert_eq!(longest("1->5, 2->5, 3->5, 4->5"), Some(22));
        assert_eq!(longest("1->2*2, 2->3"), None);
        let f = facts(&Exchange::from_arrow_text("1->2, 2->3, 3->1", None).unwrap());
        assert_eq!((f.shortest, f.longest, f.no_gap), (None, None, None));
    }
}
