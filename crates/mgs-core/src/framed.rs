//! Framed quivers, c-vectors and green/red vertices.
//!
//! Convention (Brüstle–Dupont–Pérotin, "On maximal green sequences", IMRN
//! 2014, Definitions 2.4–2.5): the framed quiver of Q adds a frozen vertex i′
//! and an arrow i → i′ for each vertex i. It is stored as [B | C], with rows
//! the mutable vertices: row i of C is the c-vector of i, and C starts as the
//! identity. A vertex is green when its c-vector is ≥ 0 and red when ≤ 0.
//! Every c-vector is one or the other (sign-coherence, Derksen–Weyman–
//! Zelevinsky 2010, BDP Theorem 2.6), and the code checks it anyway.
//!
//! Brüstle–Hermes–Igusa–Todorov append I below B instead, which amounts to
//! framing the opposite quiver; their results are translated where used.

use std::hash::{Hash, Hasher};

use rustc_hash::FxHasher;

use crate::matrix::Exchange;

/// An entry outgrew 64 bits. Only wild quivers do this, far down long green sequences.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overflow;

/// The isomorphism class of a framed quiver, fixing frozen vertices: its
/// c-vectors, sorted. (The c-matrix is invertible, so the sort is a unique
/// relabelling, and it determines the exchange matrix too; see `class_key`.)
pub type Key = Box<[i64]>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Framed {
    n: usize,
    b: Vec<i64>,
    c: Vec<i64>,
}

fn pos(x: i64) -> i128 {
    i128::from(x.max(0))
}

impl Framed {
    /// The framed quiver of q: every vertex green, C = I.
    pub fn new(q: &Exchange) -> Self {
        let n = q.n();
        let mut c = vec![0; n * n];
        for i in 0..n {
            c[i * n + i] = 1;
        }
        Self { n, b: q.entries().to_vec(), c }
    }

    pub fn n(&self) -> usize {
        self.n
    }

    pub fn b(&self, i: usize, j: usize) -> i64 {
        self.b[i * self.n + j]
    }

    pub fn c_vector(&self, i: usize) -> &[i64] {
        &self.c[i * self.n..(i + 1) * self.n]
    }

    /// The mutable part as a quiver.
    pub fn quiver(&self) -> Exchange {
        Exchange::from_rows(self.b.chunks(self.n).map(<[i64]>::to_vec).collect()).expect("mutation keeps B skew-symmetric")
    }

    /// Green (c-vector ≥ 0) or red (≤ 0). Panics if sign-coherence failed,
    /// which DWZ prove cannot happen for skew-symmetric matrices.
    pub fn is_green(&self, i: usize) -> bool {
        let v = self.c_vector(i);
        let (pos, neg) = (v.iter().any(|&x| x > 0), v.iter().any(|&x| x < 0));
        assert!(
            pos != neg,
            "c-vector of vertex {} is {v:?}: sign-coherence failed, which contradicts DWZ; please report this quiver",
            i + 1
        );
        pos
    }

    pub fn greens(&self) -> Vec<usize> {
        (0..self.n).filter(|&i| self.is_green(i)).collect()
    }

    pub fn all_red(&self) -> bool {
        (0..self.n).all(|i| !self.is_green(i))
    }

    /// Fomin–Zelevinsky mutation at k of the whole framed quiver:
    /// x′_ij = −x_ij in row or column k, else x_ij + [b_ik]₊[x_kj]₊ − [−b_ik]₊[−x_kj]₊.
    pub fn mutate(&self, k: usize) -> Result<Self, Overflow> {
        let n = self.n;
        let rule = |i: usize, xij: i64, xkj: i64| -> Result<i64, Overflow> {
            let bik = self.b(i, k);
            let v = i128::from(xij) + pos(bik) * pos(xkj) - pos(-bik) * pos(-xkj);
            i64::try_from(v).map_err(|_| Overflow)
        };
        let mut b = vec![0; n * n];
        let mut c = vec![0; n * n];
        for i in 0..n {
            for j in 0..n {
                let (bij, cij) = (self.b(i, j), self.c[i * n + j]);
                b[i * n + j] = if i == k || j == k { bij.checked_neg().ok_or(Overflow)? } else { rule(i, bij, self.b(k, j))? };
                c[i * n + j] = if i == k { cij.checked_neg().ok_or(Overflow)? } else { rule(i, cij, self.c[k * n + j])? };
            }
        }
        Ok(Self { n, b, c })
    }

    /// The class key and a hash of the exchange matrix relabelled the same way.
    ///
    /// For skew-symmetric B, B_t = C_t B_0 C_tᵀ (Fomin–Zelevinsky IV, eq. 6.14,
    /// G_t B_t = B_0 C_t, with G_tᵀ = C_t⁻¹ by Nakanishi–Zelevinsky, translated
    /// to this convention), so the c-vectors alone determine the state. The
    /// hash lets the search check that on every memo hit instead of trusting it.
    pub fn class_key(&self) -> (Key, u64) {
        let n = self.n;
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&i, &j| self.c_vector(i).cmp(self.c_vector(j)));
        let key: Key = order.iter().flat_map(|&i| self.c_vector(i).iter().copied()).collect();
        let mut h = FxHasher::default();
        for (x, &i) in order.iter().enumerate() {
            for &j in &order[x + 1..] {
                self.b(i, j).hash(&mut h);
            }
        }
        (key, h.finish())
    }

    /// If every vertex is red, C = −P for a permutation (BDP Prop. 2.10):
    /// returns σ with c-vector of i = −e_σ(i).
    pub fn final_permutation(&self) -> Option<Vec<usize>> {
        let mut sigma = Vec::with_capacity(self.n);
        for i in 0..self.n {
            let v = self.c_vector(i);
            let j = v.iter().position(|&x| x == -1)?;
            if v.iter().enumerate().any(|(t, &x)| t != j && x != 0) {
                return None;
            }
            sigma.push(j);
        }
        Some(sigma)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(text: &str) -> Exchange {
        Exchange::from_arrow_text(text, None).unwrap()
    }

    #[test]
    fn a2_by_hand() {
        let f = Framed::new(&q("1->2"));
        assert_eq!(f.greens(), vec![0, 1]);
        let f1 = f.mutate(0).unwrap();
        assert_eq!(f1.c_vector(0), &[-1, 0]);
        assert_eq!(f1.c_vector(1), &[0, 1]); // no path 2 -> 1 -> 1', so the c-vector of 2 is unchanged
        let f12 = f1.mutate(1).unwrap();
        assert!(f12.all_red());
        let f2 = f.mutate(1).unwrap();
        assert_eq!(f2.c_vector(0), &[1, 1]); // the path 1 -> 2 -> 2' adds 1 -> 2'
    }

    /// The identity behind keying states by c-vectors alone, on walks through
    /// several quivers, cyclic and wild ones included.
    #[test]
    fn exchange_matrix_is_determined_by_c_vectors() {
        for text in ["1->2, 2->3, 3->1", "1->2*2, 2->3", "1->2, 2->3, 3->4*2", "1->2*2, 1->3, 3->2, 2->4, 4->3", "1->2*3, 2->3, 3->1*2"] {
            let q0 = q(text);
            let n = q0.n();
            let mut f = Framed::new(&q0);
            for step in 0..14 {
                let k = (step * 7 + 3) % n;
                let Ok(next) = f.mutate(k) else { break };
                f = next;
                for i in 0..n {
                    for j in 0..n {
                        let mut v = 0i128;
                        for a in 0..n {
                            for b in 0..n {
                                v += i128::from(f.c_vector(i)[a]) * i128::from(q0.get(a, b)) * i128::from(f.c_vector(j)[b]);
                            }
                        }
                        assert_eq!(i128::from(f.b(i, j)), v, "{text}, step {step}, entry ({i}, {j})");
                    }
                }
            }
        }
    }

    #[test]
    fn mgs_ends_in_minus_a_permutation() {
        let mut f = Framed::new(&q("1->2, 2->3"));
        for k in [0, 1, 2] {
            f = f.mutate(k).unwrap();
        }
        assert!(f.all_red());
        assert!(f.final_permutation().is_some());
    }
}
