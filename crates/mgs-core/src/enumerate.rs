//! Small connected quivers, once each up to relabelling: every acyclic one,
//! every one (oriented cycles allowed), or a seeded random sample.
//!
//! An acyclic quiver can be relabelled so that every arrow goes from a lower
//! label to a higher one, so for those it is enough to run over
//! upper-triangular matrices with entries 0..=max_m. With oriented cycles
//! allowed, every entry above the diagonal runs over −max_m..=max_m.

use std::collections::HashSet;

use crate::canonical::canonical_key;
use crate::classify::components;
use crate::matrix::{Exchange, MAX_VERTICES};

/// The most matrices one enumeration will look at.
pub const MAX_CODES: u64 = 20_000_000;

fn pairs(n: usize) -> Vec<(usize, usize)> {
    (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j))).collect()
}

fn checked(n: usize, max_m: i64) -> Result<(), String> {
    if n == 0 || n > MAX_VERTICES {
        return Err(format!("the vertex count must be between 1 and {MAX_VERTICES}"));
    }
    if max_m < 1 {
        return Err("the largest multiplicity must be at least 1".into());
    }
    Ok(())
}

/// Every matrix of `codes` that `keep` accepts (asked first, so it should be
/// cheap) and that is connected, one per relabelling class, in order.
fn distinct(matrices: impl Iterator<Item = Exchange>, keep: impl Fn(&Exchange) -> bool) -> Vec<Exchange> {
    let mut seen = HashSet::new();
    matrices.filter(|q| keep(q) && components(q).len() == 1 && seen.insert(canonical_key(q, false))).collect()
}

/// Entries `values[code digit]` above the diagonal, digit by digit.
fn matrix(n: usize, pairs: &[(usize, usize)], values: &[i64], mut code: u64) -> Exchange {
    let mut q = Exchange::zero(n);
    let base = values.len() as u64;
    for &(i, j) in pairs {
        let v = values[(code % base) as usize];
        code /= base;
        if v != 0 {
            q.set_pair(i, j, v);
        }
    }
    q
}

fn all_codes(n: usize, values: &[i64], what: &str) -> Result<u64, String> {
    (values.len() as u64)
        .checked_pow(pairs(n).len() as u32)
        .filter(|&c| c <= MAX_CODES)
        .ok_or_else(|| format!("{n} vertices {what} is more than {MAX_CODES} matrices to look at; lower a limit, or take a random sample"))
}

/// Every connected acyclic quiver with n vertices and at most `max_m` parallel arrows.
pub fn acyclic_quivers(n: usize, max_m: i64, keep: impl Fn(&Exchange) -> bool) -> Result<Vec<Exchange>, String> {
    checked(n, max_m)?;
    let values: Vec<i64> = (0..=max_m).collect();
    let codes = all_codes(n, &values, &format!("with up to {max_m} arrows, acyclic,"))?;
    let p = pairs(n);
    Ok(distinct((0..codes).map(|c| matrix(n, &p, &values, c)), keep))
}

/// Every connected quiver with n vertices and at most `max_m` parallel arrows, oriented cycles allowed.
pub fn all_quivers(n: usize, max_m: i64, keep: impl Fn(&Exchange) -> bool) -> Result<Vec<Exchange>, String> {
    checked(n, max_m)?;
    let values: Vec<i64> = (-max_m..=max_m).collect();
    let codes = all_codes(n, &values, &format!("with up to {max_m} arrows"))?;
    let p = pairs(n);
    Ok(distinct((0..codes).map(|c| matrix(n, &p, &values, c)), keep))
}

/// `count` random matrices (each entry uniform in −max_m..=max_m, from `seed`), the distinct connected ones kept.
pub fn sampled_quivers(n: usize, max_m: i64, count: u64, seed: u64, keep: impl Fn(&Exchange) -> bool) -> Result<Vec<Exchange>, String> {
    checked(n, max_m)?;
    if count > MAX_CODES {
        return Err(format!("a sample of more than {MAX_CODES} matrices is too many"));
    }
    let values: Vec<i64> = (-max_m..=max_m).collect();
    let p = pairs(n);
    let mut state = seed;
    // SplitMix64: one 64-bit draw per entry, reduced mod the number of values.
    let mut draw = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let base = values.len() as u64;
    let matrices = (0..count).map(|_| {
        let mut q = Exchange::zero(n);
        for &(i, j) in &p {
            let v = values[(draw() % base) as usize];
            if v != 0 {
                q.set_pair(i, j, v);
            }
        }
        q
    });
    Ok(distinct(matrices, keep))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::is_acyclic;

    #[test]
    fn counts_of_small_connected_quivers() {
        // Simple graphs: 1 tree on 2 vertices; on 3, the path's 3 orientations
        // and the triangle's 1 acyclic one (up to relabelling).
        assert_eq!(acyclic_quivers(2, 1, |_| true).unwrap().len(), 1);
        assert_eq!(acyclic_quivers(3, 1, |_| true).unwrap().len(), 4);
        // With double arrows allowed, the 2-vertex ones are 1 → 2 and 1 ⇉ 2.
        assert_eq!(acyclic_quivers(2, 2, |_| true).unwrap().len(), 2);
        // Oriented cycles allowed: the oriented triangle as well.
        assert_eq!(all_quivers(3, 1, |_| true).unwrap().len(), 5);
        let all = all_quivers(4, 2, |_| true).unwrap();
        let acyclic = acyclic_quivers(4, 2, |_| true).unwrap();
        assert_eq!(all.iter().filter(|q| is_acyclic(q)).count(), acyclic.len());
        assert!(acyclic_quivers(9, 3, |_| true).is_err());
    }

    #[test]
    fn a_sample_is_repeatable_and_inside_the_full_list() {
        let a = sampled_quivers(4, 1, 2000, 7, |_| true).unwrap();
        assert_eq!(a, sampled_quivers(4, 1, 2000, 7, |_| true).unwrap());
        let full: HashSet<Vec<i64>> = all_quivers(4, 1, |_| true).unwrap().iter().map(|q| canonical_key(q, false)).collect();
        assert!(a.iter().all(|q| full.contains(&canonical_key(q, false))));
        assert!(a.len() > full.len() / 2); // 2000 draws over 3^6 = 729 matrices find most classes
    }
}
