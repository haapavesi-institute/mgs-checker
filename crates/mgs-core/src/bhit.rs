//! The one pruning rule: Brüstle–Hermes–Igusa–Todorov, J. Algebra 473
//! (2017), Corollary 3.3.2.
//!
//! As written: "Consider any maximal green sequence on any valued quiver Q.
//! Then, at each step, the mutation is at a vertex of the mutated quiver Q′
//! which is not the source of any arrow of infinite type." BHIT frame the
//! opposite quiver (see `framed`), and for a simply-laced quiver an arrow has
//! infinite type exactly when it comes with two or more parallel copies. So
//! in this convention: no step of any MGS mutates a vertex that is, at that
//! moment, the target of two or more parallel arrows. There are no other
//! hypotheses; acyclicity is not needed.

use crate::framed::Framed;

/// If mutating k now breaks BHIT 3.3.2: the vertex i with m ≥ 2 arrows i → k.
pub fn banned_332(f: &Framed, k: usize) -> Option<(usize, i64)> {
    (0..f.n()).map(|i| (i, f.b(i, k))).find(|&(_, m)| m >= 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matrix::Exchange;

    #[test]
    fn targets_of_double_arrows_are_banned() {
        let f = Framed::new(&Exchange::from_arrow_text("1->2*2, 2->3", None).unwrap());
        assert_eq!(banned_332(&f, 1), Some((0, 2)));
        assert_eq!(banned_332(&f, 0), None);
        assert_eq!(banned_332(&f, 2), None);
        let after3 = f.mutate(2).unwrap(); // 1 => 2 <- 3: 2 is still the target
        assert_eq!(banned_332(&after3, 1), Some((0, 2)));
    }
}
