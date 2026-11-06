//! A survey of conjecture #7 (see [`crate::cutting`]) over every small
//! connected acyclic quiver with a multiple arrow: plan the quivers, then
//! check each one. On acyclic quivers the statement is Igusa–Zhou's
//! Theorem 3.12, so a failure here would mean a bug.

use serde::{Deserialize, Serialize};

use crate::cutting::{iz_cut, multiple_on_cycle, CutFailure, CutVerdict};
use crate::enumerate::acyclic_quivers;
use crate::matrix::Exchange;
use crate::report::{check, CheckOptions};

/// Which quivers with a multiple arrow to check.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Class {
    /// Every one.
    #[default]
    All,
    /// A multiple arrow lies on a cycle of the underlying graph (as in Ã_n).
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
    pub class: Class,
}

impl Default for Spec {
    fn default() -> Self {
        Self { min_vertices: 2, max_vertices: 4, max_multiplicity: 2, class: Class::All }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: usize,
    pub matrix: Vec<Vec<i64>>,
}

/// The quivers to check: by vertex count, then in enumeration order.
pub fn plan(spec: &Spec) -> Result<Vec<Item>, String> {
    if spec.min_vertices < 2 || spec.min_vertices > spec.max_vertices {
        return Err("the vertex counts must satisfy 2 ≤ min ≤ max".into());
    }
    if spec.max_multiplicity < 2 {
        return Err("the largest multiplicity must be at least 2, or there is no multiple arrow to delete".into());
    }
    let wanted = |q: &Exchange| match spec.class {
        Class::All => true,
        Class::OnCycle => multiple_on_cycle(q),
        Class::OnCycleNoCut => multiple_on_cycle(q) && iz_cut(q).is_none(),
    };
    let mut items = Vec::new();
    for n in spec.min_vertices..=spec.max_vertices {
        let has_multiple = |q: &Exchange| q.entries().iter().any(|&b| b >= 2);
        for q in acyclic_quivers(n, spec.max_multiplicity, has_multiple)? {
            if wanted(&q) {
                items.push(Item { id: items.len(), matrix: q.rows() });
            }
        }
    }
    Ok(items)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Row {
    pub id: usize,
    pub vertices: usize,
    pub arrows: String,
    pub kind: String,
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

fn vector(v: &[i64]) -> String {
    format!("({})", v.iter().map(i64::to_string).collect::<Vec<_>>().join(","))
}

/// Checks one planned quiver.
pub fn run(item: &Item, opts: &CheckOptions) -> Result<Row, String> {
    let q = Exchange::from_rows(item.matrix.clone()).map_err(|e| e.to_string())?;
    let r = check(&q, &CheckOptions { sample: 0, banned_depth: 0, ..opts.clone() }).map_err(|e| e.to_string())?;
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
    Ok(Row {
        id: item.id,
        vertices: q.n(),
        arrows: r.arrows.clone(),
        kind: r.facts.kind_name.clone(),
        on_cycle: multiple_on_cycle(&q),
        iz_cut: iz_cut(&q),
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

    #[test]
    fn three_vertices_all_hold() {
        let items = plan(&Spec { max_vertices: 3, ..Spec::default() }).unwrap();
        let rows: Vec<Row> = items.iter().map(|i| run(i, &CheckOptions::default()).unwrap()).collect();
        assert_eq!(rows[0].arrows, "1->2*2"); // the Kronecker quiver comes first
        assert!(rows.iter().all(|r| r.verdict == "holds"), "{rows:#?}");
        assert!(rows.iter().all(|r| r.labels_avoid_multiple == Some(true)));
        let triangles = plan(&Spec { min_vertices: 3, max_vertices: 3, class: Class::OnCycleNoCut, ..Spec::default() }).unwrap();
        assert!(!triangles.is_empty());
        for t in &triangles {
            let q = Exchange::from_rows(t.matrix.clone()).unwrap();
            assert!(multiple_on_cycle(&q) && iz_cut(&q).is_none());
        }
    }
}
