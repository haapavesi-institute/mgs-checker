//! Counts against the literature and against the Quiver Quest engine
//! (https://www.peacecatgames.com/quiver-quest, an independent TypeScript
//! implementation with bigint arithmetic, run with BHIT 3.3.2 pruning; its
//! source is not public, so its values are written out here).

use mgs_core::{check, CheckOptions, Exchange, Report, SearchOptions, Status};

fn run(text: &str, max_gap: u32) -> Report {
    let q = Exchange::from_arrow_text(text, None).unwrap();
    check(&q, &CheckOptions { search: SearchOptions { max_gap, ..SearchOptions::default() }, ..CheckOptions::default() }).unwrap()
}

fn by_length(r: &Report) -> Vec<(u32, u128)> {
    r.by_length.iter().map(|c| (c.length, c.count.parse().unwrap())).collect()
}

fn counts(start: u32, xs: &[u128]) -> Vec<(u32, u128)> {
    xs.iter().enumerate().filter(|(_, &x)| x > 0).map(|(i, &x)| (start + i as u32, x)).collect()
}

#[test]
fn finite_type_and_pruned_quivers_match_the_typescript_engine() {
    let cases: &[(&str, u32, &[u128])] = &[
        ("1->2", 2, &[1, 1]),
        ("1->2, 2->3", 3, &[1, 4, 2, 2]),
        ("1->2, 3->2", 3, &[2, 2, 2, 4]),
        ("1->2, 2->3, 2->4", 4, &[2, 10, 10, 26, 16, 18, 24, 72, 72]),
        ("1->2*2", 2, &[1]),
        ("1->2*3", 2, &[1]),
        ("1->2*2, 2->3", 3, &[1, 2]),
        ("1->2*2, 2->3*2", 3, &[1]),
        ("1->2, 2->3, 3->4*2", 4, &[1, 7, 6, 7]), // BDP Example 5.5
        ("1->2*2, 3->2", 3, &[2, 1]),
        ("3->1, 1->2*2", 3, &[1, 2]),
        ("1->2*2, 1->3", 3, &[2, 1]),
    ];
    for &(text, start, xs) in cases {
        let r = run(text, 0);
        assert_eq!(r.status, Status::Exhausted, "{text}");
        assert_eq!(by_length(&r), counts(start, xs), "{text}");
    }
}

#[test]
fn quivers_with_an_oriented_cycle_are_refused() {
    for (text, cycle) in [
        ("1->2, 2->3, 3->1", vec![1, 2, 3, 1]),
        ("1->2*2, 2->3*2, 3->1*2", vec![1, 2, 3, 1]), // the Markov quiver
        ("1->2, 3->2, 2->4, 4->1, 4->3", vec![1, 2, 4, 1]),
    ] {
        let q = Exchange::from_arrow_text(text, None).unwrap();
        assert_eq!(check(&q, &CheckOptions::default()).unwrap_err().cycle, cycle, "{text}");
    }
}

#[test]
fn bdp_figures_for_type_a() {
    let a5 = run("1->2, 2->3, 3->4, 4->5", 0);
    assert_eq!(by_length(&a5), counts(5, &[1, 20, 112, 232, 382, 348, 456, 390, 420, 334, 286]));
    assert_eq!(run("2->1, 2->3, 3->4", 0).by_length.len(), 7);
    assert_eq!(by_length(&run("1->2, 3->2, 3->4", 0)), counts(4, &[5, 9, 9, 16, 28, 42, 70]));
}

#[test]
fn e6_and_e7_bdp_appendix_b() {
    let e6 = run("1->2, 2->3, 3->4, 4->5, 3->6", 0);
    assert_eq!((e6.status.clone(), e6.total.as_str(), e6.shortest, e6.longest), (Status::Exhausted, "253085705387", Some(6), Some(36)));
    let e7 = run("1->2, 2->3, 3->4, 4->5, 5->6, 4->7", 0);
    assert_eq!((e7.total.as_str(), e7.longest), ("372133972845031649851164", Some(63)));
    assert!(e7.agreements.iter().all(|a| a.agrees));
}

fn maximal(r: &Report) -> Vec<Vec<i64>> {
    r.seeds.as_ref().unwrap().maximal.iter().map(|m| m.vector.clone()).collect()
}

#[test]
fn dynkin_step_c_vectors_are_the_positive_roots_with_the_highest_root_on_top() {
    // BDP Thm 4.4: the longest MGS mutates every positive root once.
    for (text, roots, highest) in [
        ("1->2, 2->3, 3->4, 4->5, 3->6", 36, vec![1, 2, 3, 2, 1, 2]),          // E6
        ("1->2, 2->3, 3->4, 4->5, 5->6, 4->7", 63, vec![1, 2, 3, 4, 3, 2, 2]), // E7
        ("2->1, 3->2, 2->4, 4->5", 20, vec![1, 2, 1, 2, 1]),                   // D5: 2 on the branch point and the long arm
    ] {
        let r = run(text, 0);
        assert_eq!(r.seeds.as_ref().unwrap().step_c_vectors, roots, "{text}");
        assert_eq!(maximal(&r), vec![highest], "{text}");
    }
}

#[test]
fn tame_quivers_have_several_maximal_c_vectors() {
    // Values found by this checker, kept as a regression record.
    let a2 = run("1->2, 2->3, 1->3", 0);
    assert_eq!(maximal(&a2), vec![vec![1, 1, 0], vec![1, 0, 1], vec![0, 1, 1]]);
    let d4 = run("1->5, 2->5, 3->5, 4->5", 0);
    assert_eq!(d4.seeds.as_ref().unwrap().step_c_vectors, 33);
    assert_eq!(maximal(&d4), vec![vec![2, 1, 1, 1, 3], vec![1, 2, 1, 1, 3], vec![1, 1, 2, 1, 3], vec![1, 1, 1, 2, 3]]);
}

#[test]
fn tame_quivers_are_exact_by_the_no_gap_theorem() {
    // (quiver, total, longest): BDP Ex. 5.3 for Ã2; the others from the Quiver Quest exploration.
    for (text, total, longest) in [
        ("1->2, 2->3, 1->3", "5", 5),
        ("1->2, 2->3, 1->4, 4->3", "100", 10),
        ("1->2, 3->2, 3->4, 1->4", "100", 10),
        ("1->2, 2->3, 3->4, 1->4", "75", 9),
        ("1->2, 2->3, 3->4, 4->5, 1->5", "4882", 14),
        ("1->2, 2->3, 3->4, 1->5, 5->4", "11346", 16),
    ] {
        let r = run(text, 0);
        assert!(matches!(r.status, Status::NoGapTheorem { .. }), "{text}: {:?}", r.status);
        assert_eq!((r.total.as_str(), r.longest), (total, Some(longest)), "{text}");
        assert!(r.agreements.iter().all(|a| a.agrees), "{text}: {:?}", r.agreements);
        // Kase–Nakashima's longest
    }
}

#[test]
fn a_larger_gap_only_searches_further() {
    let g0 = run("1->2, 2->3, 1->3, 3->4", 0); // wild (Ã2 with a tail), so the gap stop is conditional
    let g3 = run("1->2, 2->3, 1->3, 3->4", 3);
    assert_eq!(g0.total, g3.total);
    assert!(g3.searched_to >= g0.searched_to + 3 || g3.status == Status::Exhausted);
}

#[test]
fn bans_on_the_worked_example() {
    // 1 => 2 -> 3: (2) and (3, 2) break BHIT 3.3.2, and these are its three MGS.
    let r = run("1->2*2, 2->3", 0);
    assert_eq!(r.sample, vec![vec![1, 2, 3], vec![1, 3, 2, 3], vec![3, 1, 2, 3]]);
    let prefixes: Vec<&Vec<usize>> = r.banned.iter().map(|b| &b.prefix).collect();
    assert!(prefixes.contains(&&vec![2]) && prefixes.contains(&&vec![3, 2]));
}

#[test]
fn igusa_zhou_theorem_3_12_on_every_quiver_up_to_4_vertices() {
    // Every MGS of an acyclic quiver is an MGS of its ME-free version, with the
    // same c-vectors (Igusa–Zhou, arXiv:1902.07375v1, Thm 3.12): conjecture #7's acyclic case.
    use mgs_core::conjecture7::{plan, run, Spec};
    let items = plan(&Spec { max_vertices: 4, max_multiplicity: 2, ..Spec::default() }).unwrap();
    assert_eq!(items.len(), 392);
    for item in &items {
        let row = run(item, &CheckOptions::default()).unwrap();
        assert!(row.verdict == "holds" || row.verdict == "no MGS", "{}: {} {}", row.arrows, row.verdict, row.detail);
        assert_ne!(row.labels_avoid_multiple, Some(false), "{}", row.arrows);
    }
}

#[test]
fn conjecture_7_holds_on_every_open_four_vertex_quiver() {
    // The open case: an oriented cycle, none through a multiple arrow.
    use mgs_core::conjecture7::{plan, run, Scope, Spec};
    let items = plan(&Spec { max_vertices: 4, max_multiplicity: 3, scope: Scope::Open, ..Spec::default() }).unwrap();
    assert_eq!(items.len(), 40);
    for item in &items {
        let row = run(item, &CheckOptions::default()).unwrap();
        assert!(row.verdict == "holds", "{}: {} {}", row.arrows, row.verdict, row.detail);
        assert!(!row.oriented_cycle.is_empty());
    }
}
