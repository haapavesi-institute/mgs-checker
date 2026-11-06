//! The inheritance conjecture and its patterns, on every small quiver: the
//! evidence behind `mgs sincere`, kept as tests.

use mgs_core::sincere::{plan, run, Family, Row, Spec};
use mgs_core::{Exchange, SearchOptions};

fn survey(family: Family, min_vertices: usize, max_vertices: usize, max_multiplicity: i64) -> Vec<Row> {
    let spec = Spec { min_vertices, max_vertices, max_multiplicity, family };
    plan(&spec).unwrap().iter().map(|i| run(i, &SearchOptions::default()).unwrap()).collect()
}

#[test]
fn inheritance_holds_and_the_minimal_quivers_are_the_known_patterns_up_to_five_vertices() {
    let mut rows = survey(Family::All, 2, 4, 2);
    rows.extend(survey(Family::All, 5, 5, 1));
    assert_eq!(rows.iter().filter(|r| r.vertices == 5).count(), 267);
    let mut extensions = 0;
    for r in &rows {
        assert!(r.settled, "{}", r.arrows);
        assert!(!r.inheritance_fails, "{}: Q∖{:?} has no sincere c-vector but Q has", r.arrows, r.bare_below);
        assert!(r.covered.is_empty(), "{}: covered {:?}", r.arrows, r.covered);
        assert!(!r.minimal || r.known_minimal, "{}: a minimal quiver that is not a known pattern", r.arrows);
        // The converse on these quivers: no pattern, so a sincere c-vector.
        assert_eq!(r.sincere.is_empty(), !r.patterns.is_empty(), "{}", r.arrows);
        extensions += r.bare_below.len();
    }
    assert!(extensions > 1500, "{extensions} one-vertex extensions checked");
}

#[test]
fn a_bare_cycle_has_no_sincere_c_vector_exactly_when_it_is_of_type_p_1_or_the_alternating_square() {
    let rows = survey(Family::Cycles, 3, 6, 1);
    assert_eq!(rows.len(), 1 + 3 + 3 + 7);
    for r in &rows {
        assert!(r.proved, "{}: tame, so exact by the No Gap theorem", r.arrows);
        assert_eq!(r.sincere.is_empty(), !r.patterns.is_empty(), "{}", r.arrows);
        // Where there are sincere c-vectors, they stay within δ plus a little: no entry above 2.
        assert!(r.sincere.iter().flatten().all(|&x| x <= 2), "{}", r.arrows);
    }
}

#[test]
fn the_transitive_tournament_has_fibonacci_many_mgs_on_simples_and_arrows() {
    let rows = survey(Family::Tournament, 2, 6, 1);
    let totals: Vec<&str> = rows.iter().map(|r| r.total.as_str()).collect();
    assert_eq!(totals, ["2", "5", "13", "34", "89"]); // F(2n − 1)
    for r in &rows {
        let n = r.vertices;
        assert_eq!(r.step_c_vectors, n * (n + 1) / 2, "e_i and e_i + e_j only");
        assert_eq!(r.sincere.is_empty(), n >= 3);
    }
}

#[test]
fn a_minimal_quiver_that_is_not_a_cycle() {
    // The one-source square with an arrow out of its source and one into its sink: wild, no sincere
    // c-vector, though each of its connected Q ∖ v has one. Found by the six-vertex survey.
    let q = Exchange::from_arrow_text("1->2, 1->3, 1->6, 2->5, 3->5, 4->5", None).unwrap();
    let r = run(&mgs_core::sincere::Item { id: 0, matrix: q.rows() }, &SearchOptions::default()).unwrap();
    assert!(r.minimal && !r.known_minimal && r.patterns.is_empty() && r.sincere.is_empty(), "{r:?}");
}
