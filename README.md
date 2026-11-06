# MGS Checker

Counts every maximal green sequence (MGS) of an acyclic quiver by length,
exactly, from its exchange matrix. It also lists the green prefixes that can
never be finished, and says how sure it is: proved, proved by a theorem, or
exact on the assumption of BDP's No Gap Conjecture with a gap you choose. And
it records what the MGS pass through: the c-vectors of their steps, the
maximal ones under the entrywise order, and the exchange-graph classes they
visit. Quivers with an oriented cycle are refused.

- `mgs check matrix.csv`: counts, maximal c-vectors, a sample and banned prefixes for one quiver
- `mgs verify matrix.csv --sequence 1,3,2,3`: check one sequence
- `mgs explore --atilde 2-4 --steps 1`: the fixed explorations around tame quivers
- `mgs conjecture7 --max-vertices 5`: Quiver Quest conjecture #7 on every small acyclic quiver with a multiple arrow
- `web/`: the same engine compiled to WebAssembly, in a static page that runs in the browser.
  It is live at **https://haapavesi-institute.github.io/mgs-checker/**.

## Quick start

```sh
cargo build --release
./target/release/mgs check examples/double-then-single.csv
./target/release/mgs check --arrows "1->2, 2->3, 1->3, 3->4" --max-gap 2
./target/release/mgs explore --kronecker --atilde 2-3 --dtilde 4 --format csv --out explore.csv
./target/release/mgs conjecture7 --max-vertices 5 --jobs 8 --format csv --out conjecture7.csv

./scripts/build-web.sh                 # needs wasm-pack
python3 -m http.server -d web 8080     # then open http://localhost:8080
```

Every push to `main` rebuilds the page and deploys it to GitHub Pages
(`.github/workflows/pages.yml`). Pull requests build it without deploying.

## Input

A CSV file holding the exchange matrix, one row per line, with
b<sub>ij</sub> = #(i → j) − #(j → i). It must be square and skew-symmetric.
Commas, semicolons, tabs or spaces separate entries. Blank lines, `#` comments
and an all-text header row are skipped. `1 ⇉ 2 → 3` is

```text
0,2,0
-2,0,1
0,-1,0
```

`--arrows "1->2*2, 2->3"` gives the same quiver inline. Vertices are numbered
from 1 everywhere.

**The quiver must be acyclic: it may have no oriented cycle.** Cycles of the
underlying graph are fine, as in Ã₂ = 1 → 2 → 3 with 1 → 3. A quiver with an
oriented cycle, such as the Markov quiver, is refused, and the error names a
cycle:
`the quiver has an oriented cycle 1 → 2 → 3 → 1; this checker accepts only acyclic quivers`.
Everything the checker leans on is stated for acyclic quivers: c-vectors are
real Schur roots, the shortest MGS has length n, and the Dynkin, tame and No
Gap theorems all concern path algebras. The one exception is
`mgs conjecture7`, whose open scope needs oriented cycles (see below).

## What a check does

1. **It walks the oriented exchange graph.** By BDP Prop. 2.13–2.14, MGS are
   paths from the framed quiver to the all-red end. States are framed quivers
   up to relabelling, and they are memoised by their sorted c-vectors.
   For skew-symmetric B the c-vectors determine the exchange matrix
   (B<sub>t</sub> = C<sub>t</sub> B<sub>0</sub> C<sub>t</sub><sup>T</sup>,
   from FZ IV (6.14) and Nakanishi–Zelevinsky). The search still checks a
   hash of B on every reuse instead of trusting it.
2. **It prunes with BHIT Corollary 3.3.2**, a theorem with no hypotheses: no
   MGS ever mutates a vertex that is, at that moment, the target of two or more
   parallel arrows. Pruning never loses an MGS. `--no-prune` turns it off.
3. **It deepens one length at a time and stops by one of these rules:**

| Status | Meaning |
|---|---|
| `exhausted` | Every green sequence ended, either all red or at a step BHIT 3.3.2 rules out. **Exact, no assumption.** |
| `no-gap-theorem` | Some MGS were found, then a length with none. A No Gap theorem covers the quiver: for tame quivers, Hermes–Igusa Cor. 4.0.2. **Exact.** |
| `conditional` | Some MGS were found, then `max_gap + 1` empty lengths in a row. **Exact if the lengths of MGS have no gap longer than `max_gap`.** BDP Conjecture 2.22, the No Gap Conjecture, is `max_gap = 0`. Green sequences still running at that point are **empirically infinite**. |
| `assumption-fails` | A tentatively banned prefix turned out to finish as a longer MGS, so the gap assumption is false for this quiver. |
| `none-found`, `length-cap`, `budget`, `overflow` | Incomplete, for the stated reason. |

4. **It sorts the banned prefixes**, the minimal green prefixes that can never
   be finished:
   - `bhit332`: the last step breaks BHIT 3.3.2. Proved.
   - `dead-end`: every continuation reaches such a step. Proved.
   - `no-gap-theorem`: no continuation within the bound, and the No Gap theorem
     applies. Proved.
   - `tentative`: no continuation within the bound. Banned only on the gap
     assumption.

   Before a ban is called tentative, a second search from that prefix, with no
   length bound and a small budget (`prove_states`), tries to settle it. It can
   upgrade the ban to a proved `dead-end`, or find a longer MGS and refute the
   assumption. Each banned prefix comes with a green continuation that keeps
   going when one exists, e.g. `(1, 3, 2, 1, 3, 2, …)`.
5. **It records what the MGS pass through.** Each step of an MGS mutates a
   green vertex, whose c-vector is then a positive real Schur root: the
   dimension vector of the brick labelling the step. Over every MGS counted, the
   report gives:
   - how many distinct step c-vectors there are;
   - the maximal ones under the entrywise order. The order is partial, so
     several can be maximal, none above another. Each comes with a shortest
     green prefix whose last step has that c-vector, and that prefix can be
     finished;
   - their largest entry;
   - how many exchange-graph classes the MGS pass through;
   - the most parallel arrows in any exchange matrix along the way.

   For a Dynkin quiver the longest MGS mutates every positive root (BDP Thm
   4.4), so there are |Φ⁺| step c-vectors, and the only maximal one is the
   highest root.
6. **It compares against the literature** whenever the result is settled:
   - acyclic quivers: shortest MGS length n (BDP Lemma 2.20, Cor. 2.18);
   - Dynkin quivers: longest |Φ⁺| (BDP Thm 4.4);
   - tame quivers: longest from Kase–Nakashima (arXiv:2010.13996): (a+b−1)(a+b)/2 + ab for Ã(a,b), 2n² − 2n − 2 for D̃<sub>n</sub>, and 78, 159, 390 for Ẽ<sub>6,7,8</sub>.

Counts are exact integers of any size. Matrix entries are 64-bit and checked.
A branch whose entries outgrow that is reported as unresolved, never rounded.

## Explorations

`mgs explore` and the page's Explore section start from tame quivers:
- the Kronecker quiver;
- every acyclic orientation, up to relabelling, of Ã<sub>n</sub>, D̃<sub>n</sub>, Ẽ<sub>6,7,8</sub> (`--orientations one` picks a single one).

They apply 0, 1 or 2 edge changes. A change either raises the multiplicity of
an existing arrow bundle by one, or adds a single arrow, in either direction,
between two vertices that had none. A new arrow that would close an oriented
cycle is never added, so every quiver explored is acyclic. Every quiver reached is checked once, up
to relabelling, or up to relabelling and reversal with `--up-to-opposite`
(BDP Prop. 2.15). The table records, for each quiver:
- where it came from;
- its type;
- whether the result is proved or conditional;
- the counts by length, and the longest MGS against the literature's value;
- the maximal step c-vectors, how many step c-vectors there are, their largest
  entry, the classes on MGS and the most parallel arrows met;
- the first moves that can never be finished, and why.

## Conjecture #7: deleting the multiple arrows

Quiver Quest conjecture #7 says: if no multiple arrow of Q lies on an oriented
cycle, then every MGS of Q is an MGS of Q′, the quiver with every bundle of two
or more arrows deleted, with the same c-vector at every step.

For acyclic quivers (no oriented cycle; cycles of the underlying graph, as in
Ã<sub>n</sub>, are fine) that is a theorem: Igusa–Zhou (arXiv:1902.07375v1)
Theorem 3.12, "MGSs of an acyclic quiver Q are a subset of the set of Q-ME-free
MGSs of its ME-free version, Q′", with the same C-matrices. Q-ME-free means no
c-vector has both ends of a multiple arrow in its support. **The open part is
quivers with an oriented cycle that avoids every multiple arrow**, and
`mgs conjecture7` is the one place this checker accepts them. Every other
command still refuses oriented cycles.

`mgs conjecture7` runs in one of three scopes (`--scope`):

| Scope | Quivers | What a failure would mean |
|---|---|---|
| `acyclic` (default) | no oriented cycle | a bug: Igusa–Zhou's theorem covers these |
| `open` | an oriented cycle, none through a multiple arrow | a counterexample to #7 |
| `control` | a multiple arrow on an oriented cycle | nothing: #7 claims nothing here, and fails (BHIT's Figure 3) |

- `mgs conjecture7 --arrows "1->2*2, 1->3, 3->2, 2->4, 4->3"` (or a CSV file)
  checks one quiver with a multiple arrow, in whichever scope it falls.
  `mgs check` also reports "Conj. #7" for acyclic quivers with a multiple arrow.
- `mgs conjecture7 --scope open --max-vertices N --max-multiplicity M` checks
  every connected quiver in the scope with 2 to N vertices, at most M parallel
  arrows and a multiple arrow, once each up to relabelling.
  - `--sample K --seed S` draws K random matrices per vertex count instead of
    listing every one. It is needed once there are more than 20 million
    matrices to list (with oriented cycles allowed, already at 5 vertices and
    3 arrows).
  - `--class on-cycle` keeps quivers with a multiple arrow on a cycle of the
    underlying graph that is not oriented. `--class on-cycle-no-cut` also
    drops those with an Igusa–Zhou partition cut (their Theorem 4.3).
  - Each quiver gets a state budget of 200 000 classes (`--max-states`).
- The page's Conjecture #7 section does the same, up to 5 vertices for
  `acyclic` and 4 for the others. It can also check the quiver in the main
  box, whatever its scope.

For each quiver the check walks every MGS the search counted, plays each step
on Q′ at the same time, and stops at the first step where Q′ disagrees: a step
that is red there, a different c-vector, or an MGS of Q that leaves Q′ with
green vertices. It reports the shortest such prefix. It also flags a step
c-vector covering both ends of a multiple arrow, among the steps it walked
before stopping. With oriented cycles there is no No Gap theorem and no
"shortest MGS has length n", so more results are conditional on the gap
assumption or unsettled. BHIT 3.3.2 pruning and sign-coherence hold for every
quiver, so the search itself is unchanged.

**Open scope: no counterexample so far.**

| Vertices | Most arrows | Quivers | Result |
|---|---|---|---|
| 4 | 3 | 40 | all hold: 36 exact, 4 if no gap |
| 5 | 2 | 2 194, every one | all hold: 1 100 exact, 1 094 if no gap, in 2.9 s |
| 5, a sample of 5 000 000 matrices (`--seed 1`) | 3 | 14 213 | all hold: 9 888 exact, 4 325 if no gap, in 2.8 s |
| 6, a sample of 5 000 000 matrices (`--seed 1`) | 2 | 76 034 | 25 114 hold exactly and 50 908 if no gap; 12 unsettled, with no MGS up to length 80; 68 s |

On 4 vertices with `--class on-cycle-no-cut` the open scope has 8 quivers
with at most 2 arrows and 28 with at most 3. Those are the counts in the board
post, whose survey sampled only 110 of the 1 461 five-vertex ones.

**Controls, 4 vertices, at most 2 arrows, `--class on-cycle-no-cut`:** 99
quivers. #7 fails on 76 of them, exactly as in the board post. 16 hold, 2 hold
if no gap, 3 have no MGS and 2 are unsettled.

**Acyclic scope (Igusa–Zhou's theorem):** every run holds, as it must.

| Vertices | Most arrows | Quivers | Result |
|---|---|---|---|
| 2–4 | 2 | 392 | all hold: 346 exact, 46 if no gap |
| 2–4 | 3 | 2 688 | all hold: 2 556 exact, 132 if no gap |
| 5 | 2 | 25 694 | all hold: 18 984 exact, 6 710 if no gap, in 1.1 s on 8 threads |
| 6, `--class on-cycle-no-cut` | 2 | 2 501 668 | all hold: 1 203 271 exact, 1 298 397 if no gap, in 17 min on 8 threads (5.4 GB) |

## Validation

`cargo test --release` reproduces:

| Quiver | Result | Source |
|---|---|---|
| A₅ linear, the other A₄ orientations | counts by length | BDP v1 Figs. 23–24 |
| E₆, E₇ | 253 085 705 387 and 372 133 972 845 031 649 851 164 | BDP v1 App. B.2.3 |
| 1 → 2 → 3 ⇉ 4 | 1, 7, 6, 7 at lengths 4–7, exact via BHIT pruning | BDP Ex. 5.5 |
| E₆, E₇, D₅ | 36, 63 and 20 step c-vectors, all of Φ⁺; the only maximal one is the highest root | BDP Thm 4.4; Bourbaki's tables |
| The oriented 3-cycle, the Markov quiver, a 4-vertex quiver with a 3-cycle | refused, with the cycle named | |
| Ã₂, both squares, Ã(3,1), Ã(4,1), Ã(3,2) | 5, 100, 100, 75, 4 882, 11 346 | BDP Ex. 5.3; Quiver Quest's engine |
| 12 small quivers, finite type or pruned to exhaustion | counts by length | Quiver Quest's TypeScript engine, an independent implementation |

[Quiver Quest](https://www.peacecatgames.com/quiver-quest) is a separate
TypeScript implementation with exact BigInt counts. Its
source is not public; the expected values are written out in
`crates/mgs-core/tests/literature.rs`.

Beyond the tests:
- E₈ comes out at exactly 5 641 983 835 250 421 910 131 380 897 514 893 004 210 307 550 157 089 MGS in 4.8 s (`--max-length 130`). That agrees with BDP's ≈ 5.641·10⁵¹ and visits exactly E₈'s 25 080 clusters. The step c-vectors are its 120 positive roots, and the only maximal one is the highest root, (2, 3, 4, 5, 6, 4, 2, 3) for the path 1 → ⋯ → 7 with 5 → 8.
- Tame quivers can have several maximal step c-vectors. Ã₂ (1 → 2 → 3, 1 → 3) has three: (1, 1, 0), (1, 0, 1) and (0, 1, 1). D̃₄ with every arrow into the centre has four, (2, 1, 1, 1, 3) and its permutations, among 33 step c-vectors. Ẽ₆ in the orientation below has just one, (10, 6, 3, 6, 3, 6, 3) = 3δ + e₁, among 137.
- The longest MGS of D̃₄ (22), D̃₅ (38), Ẽ₆ (78), Ẽ₇ (159), Ã(4,2) (23) and Ã(5,1) (20) all agree with Kase–Nakashima.
- The totals for these tame quivers are exact by the No Gap theorem:

  | Quiver | MGS | Time |
  |---|---|---|
  | Ã(5,1) | 1 645 136 | 24 ms |
  | D̃₄ | 641 496 | 7 ms |
  | D̃₅ | 2 468 030 689 952 | 0.1 s |
  | Ẽ₆, as 2→1, 3→2, 4→1, 5→4, 6→1, 7→6 | 3 100 003 747 781 579 442 794 657 607 372 | 3 s |
  | Ẽ₇, as 2→1, 3→1, 4→3, 5→4, 6→1, 7→6, 8→7 | 14 285 849 796 116 326 214 400 648 661 951 543 760 514 582 858 467 051 984 158 525 824 879 244 | 98 s, 372 172 classes |

## Research examples

`crates/mgs-core/examples/` has `cycle_cut.rs` and `cycle_cut_detail.rs`.
They are the survey behind the cycle-cutting conjecture on the Quiver Quest
board, posted as disproved, and its surviving Ã<sub>n−1,1</sub> case. Like
everything here they look only at acyclic quivers.

### One cycle: Ã with trees attached (`unicyclic.rs`)

`cargo run --release -p mgs-core --example unicyclic -- N M` surveys every
connected acyclic quiver on N vertices, with at most M parallel arrows, whose
underlying graph has exactly one cycle C. Each such quiver has an
Ã<sub>n</sub> subquiver, with trees attached.

**What is known.** Whether a wild acyclic quiver with four or more vertices
has finitely many MGS is open (BDP, §5). Known cases:
- Dynkin and affine quivers (BDP Thms 4.1 and 5.2);
- quivers with at most three vertices (BDP Thm 5.4);
- quivers whose ME-free version is finite (Igusa–Zhou Cor 3.14);
- two orientations of Ã₂ with a pendant vertex (Hofmann, Bonn thesis 2021,
  Ex. 4.33). He shows that no sincere brick occurs in their MGS.

**A reduction.** For a vertex v, keeping the steps of an MGS whose c-vector
has c<sub>v</sub> = 0 gives an MGS of Q ∖ v. This is Muller's restriction, which
is the lattice quotient tors kQ → tors kQ/⟨e<sub>v</sub>⟩ of
Demonet–Iyama–Reading–Reiten–Thomas. So every step c-vector of Q is either
a step c-vector of some Q ∖ v, or sincere. An MGS is a sequence of distinct
c-vectors, so MGS(Q) is finite once every Q ∖ v is MGS-finite and only
finitely many sincere c-vectors occur. In a one-cycle quiver a sincere
c-vector covers C.

**Observations on 3–6 vertices**
- Restriction to C and to every Q ∖ v always gives an MGS: no exception.
- **No c-vector covers C when C is of type Ã(p, 1)**, that is, when all but
  one of its arrows point the same way round. This held on 2 123 quivers: 27 on
  3 vertices with ≤ 3 arrows, 567 on 4 with ≤ 3, 1 296 on 5 with ≤ 2, and 233
  on 6 with single arrows.
  - When C carries a multiple arrow this is Igusa–Zhou Thm 3.12, since the
    c-vectors are ME-free.
  - Consequently the product of the restrictions, MGS(Q) → ∏<sub>v ∈ C</sub>
    MGS(Q ∖ v), is finite-to-one. In fact it was injective on all 1 968 of
    these quivers whose MGS were listed in full. So such a Q is MGS-finite
    whenever its forests Q ∖ v are.
- **No c-vector is ≥ 2δ<sub>C</sub> on C when every Q ∖ v (v on C) is a
  Dynkin forest.** This held on 344 simply laced quivers with 4–6 vertices.
  Without that hypothesis it fails. With a D̃₄ attached,
  1 → 2, 1 → 3, 2 → 4, 3 → 4, 4 → 5, 4 → 6 has the c-vector (4, 2, 2, 3, 2, 1).
- **Cutting an arrow α of C:** deleting the steps whose c-vector has both ends
  of α in its support always leaves an MGS of the tree Q ∖ α, with no exception.
  The map is not onto, and its fibres reach 371 on 6 vertices.
- Simply laced quivers stop by the No Gap rule (max gap 0). On 5 vertices
  the totals do not change with `--max-gap 8`.

## References

- Brüstle, Dupont, Pérotin, *On maximal green sequences*, IMRN 2014 (arXiv:1205.2050).
- Brüstle, Hermes, Igusa, Todorov, *Semi-invariant pictures and two conjectures on maximal green sequences*, J. Algebra 473 (2017).
- Hermes, Igusa, *The No Gap Conjecture for tame hereditary algebras*, arXiv:1601.04054.
- Kase, Nakashima, *Lengths of maximal green sequences for tame path algebras*, arXiv:2010.13996 (preprint).
- Igusa, Zhou, *Maximal green sequences of quivers with multiple edges*, arXiv:1902.07375 (preprint).
- Muller, *The existence of a maximal green sequence is not invariant under quiver mutation*, Electron. J. Combin. 23(2) (2016), arXiv:1503.04675.
- Demonet, Iyama, Reading, Reiten, Thomas, *Lattice theory of torsion classes*, arXiv:1711.01785.
- Hofmann, PhD thesis, Universität Bonn (2021), d-nb.info/1270571427.

## License

Copyright 2026 Ian Joiner. Licensed under the
[Apache License, Version 2.0](LICENSE); see also [NOTICE](NOTICE). Unless you explicitly
state otherwise, any contribution you submit for inclusion in this work is
licensed under the same terms, without any additional terms or conditions
(section 5 of the license).
