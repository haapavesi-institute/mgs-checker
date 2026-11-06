//! A survey of conjecture #7 (see [`crate::cutting`]) over small connected
//! quivers with a multiple arrow: plan the quivers, then check each one.
//!
//! Three scopes:
//! - `acyclic`: Igusa–Zhou's Theorem 3.12 covers these, so a failure would mean a bug;
//! - `open`: an oriented cycle, but none through a multiple arrow. This is
//!   the open part of the conjecture, and the only place outside `cutting`
//!   where this checker accepts an oriented cycle;
//! - `control`: a multiple arrow on an oriented cycle. The conjecture claims
//!   nothing there, and it is known to fail (BHIT's Figure 3).

use serde::{Deserialize, Serialize};

use crate::acyclic::oriented_cycle;
use crate::classify::is_acyclic;
use crate::cutting::{iz_cut, multiple_on_cycle, multiple_on_oriented_cycle, CutFailure, CutVerdict};
use crate::enumerate::{acyclic_quivers, all_quivers, sampled_quivers};
use crate::matrix::Exchange;
use crate::report::{check_any, CheckOptions};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    #[default]
    Acyclic,
    Open,
    Control,
}

impl Scope {
    /// The scope q belongs to (q has a multiple arrow).
    pub fn of(q: &Exchange) -> Self {
        if is_acyclic(q) {
            Self::Acyclic
        } else if multiple_on_oriented_cycle(q) {
            Self::Control
        } else {
            Self::Open
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Acyclic => "acyclic",
            Self::Open => "open",
            Self::Control => "control",
        }
    }
}

/// Which quivers with a multiple arrow to check, within the scope.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Class {
    /// Every one.
    #[default]
    All,
    /// A multiple arrow lies on a cycle of the underlying graph that is not an oriented cycle (as in Ã_n).
    OnCycle,
    /// That, and no Igusa–Zhou partition cut (their Theorem 4.3) exists.
    OnCycleNoCut,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Spec {
    pub min_vertices: usize,
    pub max_vertices: usize,
    pub max_multiplicity: i64,
    pub scope: Scope,
    pub class: Class,
    /// Draw this many random matrices per vertex count instead of taking every one.
    pub sample: Option<u64>,
    pub seed: u64,
}

impl Default for Spec {
    fn default() -> Self {
        Self { min_vertices: 2, max_vertices: 4, max_multiplicity: 2, scope: Scope::Acyclic, class: Class::All, sample: None, seed: 1 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: usize,
    pub matrix: Vec<Vec<i64>>,
}

fn has_multiple(q: &Exchange) -> bool {
    q.entries().iter().any(|&b| b >= 2)
}

/// The quivers to check: by vertex count, then in enumeration order.
pub fn plan(spec: &Spec) -> Result<Vec<Item>, String> {
    if spec.min_vertices < 2 || spec.min_vertices > spec.max_vertices {
        return Err("the vertex counts must satisfy 2 ≤ min ≤ max".into());
    }
    if spec.max_multiplicity < 2 {
        return Err("the largest multiplicity must be at least 2, or there is no multiple arrow to delete".into());
    }
    let wanted = |q: &Exchange| {
        has_multiple(q)
            && Scope::of(q) == spec.scope
            && match spec.class {
                Class::All => true,
                Class::OnCycle => multiple_on_cycle(q),
                Class::OnCycleNoCut => multiple_on_cycle(q) && iz_cut(q).is_none(),
            }
    };
    let mut items = Vec::new();
    for n in spec.min_vertices..=spec.max_vertices {
        let m = spec.max_multiplicity;
        let quivers = match (spec.sample, spec.scope) {
            (Some(count), _) => sampled_quivers(n, m, count, spec.seed.wrapping_add(n as u64), wanted)?,
            (None, Scope::Acyclic) => acyclic_quivers(n, m, wanted)?,
            (None, _) => all_quivers(n, m, wanted)?,
        };
        items.extend(quivers.into_iter().map(|q| q.rows()));
    }
    Ok(items.into_iter().enumerate().map(|(id, matrix)| Item { id, matrix }).collect())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Row {
    pub id: usize,
    pub vertices: usize,
    pub arrows: String,
    pub kind: String,
    /// "acyclic", "open" or "control" (see [`Scope`]).
    pub scope: &'static str,
    /// An oriented cycle of Q, e.g. "2→4→3→2", or "".
    pub oriented_cycle: String,
    pub on_cycle: bool,
    /// The least k of an Igusa–Zhou cut, if there is one.
    pub iz_cut: Option<i64>,
    pub status: String,
    pub settled: bool,
    pub proved: bool,
    pub total: String,
    pub longest: Option<u32>,
    /// "holds", "fails", "no MGS", "unsettled" or "unfinished".
    pub verdict: String,
    /// For a failure: the shortest failing prefix and what went wrong.
    pub detail: String,
    /// No step c-vector has both ends of a multiple arrow in its support.
    pub labels_avoid_multiple: Option<bool>,
    pub q_prime: String,
    pub joint_states: usize,
}

impl Row {
    /// A failure in the conjecture's hypothesis: no multiple arrow on an oriented cycle.
    pub fn counterexample(&self) -> bool {
        self.verdict == "fails" && self.scope != "control"
    }
}

fn vector(v: &[i64]) -> String {
    format!("({})", v.iter().map(i64::to_string).collect::<Vec<_>>().join(","))
}

/// Checks one planned quiver.
pub fn run(item: &Item, opts: &CheckOptions) -> Result<Row, String> {
    check_one(&Exchange::from_rows(item.matrix.clone()).map_err(|e| e.to_string())?, item.id, opts)
}

/// Checks one quiver with a multiple arrow, in whichever scope it falls.
pub fn check_one(q: &Exchange, id: usize, opts: &CheckOptions) -> Result<Row, String> {
    if !has_multiple(q) {
        return Err("the quiver has no multiple arrow, so deleting them changes nothing".into());
    }
    let r = check_any(q, &CheckOptions { sample: 0, banned_depth: 0, ..opts.clone() });
    let cut = r.cut.as_ref();
    let (verdict, detail) = match cut.map(|c| &c.verdict) {
        Some(CutVerdict::Fails { prefix, failure }) => {
            let seq = prefix.iter().map(usize::to_string).collect::<Vec<_>>().join(",");
            let what = match failure {
                CutFailure::RedInPrime { c_in_q, c_in_prime } => {
                    format!("its last step is red in Q′: c = {} in Q, {} in Q′", vector(c_in_q), vector(c_in_prime))
                }
                CutFailure::DifferentCVector { c_in_q, c_in_prime } => {
                    format!("its last step has c = {} in Q but {} in Q′", vector(c_in_q), vector(c_in_prime))
                }
                CutFailure::NotMaximalInPrime { still_green } => {
                    format!("it is an MGS of Q, but in Q′ these are still green: {still_green:?}")
                }
            };
            ("fails".to_string(), format!("({seq}): {what}"))
        }
        Some(CutVerdict::Unfinished) => ("unfinished".into(), "the walk ran out of its state budget".into()),
        Some(CutVerdict::Holds) if r.status.settled() => ("holds".into(), String::new()),
        Some(CutVerdict::Holds) => ("unsettled".into(), format!("holds on every MGS of length ≤ {}", r.searched_to)),
        None if r.status.settled() => ("no MGS".into(), String::new()),
        None => ("unsettled".into(), "no MGS found yet".into()),
    };
    let cycle = oriented_cycle(q).map_or(String::new(), |c| c.iter().map(|v| (v + 1).to_string()).collect::<Vec<_>>().join("→"));
    Ok(Row {
        id,
        vertices: q.n(),
        arrows: r.arrows.clone(),
        kind: r.facts.kind_name.clone(),
        scope: Scope::of(q).name(),
        oriented_cycle: cycle,
        on_cycle: multiple_on_cycle(q),
        iz_cut: iz_cut(q),
        status: r.explanation.clone(),
        settled: r.status.settled(),
        proved: r.status.proved(),
        total: r.total.clone(),
        longest: r.longest,
        verdict,
        detail,
        labels_avoid_multiple: cut.map(|c| c.label_covers_multiple.is_none()),
        q_prime: cut.map_or(String::new(), |c| c.q_prime.clone()),
        joint_states: cut.map_or(0, |c| c.joint_states),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(text: &str) -> Row {
        check_one(&Exchange::from_arrow_text(text, None).unwrap(), 0, &CheckOptions::default()).unwrap()
    }

    #[test]
    fn three_vertices_all_hold() {
        let items = plan(&Spec { max_vertices: 3, ..Spec::default() }).unwrap();
        let rows: Vec<Row> = items.iter().map(|i| run(i, &CheckOptions::default()).unwrap()).collect();
        assert_eq!(rows[0].arrows, "1->2*2"); // the Kronecker quiver comes first
        assert!(rows.iter().all(|r| r.verdict == "holds" && r.scope == "acyclic"), "{rows:#?}");
        assert!(rows.iter().all(|r| r.labels_avoid_multiple == Some(true)));
    }

    #[test]
    fn the_board_example_holds_and_the_controls_fail() {
        // Conjecture #7's example: the oriented cycle 2 → 4 → 3 → 2 avoids 1 ⇉ 2. It has 23 MGS.
        let ex = one("1->2*2, 1->3, 3->2, 2->4, 4->3");
        assert_eq!((ex.scope, ex.verdict.as_str(), ex.total.as_str()), ("open", "holds", "23"));
        assert_eq!(ex.oriented_cycle, "2→4→3→2");
        // BHIT's Figure 3, and the post's other control: every MGS fails on Q′.
        for text in ["1->2, 2->3, 3->1*2", "1->2*2, 2->3, 3->1, 2->4, 1->4"] {
            let r = one(text);
            assert_eq!((r.scope, r.verdict.as_str()), ("control", "fails"), "{text}");
            assert!(!r.counterexample());
        }
        assert!(check_one(&Exchange::from_arrow_text("1->2, 2->3", None).unwrap(), 0, &CheckOptions::default()).is_err());
    }

    #[test]
    fn scopes_partition_the_four_vertex_survey_of_the_board_post() {
        // The board post's four-vertex controls (multiplicities ≤ 2, a multiple arrow on a
        // cycle, no Igusa–Zhou cut): 125 acyclic, 8 open, 99 with a multiple arrow on an oriented cycle.
        let count =
            |scope| plan(&Spec { min_vertices: 4, max_vertices: 4, scope, class: Class::OnCycleNoCut, ..Spec::default() }).unwrap().len();
        assert_eq!((count(Scope::Acyclic), count(Scope::Open), count(Scope::Control)), (125, 8, 99));
    }
}
