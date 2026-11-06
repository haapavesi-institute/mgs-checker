//! Fixed explorations around tame quivers: start from the chosen families'
//! acyclic orientations, apply one-edge changes `steps` times, and check
//! every quiver reached (each once, up to relabelling). Changes that would
//! close an oriented cycle are never made, so every quiver is acyclic.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::canonical::canonical_key;
use crate::families::{Family, Orientations};
use crate::matrix::Exchange;
use crate::modify::{changes, Mods};
use crate::report::{check, CheckOptions};
use crate::walk::BanReason;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExploreSpec {
    pub families: Vec<Family>,
    pub orientations: Orientations,
    /// Check the tame starting quivers too, as a baseline.
    pub include_base: bool,
    pub mods: Mods,
    /// How many one-edge changes to apply (1 or 2).
    pub steps: u32,
    /// Treat Q and Q^op as one quiver (they have the same MGS counts by length, BDP Prop. 2.15).
    pub up_to_opposite: bool,
}

impl Default for ExploreSpec {
    fn default() -> Self {
        Self {
            families: vec![Family::Kronecker, Family::ATilde { rank: 2 }, Family::ATilde { rank: 3 }],
            orientations: Orientations::All,
            include_base: true,
            mods: Mods::default(),
            steps: 1,
            up_to_opposite: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: usize,
    /// The tame quiver this came from, e.g. "Ã2 #1: 1->2, 1->3, 2->3".
    pub base: String,
    /// The changes applied, e.g. "new 1->3; 2->3 ×1 to ×2", or "" for the base itself.
    pub change: String,
    pub parent: Option<usize>,
    pub matrix: Vec<Vec<i64>>,
}

/// The quivers an exploration will check, in order. Fails on an impossible family.
pub fn plan(spec: &ExploreSpec) -> Result<Vec<Item>, String> {
    if spec.steps > 2 {
        return Err("steps must be 0, 1 or 2".into());
    }
    let mut seen = HashSet::new();
    let mut items: Vec<(Item, Exchange)> = Vec::new();
    let mut layer = Vec::new();
    for f in &spec.families {
        for (t, q) in f.orientations(spec.orientations, spec.up_to_opposite)?.into_iter().enumerate() {
            if seen.insert(canonical_key(&q, spec.up_to_opposite)) {
                let base = format!("{} #{}: {}", f.name(), t + 1, q.arrow_text());
                layer.push((base, String::new(), None, q));
            }
        }
    }
    for step in 0..=spec.steps {
        let mut next = Vec::new();
        for (base, change, parent, q) in layer {
            let id = items.len();
            if step < spec.steps {
                for c in changes(&q, spec.mods) {
                    if seen.insert(canonical_key(&c.quiver, spec.up_to_opposite)) {
                        let label = if change.is_empty() { c.label } else { format!("{change}; {}", c.label) };
                        next.push((base.clone(), label, Some(id), c.quiver));
                    }
                }
            }
            items.push((Item { id, base, change, parent, matrix: q.rows() }, q));
        }
        layer = next;
    }
    let keep_base = spec.include_base;
    Ok(items.into_iter().map(|(i, _)| i).filter(|i| keep_base || i.parent.is_some()).collect())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Row {
    pub id: usize,
    pub parent: Option<usize>,
    pub base: String,
    pub change: String,
    pub arrows: String,
    pub kind: String,
    pub status: String,
    /// Settled with no unproved assumption.
    pub proved: bool,
    pub settled: bool,
    pub searched_to: u32,
    pub shortest: Option<u32>,
    pub longest: Option<u32>,
    /// The literature's longest MGS, for tame and Dynkin quivers.
    pub predicted_longest: Option<u32>,
    pub total: String,
    pub by_length: String,
    pub states: usize,
    /// Exchange-graph classes the MGS pass through.
    pub classes_on_mgs: Option<usize>,
    /// Distinct c-vectors at the steps of the MGS.
    pub step_c_vectors: Option<usize>,
    /// The maximal step c-vectors under the entrywise order, e.g. "(1,2,1) (2,1,1)".
    pub maximal_c: String,
    /// The largest entry of any step c-vector.
    pub largest_c_entry: Option<i64>,
    /// The most parallel arrows in any exchange matrix along an MGS.
    pub most_arrows: Option<i64>,
    /// First moves that can never be finished, e.g. "2 (BHIT), 3 (tentative)".
    pub banned_first: String,
}

/// Checks one planned quiver.
pub fn run(item: &Item, opts: &CheckOptions) -> Result<Row, String> {
    let q = Exchange::from_rows(item.matrix.clone()).map_err(|e| e.to_string())?;
    let r = check(&q, &CheckOptions { sample: 0, banned_depth: 1, ..opts.clone() }).map_err(|e| e.to_string())?;
    let seeds = r.seeds.as_ref();
    let vector = |v: &[i64]| format!("({})", v.iter().map(i64::to_string).collect::<Vec<_>>().join(","));
    let banned_first = r
        .banned
        .iter()
        .map(|b| {
            let why = match b.reason {
                BanReason::Bhit332 { .. } => "BHIT",
                BanReason::DeadEnd { .. } => "dead end",
                BanReason::LongerMgs { .. } => "GAP FAILS",
                BanReason::NoGapTheorem => "No Gap thm",
                BanReason::Tentative { .. } => "tentative",
                BanReason::Unresolved => "unresolved",
            };
            format!("{} ({why})", b.prefix[0])
        })
        .collect::<Vec<_>>()
        .join(", ");
    Ok(Row {
        id: item.id,
        parent: item.parent,
        base: item.base.clone(),
        change: item.change.clone(),
        arrows: r.arrows,
        kind: r.facts.kind_name.clone(),
        status: r.status.explain(r.searched_to),
        proved: r.status.proved(),
        settled: r.status.settled(),
        searched_to: r.searched_to,
        shortest: r.shortest,
        longest: r.longest,
        predicted_longest: r.facts.longest.as_ref().map(|f| f.value),
        total: r.total,
        by_length: r.by_length.iter().map(|c| format!("{}:{}", c.length, c.count)).collect::<Vec<_>>().join(" "),
        states: r.states,
        classes_on_mgs: seeds.map(|s| s.classes),
        step_c_vectors: seeds.map(|s| s.step_c_vectors),
        maximal_c: seeds.map_or(String::new(), |s| s.maximal.iter().map(|m| vector(&m.vector)).collect::<Vec<_>>().join(" ")),
        largest_c_entry: seeds.map(|s| s.largest_entry),
        most_arrows: seeds.map(|s| s.most_arrows),
        banned_first,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kronecker_neighbourhood() {
        let spec = ExploreSpec { families: vec![Family::Kronecker], ..ExploreSpec::default() };
        let items = plan(&spec).unwrap();
        assert_eq!(items.len(), 2); // 1 => 2, and 1 =>(3) 2
        let rows: Vec<Row> = items.iter().map(|i| run(i, &CheckOptions::default()).unwrap()).collect();
        assert_eq!((rows[0].total.as_str(), rows[0].proved), ("1", true));
        assert_eq!(rows[1].change, "2->1 ×2 to ×3"); // the canonical labelling of 1 => 2 is 2 => 1
        assert_eq!(rows[1].total, "1");
        assert_eq!(rows[1].banned_first, "1 (BHIT)"); // 1 is the target of the triple arrow from 2
        assert_eq!((rows[0].maximal_c.as_str(), rows[0].step_c_vectors), ("(1,0) (0,1)", Some(2)));
    }

    #[test]
    fn no_planned_quiver_has_an_oriented_cycle() {
        let spec =
            ExploreSpec { families: vec![Family::ATilde { rank: 2 }, Family::ATilde { rank: 3 }], steps: 2, ..ExploreSpec::default() };
        for item in plan(&spec).unwrap() {
            let q = Exchange::from_rows(item.matrix).unwrap();
            assert!(crate::classify::is_acyclic(&q), "{} {}: {}", item.base, item.change, q.arrow_text());
        }
    }

    #[test]
    fn two_steps_reach_more_and_never_repeat() {
        let one = plan(&ExploreSpec { families: vec![Family::ATilde { rank: 2 }], ..ExploreSpec::default() }).unwrap();
        let two = plan(&ExploreSpec { families: vec![Family::ATilde { rank: 2 }], steps: 2, ..ExploreSpec::default() }).unwrap();
        assert!(two.len() > one.len());
        let keys: HashSet<Vec<i64>> = two.iter().map(|i| canonical_key(&Exchange::from_rows(i.matrix.clone()).unwrap(), false)).collect();
        assert_eq!(keys.len(), two.len());
    }
}
