//! Exchange matrices of quivers: skew-symmetric integer matrices with
//! b_ij = #(i → j) − #(j → i). Vertices are 0-based in code and 1-based in
//! everything a person reads or writes.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The largest quiver accepted. Searches stop being feasible well before
/// this; the cap only keeps a bad file from allocating a huge matrix.
pub const MAX_VERTICES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "Vec<Vec<i64>>", into = "Vec<Vec<i64>>")]
pub struct Exchange {
    n: usize,
    b: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatrixError {
    Empty,
    TooLarge { n: usize },
    NotSquare { row: usize, len: usize, n: usize },
    Diagonal { i: usize, value: i64 },
    NotSkew { i: usize, j: usize, bij: i64, bji: i64 },
    Parse { line: usize, column: usize, token: String },
    Arrow { token: String, reason: String },
}

impl fmt::Display for MatrixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "the matrix is empty"),
            Self::TooLarge { n } => {
                write!(f, "{n} vertices is more than the {MAX_VERTICES} accepted")
            }
            Self::NotSquare { row, len, n } => write!(f, "row {row} has {len} entries, but the matrix has {n} rows"),
            Self::Diagonal { i, value } => write!(f, "entry ({i}, {i}) is {value}; a quiver has no loops, so the diagonal must be 0"),
            Self::NotSkew { i, j, bij, bji } => {
                write!(
                    f,
                    "entries ({i}, {j}) = {bij} and ({j}, {i}) = {bji} are not negatives of each other; the matrix must be skew-symmetric"
                )
            }
            Self::Parse { line, column, token } => write!(f, "line {line}, column {column}: {token:?} is not an integer"),
            Self::Arrow { token, reason } => write!(f, "arrow {token:?}: {reason}"),
        }
    }
}

impl std::error::Error for MatrixError {}

impl Exchange {
    /// The matrix with these rows, checked: square, zero diagonal, skew-symmetric.
    pub fn from_rows(rows: Vec<Vec<i64>>) -> Result<Self, MatrixError> {
        let n = rows.len();
        if n == 0 {
            return Err(MatrixError::Empty);
        }
        if n > MAX_VERTICES {
            return Err(MatrixError::TooLarge { n });
        }
        if let Some((row, r)) = rows.iter().enumerate().find(|(_, r)| r.len() != n) {
            return Err(MatrixError::NotSquare { row: row + 1, len: r.len(), n });
        }
        for (i, row) in rows.iter().enumerate() {
            if row[i] != 0 {
                return Err(MatrixError::Diagonal { i: i + 1, value: row[i] });
            }
            for (j, &bij) in row.iter().enumerate().skip(i + 1) {
                let bji = rows[j][i];
                if bij.checked_neg() != Some(bji) {
                    return Err(MatrixError::NotSkew { i: i + 1, j: j + 1, bij, bji });
                }
            }
        }
        Ok(Self { n, b: rows.into_iter().flatten().collect() })
    }

    /// The quiver with n vertices and no arrows.
    pub fn zero(n: usize) -> Self {
        Self { n, b: vec![0; n * n] }
    }

    pub fn n(&self) -> usize {
        self.n
    }

    pub fn get(&self, i: usize, j: usize) -> i64 {
        self.b[i * self.n + j]
    }

    /// Sets b_ij = v and b_ji = −v.
    pub fn set_pair(&mut self, i: usize, j: usize, v: i64) {
        assert!(i != j, "a quiver has no loops");
        self.b[i * self.n + j] = v;
        self.b[j * self.n + i] = -v;
    }

    pub fn entries(&self) -> &[i64] {
        &self.b
    }

    pub fn rows(&self) -> Vec<Vec<i64>> {
        self.b.chunks(self.n).map(<[i64]>::to_vec).collect()
    }

    /// Every arrow bundle (i, j, m): m > 0 arrows i → j.
    pub fn arrows(&self) -> Vec<(usize, usize, i64)> {
        let mut out = Vec::new();
        for i in 0..self.n {
            for j in 0..self.n {
                if self.get(i, j) > 0 {
                    out.push((i, j, self.get(i, j)));
                }
            }
        }
        out
    }

    /// "1->2*2, 2->3", or "(no arrows)".
    pub fn arrow_text(&self) -> String {
        let parts: Vec<String> = self
            .arrows()
            .into_iter()
            .map(|(i, j, m)| if m == 1 { format!("{}->{}", i + 1, j + 1) } else { format!("{}->{}*{m}", i + 1, j + 1) })
            .collect();
        if parts.is_empty() {
            "(no arrows)".into()
        } else {
            parts.join(", ")
        }
    }

    /// Parses "1->2*2, 2->3". Bundles between the same pair add up, so
    /// "1->2, 2->1" cancels. `n` defaults to the largest vertex named.
    pub fn from_arrow_text(text: &str, n: Option<usize>) -> Result<Self, MatrixError> {
        let mut arrows = Vec::new();
        for token in text.split(|c: char| c == ',' || c == ';' || c.is_whitespace()).filter(|t| !t.is_empty()) {
            let bad = |reason: &str| MatrixError::Arrow { token: token.into(), reason: reason.into() };
            let (from, rest) = token.split_once("->").ok_or_else(|| bad("write arrows as i->j or i->j*m"))?;
            let (to, m) = rest.split_once('*').map_or((rest, "1"), |(t, m)| (t, m));
            let parse = |s: &str| s.parse::<usize>().ok().filter(|&v| v >= 1);
            let (Some(from), Some(to)) = (parse(from), parse(to)) else {
                return Err(bad("vertices are numbered from 1"));
            };
            let m: i64 = m.parse().ok().filter(|&m| m >= 1).ok_or_else(|| bad("the multiplicity must be a positive integer"))?;
            if from == to {
                return Err(bad("a quiver has no loops"));
            }
            arrows.push((from - 1, to - 1, m));
        }
        let n = n.unwrap_or_else(|| arrows.iter().map(|&(i, j, _)| i.max(j) + 1).max().unwrap_or(0));
        if n == 0 {
            return Err(MatrixError::Empty);
        }
        if n > MAX_VERTICES {
            return Err(MatrixError::TooLarge { n });
        }
        let mut q = Self::zero(n);
        for (i, j, m) in arrows {
            if i >= n || j >= n {
                return Err(MatrixError::Arrow {
                    token: format!("{}->{}", i + 1, j + 1),
                    reason: format!("the quiver has only {n} vertices"),
                });
            }
            let v = q
                .get(i, j)
                .checked_add(m)
                .ok_or(MatrixError::Arrow { token: format!("{}->{}", i + 1, j + 1), reason: "too many arrows".into() })?;
            q.set_pair(i, j, v);
        }
        Ok(q)
    }

    /// Q^op: every arrow reversed.
    pub fn opposite(&self) -> Self {
        Self { n: self.n, b: self.b.iter().map(|x| -x).collect() }
    }

    /// Relabelled so that new vertex i is old vertex p[i].
    pub fn permuted(&self, p: &[usize]) -> Self {
        let n = self.n;
        let mut b = vec![0; n * n];
        for i in 0..n {
            for j in 0..n {
                b[i * n + j] = self.get(p[i], p[j]);
            }
        }
        Self { n, b }
    }
}

impl TryFrom<Vec<Vec<i64>>> for Exchange {
    type Error = MatrixError;
    fn try_from(rows: Vec<Vec<i64>>) -> Result<Self, MatrixError> {
        Self::from_rows(rows)
    }
}

impl From<Exchange> for Vec<Vec<i64>> {
    fn from(q: Exchange) -> Self {
        q.rows()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_shape_and_skew_symmetry() {
        assert_eq!(Exchange::from_rows(vec![]), Err(MatrixError::Empty));
        assert!(matches!(Exchange::from_rows(vec![vec![0, 1], vec![-1]]), Err(MatrixError::NotSquare { row: 2, .. })));
        assert!(matches!(Exchange::from_rows(vec![vec![1, 1], vec![-1, 0]]), Err(MatrixError::Diagonal { i: 1, value: 1 })));
        assert!(matches!(Exchange::from_rows(vec![vec![0, 2], vec![-1, 0]]), Err(MatrixError::NotSkew { i: 1, j: 2, .. })));
        assert!(Exchange::from_rows(vec![vec![0, 2], vec![-2, 0]]).is_ok());
    }

    #[test]
    fn arrow_text_round_trips_and_cancels() {
        let q = Exchange::from_arrow_text("1->2*2, 2->3", None).unwrap();
        assert_eq!(q.rows(), vec![vec![0, 2, 0], vec![-2, 0, 1], vec![0, -1, 0]]);
        assert_eq!(q.arrow_text(), "1->2*2, 2->3");
        assert_eq!(Exchange::from_arrow_text("1->2, 2->1", None).unwrap().arrow_text(), "(no arrows)");
        assert_eq!(Exchange::from_arrow_text("1->2", Some(3)).unwrap().n(), 3);
        assert!(Exchange::from_arrow_text("1->1", None).is_err());
        assert!(Exchange::from_arrow_text("0->1", None).is_err());
        assert!(Exchange::from_arrow_text("1-2", None).is_err());
    }

    #[test]
    fn opposite_and_permutation() {
        let q = Exchange::from_arrow_text("1->2*2, 2->3", None).unwrap();
        assert_eq!(q.opposite().arrow_text(), "2->1*2, 3->2");
        assert_eq!(q.permuted(&[2, 1, 0]).arrow_text(), "2->1, 3->2*2");
    }
}
