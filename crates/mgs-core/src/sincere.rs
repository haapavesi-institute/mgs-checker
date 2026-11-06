//! Sincere step c-vectors, and a survey of the inheritance conjecture.
//!
//! S(Q) is the set of c-vectors at the steps of Q's maximal green sequences,
//! and Sinc(Q) the sincere ones (positive at every vertex). Keeping the steps
//! whose c-vector vanishes at v gives an MGS of Q ∖ v (Muller,
//! arXiv:1503.04675; it is the lattice quotient of Demonet–Iyama–Reading–
//! Reiten–Thomas, arXiv:1711.01785). So every c ∈ S(Q) is a sincere step
//! c-vector of the full subquiver on its support, and Q has finitely many MGS
//! once every connected full subquiver has finitely many sincere ones.
//!
//! **Inheritance conjecture.** If a connected acyclic quiver has no sincere
//! step c-vector, neither does any connected acyclic quiver containing it as
//! a full subquiver. Igusa–Zhou's Theorem 3.12 is the case of a multiple
//! arrow. Adding one vertex at a time, it is enough to check: if Q ∖ v is
//! connected and Sinc(Q ∖ v) is empty, then Sinc(Q) is empty.
//!
//! The survey checks that for every quiver of a family, lists the *minimal*
//! quivers without a sincere c-vector (every connected Q ∖ v has one), says
//! which of them are the known patterns of [`crate::patterns`], and flags any
//! known pattern that a step c-vector covers.

use serde::{Deserialize, Serialize};

use crate::canonical::canonical_key;
use crate::classify::{classify, components};
use crate::engine::{search, SearchOptions, Stop};
use crate::enumerate::acyclic_quivers;
use crate::framed::Framed;
use crate::known::facts;
use crate::matrix::Exchange;
use crate::patterns::patterns;

/// The step c-vectors of q, with what the search behind them established.
pub struct Steps {
    pub vectors: Vec<Vec<i64>>,
    /// The search finished (possibly on the gap assumption).
    pub settled: bool,
    /// It finished with no unproved assumption.
    pub proved: bool,
    /// Number of MGS, in decimal.
    pub total: String,
}

pub fn steps(q: &Exchange, opts: &SearchOptions) -> Steps {
    let (engine, out) = search(q, opts);
    let exhausted = out.stop == Stop::Exhausted;
    let gap = out.stop == Stop::Gap;
    Steps {
        vectors: engine.step_c_vectors(&Framed::new(q), out.searched_to),
        settled: exhausted || gap,
        proved: exhausted || (gap && facts(q).no_gap.is_some()),
        total: out.root.map_or_else(|| "0".into(), |r| r.counts.total_within(out.searched_to).to_string()),
    }
}

pub fn is_sincere(c: &[i64]) -> bool {
    c.iter().all(|&x| x > 0)
}

/// Which quivers a survey runs over.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    /// Every connected acyclic quiver.
    #[default]
    All,
    /// Trees: n − 1 edges.
    Trees,
    /// One cycle in the underlying graph: n edges.
    OneCycle,
    /// The bare cycles of length n, in every orientation, single arrows.
    Cycles,
    /// The transitive tournament: i → j for every i < j.
    Tournament,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Spec {
    pub min_vertices: usize,
    pub max_vertices: usize,
    /// Ignored by `cycles` and `tournament`, which have single arrows.
    pub max_multiplicity: i64,
    pub family: Family,
}

impl Default for Spec {
    fn default() -> Self {
        Self { min_vertices: 2, max_vertices: 5, max_multiplicity: 1, family: Family::All }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: usize,
    pub matrix: Vec<Vec<i64>>,
}

fn family(n: usize, spec: &Spec) -> Result<Vec<Exchange>, String> {
    let edges = |q: &Exchange| q.arrows().len();
    Ok(match spec.family {
        Family::All => acyclic_quivers(n, spec.max_multiplicity, |_| true)?,
        Family::Trees => acyclic_quivers(n, spec.max_multiplicity, |q| edges(q) + 1 == n)?,
        Family::OneCycle => acyclic_quivers(n, spec.max_multiplicity, |q| edges(q) == n)?,
        Family::Tournament => {
            let mut q = Exchange::zero(n);
            for i in 0..n {
                for j in i + 1..n {
                    q.set_pair(i, j, 1);
                }
            }
            vec![q]
        }
        Family::Cycles if !(3..=16).contains(&n) => return Err("a cycle needs between 3 and 16 vertices".into()),
        Family::Cycles => {
            let mut seen = std::collections::HashSet::new();
            let mut out = Vec::new();
            for mask in 1u32..(1 << n) - 1 {
                let mut q = Exchange::zero(n);
                for i in 0..n {
                    let j = (i + 1) % n;
                    if mask >> i & 1 == 1 {
                        q.set_pair(i, j, 1);
                    } else {
                        q.set_pair(j, i, 1);
                    }
                }
                if seen.insert(canonical_key(&q, true)) {
                    out.push(q);
                }
            }
            out
        }
    })
}

/// The quivers to survey: by vertex count, then in enumeration order.
pub fn plan(spec: &Spec) -> Result<Vec<Item>, String> {
    if spec.min_vertices < 1 || spec.min_vertices > spec.max_vertices {
        return Err("the vertex counts must satisfy 1 ≤ min ≤ max".into());
    }
    let mut items = Vec::new();
    for n in spec.min_vertices..=spec.max_vertices {
        if spec.family == Family::Cycles && n < 3 {
            continue;
        }
        items.extend(family(n, spec)?.into_iter().map(|q| q.rows()));
    }
    Ok(items.into_iter().enumerate().map(|(id, matrix)| Item { id, matrix }).collect())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Row {
    pub id: usize,
    pub vertices: usize,
    pub arrows: String,
    pub kind: String,
    /// The search for Q and for every connected Q ∖ v finished.
    pub settled: bool,
    /// With no unproved assumption (for Q itself).
    pub proved: bool,
    pub total: String,
    pub step_c_vectors: usize,
    pub sincere: Vec<Vec<i64>>,
    /// The known patterns in Q, in words.
    pub patterns: Vec<String>,
    /// Known patterns that some step c-vector covers: counterexamples to the covering statement.
    pub covered: Vec<String>,
    /// The vertices v (1-based) with Q ∖ v connected and Sinc(Q ∖ v) empty.
    pub bare_below: Vec<usize>,
    /// Sinc(Q) is not empty although some `bare_below` exists: a counterexample to inheritance.
    pub inheritance_fails: bool,
    /// Sinc(Q) is empty and every connected Q ∖ v has a sincere step c-vector.
    pub minimal: bool,
    /// A minimal quiver that is itself one of the known patterns.
    pub known_minimal: bool,
}

/// Surveys one quiver: its sincere step c-vectors against those of each connected Q ∖ v.
pub fn run(item: &Item, opts: &SearchOptions) -> Result<Row, String> {
    let q = Exchange::from_rows(item.matrix.clone()).map_err(|e| e.to_string())?;
    let n = q.n();
    let own = steps(&q, opts);
    let sincere: Vec<Vec<i64>> = own.vectors.iter().filter(|c| is_sincere(c)).cloned().collect();
    let known = patterns(&q);
    let (mut settled, mut bare_below, mut all_below_sincere) = (own.settled, Vec::new(), true);
    for v in 0..n {
        let keep: Vec<usize> = (0..n).filter(|&u| u != v).collect();
        if keep.is_empty() {
            continue;
        }
        let part =
            Exchange::from_rows(keep.iter().map(|&i| keep.iter().map(|&j| q.get(i, j)).collect()).collect()).map_err(|e| e.to_string())?;
        if components(&part).len() != 1 {
            continue;
        }
        let below = steps(&part, opts);
        settled &= below.settled;
        if !below.vectors.iter().any(|c| is_sincere(c)) {
            all_below_sincere = false;
            bare_below.push(v + 1);
        }
    }
    let minimal = settled && sincere.is_empty() && all_below_sincere;
    Ok(Row {
        id: item.id,
        vertices: n,
        arrows: q.arrow_text(),
        kind: classify(&q).name(),
        settled,
        proved: own.proved,
        total: own.total,
        step_c_vectors: own.vectors.len(),
        patterns: known.iter().map(|p| p.describe()).collect(),
        covered: known.iter().filter(|p| own.vectors.iter().any(|c| p.covered_by(c))).map(|p| p.describe()).collect(),
        inheritance_fails: settled && !sincere.is_empty() && !bare_below.is_empty(),
        known_minimal: minimal && known.iter().any(|p| p.vertices().len() == n),
        minimal,
        bare_below,
        sincere,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(text: &str) -> Row {
        let q = Exchange::from_arrow_text(text, None).unwrap();
        run(&Item { id: 0, matrix: q.rows() }, &SearchOptions::default()).unwrap()
    }

    #[test]
    fn the_known_minimal_quivers() {
        for text in ["1->2*2", "1->2*3", "1->2, 2->3, 1->3", "1->2, 2->3, 3->4, 1->4", "1->3, 1->4, 2->3, 2->4"] {
            let r = row(text);
            assert!(r.minimal && r.known_minimal && r.sincere.is_empty() && r.covered.is_empty(), "{text}: {r:?}");
        }
        // The one-source square has two: the projective at its source and the injective at its sink.
        let square = row("1->2, 2->4, 1->3, 3->4");
        assert_eq!(square.sincere, vec![vec![1, 1, 1, 2], vec![2, 1, 1, 1]]);
        assert!(!square.minimal && !square.inheritance_fails && square.bare_below.is_empty());
    }

    #[test]
    fn inheritance_on_an_extension_and_a_dynkin_quiver() {
        // Ã(2,1) with a tail: Q ∖ 4 is the triangle, which has no sincere c-vector, and neither has Q.
        let r = row("1->2, 2->3, 1->3, 3->4");
        assert_eq!((r.bare_below.clone(), r.sincere.len(), r.inheritance_fails, r.minimal), (vec![4], 0, false, false));
        let d4 = row("1->2, 3->2, 4->2");
        assert_eq!((d4.sincere.len(), d4.minimal, d4.proved), (2, false, true));
    }

    #[test]
    fn families() {
        let count = |family, n| plan(&Spec { min_vertices: n, max_vertices: n, max_multiplicity: 1, family }).unwrap().len();
        assert_eq!((count(Family::All, 4), count(Family::Trees, 4), count(Family::OneCycle, 4)), (24, 8, 9));
        // Cycles up to rotation, reflection and reversing every arrow: 1, 3, 3, 7 for lengths 3 to 6.
        assert_eq!([3, 4, 5, 6].map(|n| count(Family::Cycles, n)), [1, 3, 3, 7]);
        assert_eq!(count(Family::Tournament, 5), 1);
    }
}
