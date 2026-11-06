//! One-edge changes to a quiver: raise the multiplicity of an existing arrow
//! bundle by one, or add a single arrow between two vertices that had none.
//! A new arrow that would close an oriented cycle is left out, since the
//! checker accepts only acyclic quivers.

use serde::{Deserialize, Serialize};

use crate::classify::is_acyclic;
use crate::matrix::Exchange;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Mods {
    /// i → j with m arrows becomes m + 1.
    pub multiplicity: bool,
    /// A single arrow, in each direction, between two vertices with no arrows between them.
    pub new_edge: bool,
}

impl Default for Mods {
    fn default() -> Self {
        Self { multiplicity: true, new_edge: true }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub label: String,
    pub quiver: Exchange,
}

/// Every one-edge change allowed by `mods`, in a fixed order.
pub fn changes(q: &Exchange, mods: Mods) -> Vec<Change> {
    let n = q.n();
    let mut out = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            let b = q.get(i, j);
            if b != 0 && mods.multiplicity {
                let (from, to, m) = if b > 0 { (i, j, b) } else { (j, i, -b) };
                let mut r = q.clone();
                r.set_pair(from, to, m + 1);
                out.push(Change { label: format!("{}->{} ×{m} to ×{}", from + 1, to + 1, m + 1), quiver: r });
            }
            if b == 0 && mods.new_edge {
                for (from, to) in [(i, j), (j, i)] {
                    let mut r = q.clone();
                    r.set_pair(from, to, 1);
                    if is_acyclic(&r) {
                        out.push(Change { label: format!("new {}->{}", from + 1, to + 1), quiver: r });
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_of_a_path() {
        let q = Exchange::from_arrow_text("1->2, 2->3", None).unwrap();
        let all: Vec<(String, String)> = changes(&q, Mods::default()).into_iter().map(|c| (c.label, c.quiver.arrow_text())).collect();
        assert_eq!(
            all,
            vec![
                ("1->2 ×1 to ×2".into(), "1->2*2, 2->3".into()),
                ("new 1->3".into(), "1->2, 1->3, 2->3".into()),
                ("2->3 ×1 to ×2".into(), "1->2, 2->3*2".into()),
            ]
        );
        assert_eq!(changes(&q, Mods { multiplicity: false, new_edge: true }).len(), 1);
        // 3 -> 1 would close 1 -> 2 -> 3
    }
}
