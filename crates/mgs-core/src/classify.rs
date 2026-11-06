//! What kind of quiver this is: connected components, oriented cycles, and
//! for a connected acyclic quiver the type of its underlying graph (Dynkin,
//! Euclidean or wild), recognised from the graph's shape.
//!
//! Gabriel and Dlab–Ringel/Donovan–Freislich: a connected quiver's path
//! algebra is representation-finite exactly when the underlying graph is
//! A_n, D_n, E_6, E_7 or E_8, and tame exactly when it is Ã_n (a cycle; the
//! Kronecker double edge is Ã_1), D̃_n, Ẽ_6, Ẽ_7 or Ẽ_8.

use serde::Serialize;

use crate::matrix::Exchange;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Kind {
    /// Dynkin: family 'A', 'D' or 'E', rank = number of vertices.
    Dynkin {
        family: char,
        rank: usize,
    },
    /// Ã with a arrows one way round the cycle and b the other (a ≥ b ≥ 1, a + b vertices).
    ATilde {
        a: usize,
        b: usize,
    },
    /// D̃_rank or Ẽ_rank, with rank + 1 vertices.
    Euclidean {
        family: char,
        rank: usize,
    },
    Wild,
    /// Has an oriented cycle, so the representation type of the graph says nothing directly.
    Cyclic,
    Disconnected {
        parts: Vec<Kind>,
    },
}

impl Kind {
    pub fn name(&self) -> String {
        match self {
            Self::Dynkin { family, rank } => format!("Dynkin {family}{rank}"),
            Self::ATilde { a, b } => format!("Euclidean Ã{} = Ã({a},{b})", a + b - 1),
            Self::Euclidean { family, rank } => format!("Euclidean {family}\u{303}{rank}"),
            Self::Wild => "wild (acyclic)".into(),
            Self::Cyclic => "has an oriented cycle".into(),
            Self::Disconnected { parts } => format!("disconnected: {}", parts.iter().map(Kind::name).collect::<Vec<_>>().join(" + ")),
        }
    }

    pub fn is_tame(&self) -> bool {
        matches!(self, Self::ATilde { .. } | Self::Euclidean { .. })
    }
}

/// The vertex sets of the connected components, each sorted.
pub fn components(q: &Exchange) -> Vec<Vec<usize>> {
    let n = q.n();
    let mut seen = vec![false; n];
    let mut out = Vec::new();
    for s in 0..n {
        if seen[s] {
            continue;
        }
        let (mut part, mut stack) = (Vec::new(), vec![s]);
        seen[s] = true;
        while let Some(v) = stack.pop() {
            part.push(v);
            for (w, seen_w) in seen.iter_mut().enumerate() {
                if !*seen_w && q.get(v, w) != 0 {
                    *seen_w = true;
                    stack.push(w);
                }
            }
        }
        part.sort_unstable();
        out.push(part);
    }
    out
}

/// No oriented cycle (Kahn's algorithm).
pub fn is_acyclic(q: &Exchange) -> bool {
    let n = q.n();
    let mut indeg: Vec<usize> = (0..n).map(|j| (0..n).filter(|&i| q.get(i, j) > 0).count()).collect();
    let mut ready: Vec<usize> = (0..n).filter(|&j| indeg[j] == 0).collect();
    let mut done = 0;
    while let Some(v) = ready.pop() {
        done += 1;
        for (w, d) in indeg.iter_mut().enumerate() {
            if q.get(v, w) > 0 {
                *d -= 1;
                if *d == 0 {
                    ready.push(w);
                }
            }
        }
    }
    done == n
}

pub fn classify(q: &Exchange) -> Kind {
    let parts = components(q);
    if parts.len() > 1 {
        return Kind::Disconnected { parts: parts.iter().map(|p| classify(&sub(q, p))).collect() };
    }
    if !is_acyclic(q) {
        return Kind::Cyclic;
    }
    graph_type(q)
}

fn sub(q: &Exchange, vs: &[usize]) -> Exchange {
    Exchange::from_rows(vs.iter().map(|&i| vs.iter().map(|&j| q.get(i, j)).collect()).collect())
        .expect("a principal submatrix is skew-symmetric")
}

/// The type of a connected acyclic quiver's underlying graph.
fn graph_type(q: &Exchange) -> Kind {
    let n = q.n();
    let adj: Vec<Vec<usize>> = (0..n).map(|i| (0..n).filter(|&j| q.get(i, j) != 0).collect()).collect();
    let max_m = (0..n).flat_map(|i| (0..n).map(move |j| (i, j))).map(|(i, j)| q.get(i, j).abs()).max().unwrap_or(0);
    if n == 1 {
        return Kind::Dynkin { family: 'A', rank: 1 };
    }
    if max_m >= 3 || (max_m == 2 && n > 2) {
        return Kind::Wild;
    }
    if max_m == 2 {
        return Kind::ATilde { a: 1, b: 1 }; // the Kronecker quiver
    }
    let edges = adj.iter().map(Vec::len).sum::<usize>() / 2;
    if edges == n {
        return if adj.iter().all(|a| a.len() == 2) { cycle_type(q, &adj) } else { Kind::Wild };
    }
    if edges > n {
        return Kind::Wild;
    }
    tree_type(&adj)
}

fn cycle_type(q: &Exchange, adj: &[Vec<usize>]) -> Kind {
    let (mut prev, mut cur, mut forward) = (0, adj[0][0], 0);
    if q.get(0, cur) > 0 {
        forward += 1;
    }
    while cur != 0 {
        let next = if adj[cur][0] == prev { adj[cur][1] } else { adj[cur][0] };
        if q.get(cur, next) > 0 {
            forward += 1;
        }
        (prev, cur) = (cur, next);
    }
    let backward = adj.len() - forward;
    Kind::ATilde { a: forward.max(backward), b: forward.min(backward) }
}

/// Arm lengths (vertices beyond `centre`) of a tree at a branch point.
fn arms(adj: &[Vec<usize>], centre: usize) -> Vec<usize> {
    let mut out: Vec<usize> = adj[centre]
        .iter()
        .map(|&first| {
            let (mut prev, mut cur, mut len) = (centre, first, 1);
            while adj[cur].len() == 2 {
                let next = if adj[cur][0] == prev { adj[cur][1] } else { adj[cur][0] };
                (prev, cur, len) = (cur, next, len + 1);
            }
            len
        })
        .collect();
    out.sort_unstable();
    out
}

fn tree_type(adj: &[Vec<usize>]) -> Kind {
    let n = adj.len();
    let branch: Vec<usize> = (0..n).filter(|&v| adj[v].len() >= 3).collect();
    match branch.as_slice() {
        [] => Kind::Dynkin { family: 'A', rank: n },
        [c] if adj[*c].len() == 4 => {
            if n == 5 {
                Kind::Euclidean { family: 'D', rank: 4 }
            } else {
                Kind::Wild
            }
        }
        [c] if adj[*c].len() == 3 => match arms(adj, *c).as_slice() {
            [1, 1, _] => Kind::Dynkin { family: 'D', rank: n },
            [1, 2, 2] | [1, 2, 3] | [1, 2, 4] => Kind::Dynkin { family: 'E', rank: n },
            [2, 2, 2] | [1, 3, 3] | [1, 2, 5] => Kind::Euclidean { family: 'E', rank: n - 1 },
            _ => Kind::Wild,
        },
        [x, y] if adj[*x].len() == 3 && adj[*y].len() == 3 => {
            // D̃: each branch point carries two leaves, and a path joins them.
            let leaves = |c: usize| adj[c].iter().filter(|&&w| adj[w].len() == 1).count();
            if leaves(*x) == 2 && leaves(*y) == 2 {
                Kind::Euclidean { family: 'D', rank: n - 1 }
            } else {
                Kind::Wild
            }
        }
        _ => Kind::Wild,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(text: &str) -> Kind {
        classify(&Exchange::from_arrow_text(text, None).unwrap())
    }

    #[test]
    fn recognises_the_families() {
        assert_eq!(kind("1->2, 2->3"), Kind::Dynkin { family: 'A', rank: 3 });
        assert_eq!(kind("1->2, 3->2, 4->2"), Kind::Dynkin { family: 'D', rank: 4 });
        assert_eq!(kind("1->2, 2->3, 3->4, 4->5, 3->6"), Kind::Dynkin { family: 'E', rank: 6 });
        assert_eq!(kind("1->2*2"), Kind::ATilde { a: 1, b: 1 });
        assert_eq!(kind("1->2, 2->3, 1->3"), Kind::ATilde { a: 2, b: 1 });
        assert_eq!(kind("1->2, 3->2, 3->4, 1->4"), Kind::ATilde { a: 2, b: 2 });
        assert_eq!(kind("1->5, 2->5, 3->5, 4->5"), Kind::Euclidean { family: 'D', rank: 4 });
        assert_eq!(kind("1->3, 2->3, 3->4, 5->4, 6->4"), Kind::Euclidean { family: 'D', rank: 5 });
        assert_eq!(kind("2->1, 3->2, 4->1, 5->4, 6->1, 7->6"), Kind::Euclidean { family: 'E', rank: 6 });
        assert_eq!(kind("1->2*2, 2->3"), Kind::Wild);
        assert_eq!(kind("1->6, 2->6, 3->6, 4->6, 5->6"), Kind::Wild);
        assert_eq!(kind("1->2, 2->3, 3->1"), Kind::Cyclic);
        assert!(matches!(kind("1->2, 3->4"), Kind::Disconnected { .. }));
    }

    #[test]
    fn e_tilde_7_and_8_and_their_wild_neighbours() {
        let arms = |a: &[usize]| {
            let mut text = Vec::new();
            let mut next = 2;
            for &len in a {
                let mut prev = 1;
                for _ in 0..len {
                    text.push(format!("{next}->{prev}"));
                    (prev, next) = (next, next + 1);
                }
            }
            kind(&text.join(", "))
        };
        assert_eq!(arms(&[1, 3, 3]), Kind::Euclidean { family: 'E', rank: 7 });
        assert_eq!(arms(&[1, 2, 5]), Kind::Euclidean { family: 'E', rank: 8 });
        assert_eq!(arms(&[1, 2, 4]), Kind::Dynkin { family: 'E', rank: 8 });
        assert_eq!(arms(&[1, 2, 6]), Kind::Wild); // E10
        assert_eq!(arms(&[2, 2, 3]), Kind::Wild);
    }
}
