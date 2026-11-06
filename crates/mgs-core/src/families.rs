//! The tame (Euclidean) quivers that explorations start from: the Kronecker
//! quiver, and acyclic orientations of Ã_n, D̃_n, Ẽ6, Ẽ7 and Ẽ8, each up to
//! relabelling.

use serde::{Deserialize, Serialize};

use crate::canonical::{canonical, canonical_key};
use crate::classify::is_acyclic;
use crate::matrix::Exchange;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "kebab-case")]
pub enum Family {
    /// 1 ⇉ 2 (Ã1).
    Kronecker,
    /// Ã_rank: a cycle on rank + 1 ≥ 3 vertices.
    ATilde { rank: usize },
    /// D̃_rank, rank ≥ 4: rank + 1 vertices.
    DTilde { rank: usize },
    /// Ẽ6, Ẽ7 or Ẽ8.
    ETilde { rank: usize },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Orientations {
    /// Every acyclic orientation, up to relabelling.
    #[default]
    All,
    /// One standard orientation: every edge from the lower label to the higher.
    One,
}

impl Family {
    pub fn name(&self) -> String {
        match self {
            Self::Kronecker => "Kronecker".into(),
            Self::ATilde { rank } => format!("Ã{rank}"),
            Self::DTilde { rank } => format!("D̃{rank}"),
            Self::ETilde { rank } => format!("Ẽ{rank}"),
        }
    }

    /// The underlying graph: vertex count and edges (u, v) with u < v.
    fn graph(&self) -> Result<(usize, Vec<(usize, usize)>), String> {
        Ok(match *self {
            Self::Kronecker => (2, vec![(0, 1)]),
            Self::ATilde { rank } if rank >= 2 => {
                let n = rank + 1;
                let mut e: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
                e.push((0, n - 1));
                (n, e)
            }
            Self::DTilde { rank: 4 } => (5, (1..5).map(|i| (0, i)).collect()),
            Self::DTilde { rank } if rank >= 5 => {
                let n = rank + 1;
                let spine = n - 4; // the path between the two branch points
                let mut e: Vec<(usize, usize)> = (0..spine - 1).map(|i| (i, i + 1)).collect();
                e.extend([(0, spine), (0, spine + 1), (spine - 1, spine + 2), (spine - 1, spine + 3)]);
                (n, e)
            }
            Self::ETilde { rank: 6 } => arms(&[2, 2, 2]),
            Self::ETilde { rank: 7 } => arms(&[1, 3, 3]),
            Self::ETilde { rank: 8 } => arms(&[1, 2, 5]),
            Self::ATilde { .. } => return Err("Ã needs rank ≥ 2 (Ã1 is the Kronecker quiver)".into()),
            Self::DTilde { .. } => return Err("D̃ needs rank ≥ 4".into()),
            Self::ETilde { .. } => return Err("Ẽ exists only in ranks 6, 7 and 8".into()),
        })
    }

    /// The orientations to explore, deduplicated up to relabelling (and up to
    /// reversing all arrows, if `with_opposite`), in a fixed order.
    pub fn orientations(&self, which: Orientations, with_opposite: bool) -> Result<Vec<Exchange>, String> {
        let (n, edges) = self.graph()?;
        let m = if *self == Self::Kronecker { 2 } else { 1 };
        let masks: Vec<u64> = match which {
            Orientations::All => (0..1u64 << edges.len()).collect(),
            Orientations::One => vec![0],
        };
        let mut seen = std::collections::BTreeMap::new();
        for mask in masks {
            let mut q = Exchange::zero(n);
            for (t, &(u, v)) in edges.iter().enumerate() {
                q.set_pair(u, v, if mask >> t & 1 == 0 { m } else { -m });
            }
            if is_acyclic(&q) {
                seen.entry(canonical_key(&q, with_opposite)).or_insert_with(|| if which == Orientations::One { q } else { canonical(&q) });
            }
        }
        Ok(seen.into_values().collect())
    }
}

/// A star with arms of the given lengths: centre 0, then each arm outwards.
fn arms(lengths: &[usize]) -> (usize, Vec<(usize, usize)>) {
    let mut e = Vec::new();
    let mut next = 1;
    for &len in lengths {
        let mut prev = 0;
        for _ in 0..len {
            e.push((prev, next));
            (prev, next) = (next, next + 1);
        }
    }
    (next, e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::{classify, Kind};

    #[test]
    fn every_orientation_has_the_family_type() {
        for (f, count) in
            [(Family::Kronecker, 1), (Family::ATilde { rank: 2 }, 1), (Family::ATilde { rank: 3 }, 3), (Family::DTilde { rank: 4 }, 5)]
        {
            let qs = f.orientations(Orientations::All, false).unwrap();
            assert_eq!(qs.len(), count, "{}", f.name());
            assert!(qs.iter().all(|q| classify(q).is_tame()));
        }
        for f in [Family::DTilde { rank: 5 }, Family::ETilde { rank: 6 }, Family::ETilde { rank: 7 }, Family::ETilde { rank: 8 }] {
            let q = &f.orientations(Orientations::One, false).unwrap()[0];
            assert!(matches!(classify(q), Kind::Euclidean { .. }), "{}", f.name());
            assert_eq!(
                q.n(),
                match f {
                    Family::DTilde { rank } | Family::ETilde { rank } => rank + 1,
                    _ => 0,
                }
            );
        }
        assert!(Family::ETilde { rank: 5 }.orientations(Orientations::One, false).is_err());
    }

    #[test]
    fn identifying_opposites() {
        // D̃4: k arrows into the centre, k = 0..4; reversing swaps k and 4 − k.
        assert_eq!(Family::DTilde { rank: 4 }.orientations(Orientations::All, true).unwrap().len(), 3);
        // The square's three orientations, Ã(3,1) and the two Ã(2,2)s, are each their own opposite.
        assert_eq!(Family::ATilde { rank: 3 }.orientations(Orientations::All, true).unwrap().len(), 3);
    }
}
