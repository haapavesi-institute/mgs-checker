//! A canonical form of a quiver up to relabelling its vertices, so that an
//! exploration checks each quiver once.
//!
//! Colour refinement first splits the vertices by an isomorphism-invariant
//! colour (the multiset of arrow counts to each colour, iterated until
//! stable). Then every relabelling that lists the colour classes in order is
//! tried, and the smallest upper triangle wins. The colours are invariant and
//! every consistent relabelling is tried, so isomorphic quivers get the same
//! form; for the trees and cycles explored here the classes are tiny.

use crate::matrix::Exchange;

fn refine(q: &Exchange) -> Vec<usize> {
    let n = q.n();
    let mut colour = vec![0usize; n];
    let mut classes = 1;
    loop {
        let sig: Vec<(usize, Vec<(i64, usize)>)> = (0..n)
            .map(|i| {
                let mut s: Vec<(i64, usize)> = (0..n).filter(|&j| q.get(i, j) != 0).map(|j| (q.get(i, j), colour[j])).collect();
                s.sort_unstable();
                (colour[i], s)
            })
            .collect();
        let mut distinct = sig.clone();
        distinct.sort();
        distinct.dedup();
        let next: Vec<usize> = sig.iter().map(|s| distinct.binary_search(s).expect("present")).collect();
        if distinct.len() == classes {
            return next;
        }
        classes = distinct.len();
        colour = next;
    }
}

fn upper(q: &Exchange, p: &[usize]) -> Vec<i64> {
    let n = p.len();
    (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j))).map(|(i, j)| q.get(p[i], p[j])).collect()
}

/// The canonical relabelling of q.
pub fn canonical(q: &Exchange) -> Exchange {
    let n = q.n();
    let colour = refine(q);
    let mut classes: Vec<Vec<usize>> = Vec::new();
    for c in 0..=colour.iter().copied().max().unwrap_or(0) {
        let class: Vec<usize> = (0..n).filter(|&v| colour[v] == c).collect();
        if !class.is_empty() {
            classes.push(class);
        }
    }
    let mut best: Option<(Vec<i64>, Vec<usize>)> = None;
    let mut p = Vec::with_capacity(n);
    search(q, &classes, 0, &mut vec![false; n], &mut p, &mut best);
    q.permuted(&best.expect("at least one relabelling").1)
}

fn search(
    q: &Exchange,
    classes: &[Vec<usize>],
    at: usize,
    used: &mut Vec<bool>,
    p: &mut Vec<usize>,
    best: &mut Option<(Vec<i64>, Vec<usize>)>,
) {
    let Some(class) = classes.get(at) else {
        let u = upper(q, p);
        if best.as_ref().is_none_or(|(b, _)| u < *b) {
            *best = Some((u, p.clone()));
        }
        return;
    };
    let placed = class.iter().filter(|&&v| used[v]).count();
    if placed == class.len() {
        return search(q, classes, at + 1, used, p, best);
    }
    for &v in class {
        if !used[v] {
            used[v] = true;
            p.push(v);
            search(q, classes, at, used, p, best);
            p.pop();
            used[v] = false;
        }
    }
}

/// Canonical up to relabelling and, if `with_opposite`, up to reversing every arrow too.
pub fn canonical_key(q: &Exchange, with_opposite: bool) -> Vec<i64> {
    let a = canonical(q).entries().to_vec();
    if !with_opposite {
        return a;
    }
    let b = canonical(&q.opposite()).entries().to_vec();
    a.min(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(text: &str) -> Exchange {
        Exchange::from_arrow_text(text, None).unwrap()
    }

    #[test]
    fn isomorphic_quivers_share_a_form() {
        let a = q("1->2, 2->3, 1->3");
        for p in [[0, 1, 2], [2, 0, 1], [1, 2, 0], [2, 1, 0]] {
            assert_eq!(canonical(&a.permuted(&p)), canonical(&a));
        }
        assert_ne!(canonical(&q("1->2, 2->3")), canonical(&q("1->2, 3->2")));
        assert_eq!(canonical_key(&q("1->2, 2->3, 3->4*2"), true), canonical_key(&q("1->2*2, 2->3, 3->4"), true)); // op, relabelled
        assert_ne!(canonical_key(&q("1->2, 2->3, 3->4*2"), false), canonical_key(&q("1->2*2, 2->3, 3->4"), false));
    }

    #[test]
    fn symmetric_quivers() {
        let star = q("1->5, 2->5, 3->5, 4->5");
        assert_eq!(canonical(&star.permuted(&[4, 2, 0, 3, 1])), canonical(&star));
        let square = q("1->2, 3->2, 3->4, 1->4");
        assert_eq!(canonical(&square.permuted(&[1, 2, 3, 0])), canonical(&square));
    }
}
