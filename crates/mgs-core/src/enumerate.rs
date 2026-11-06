//! Every connected acyclic quiver with n vertices and at most `max_m`
//! parallel arrows, once each up to relabelling.
//!
//! An acyclic quiver can be relabelled so that every arrow goes from a lower
//! label to a higher one. So it is enough to run over upper-triangular
//! matrices with entries 0..=max_m, keeping one per relabelling class.

use std::collections::HashSet;

use crate::canonical::canonical_key;
use crate::classify::components;
use crate::matrix::{Exchange, MAX_VERTICES};

/// The most matrices one enumeration will look at.
pub const MAX_CODES: u64 = 20_000_000;

/// The quivers, in a fixed order, among those `keep` accepts (it is asked
/// before the relabelling class is computed, so it should be cheap).
pub fn acyclic_quivers(n: usize, max_m: i64, keep: impl Fn(&Exchange) -> bool) -> Result<Vec<Exchange>, String> {
    if n == 0 || n > MAX_VERTICES {
        return Err(format!("the vertex count must be between 1 and {MAX_VERTICES}"));
    }
    if max_m < 1 {
        return Err("the largest multiplicity must be at least 1".into());
    }
    let pairs: Vec<(usize, usize)> = (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j))).collect();
    let base = max_m as u64 + 1;
    let codes = base
        .checked_pow(pairs.len() as u32)
        .filter(|&c| c <= MAX_CODES)
        .ok_or_else(|| format!("{n} vertices with up to {max_m} arrows is more than {MAX_CODES} matrices to look at; lower one of them"))?;
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for code in 0..codes {
        let mut q = Exchange::zero(n);
        let mut c = code;
        for &(i, j) in &pairs {
            let m = (c % base) as i64;
            c /= base;
            if m > 0 {
                q.set_pair(i, j, m);
            }
        }
        if keep(&q) && components(&q).len() == 1 && seen.insert(canonical_key(&q, false)) {
            out.push(q);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_of_small_connected_acyclic_quivers() {
        // Simple graphs: 1 tree on 2 vertices; on 3, the path's 3 orientations
        // and the triangle's 1 acyclic one (up to relabelling).
        assert_eq!(acyclic_quivers(2, 1, |_| true).unwrap().len(), 1);
        assert_eq!(acyclic_quivers(3, 1, |_| true).unwrap().len(), 4);
        // With double arrows allowed, the 2-vertex ones are 1 → 2 and 1 ⇉ 2.
        assert_eq!(acyclic_quivers(2, 2, |_| true).unwrap().len(), 2);
        for q in acyclic_quivers(4, 2, |_| true).unwrap() {
            assert!(crate::classify::is_acyclic(&q));
        }
        assert!(acyclic_quivers(9, 3, |_| true).is_err());
    }
}
