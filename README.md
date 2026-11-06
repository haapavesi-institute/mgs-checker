# MGS Checker

Counts every maximal green sequence (MGS) of a quiver by length, exactly,
from its exchange matrix. It also lists the green prefixes that can never be
finished, and says how sure it is: proved, proved by a theorem, or exact on the
assumption of BDP's No Gap Conjecture with a gap you choose.

- `mgs check matrix.csv`: count, sample and banned prefixes for one quiver
- `mgs verify matrix.csv --sequence 1,3,2,3`: check one sequence
- `mgs explore --atilde 2-4 --steps 1`: the fixed explorations around tame quivers
- `web/`: the same engine compiled to WebAssembly, in a static page that runs in the browser.
  It is live at **https://peace-cat-games.github.io/mgs-checker/**.

## Quick start

```sh
cargo build --release
./target/release/mgs check examples/double-then-single.csv
./target/release/mgs check --arrows "1->2, 2->3, 1->3, 3->4" --max-gap 2
./target/release/mgs explore --kronecker --atilde 2-3 --dtilde 4 --format csv --out explore.csv

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
5. **It compares against the literature** whenever the result is settled:
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
between two vertices that had none. Every quiver reached is checked once, up
to relabelling, or up to relabelling and reversal with `--up-to-opposite`
(BDP Prop. 2.15). The table records, for each quiver:
- where it came from;
- its type;
- whether the result is proved or conditional;
- the counts by length, and the longest MGS against the literature's value;
- the first moves that can never be finished, and why.

## Validation

`cargo test --release` reproduces:

| Quiver | Result | Source |
|---|---|---|
| A₅ linear, the other A₄ orientations | counts by length | BDP v1 Figs. 23–24 |
| E₆, E₇ | 253 085 705 387 and 372 133 972 845 031 649 851 164 | BDP v1 App. B.2.3 |
| 1 → 2 → 3 ⇉ 4 | 1, 7, 6, 7 at lengths 4–7, exact via BHIT pruning | BDP Ex. 5.5 |
| Markov quiver | none | BDP Prop. 2.21 |
| Ã₂, both squares, Ã(3,1), Ã(4,1), Ã(3,2) | 5, 100, 100, 75, 4 882, 11 346 | BDP Ex. 5.3; Quiver Quest's engine |
| 15 small quivers, finite type or pruned to exhaustion | counts by length | Quiver Quest's TypeScript engine, an independent implementation |

[Quiver Quest](https://www.peacecatgames.com/quiver-quest) is a separate
TypeScript implementation with exact BigInt counts. Its
source is not public; the expected values are written out in
`crates/mgs-core/tests/literature.rs`.

Beyond the tests:
- E₈ comes out at exactly 5 641 983 835 250 421 910 131 380 897 514 893 004 210 307 550 157 089 MGS in 4.8 s. That agrees with BDP's ≈ 5.641·10⁵¹ and visits exactly E₈'s 25 080 clusters.
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
board, posted as disproved, and its surviving Ã<sub>n−1,1</sub> case.

## References

- Brüstle, Dupont, Pérotin, *On maximal green sequences*, IMRN 2014 (arXiv:1205.2050).
- Brüstle, Hermes, Igusa, Todorov, *Semi-invariant pictures and two conjectures on maximal green sequences*, J. Algebra 473 (2017).
- Hermes, Igusa, *The No Gap Conjecture for tame hereditary algebras*, arXiv:1601.04054.
- Kase, Nakashima, *Lengths of maximal green sequences for tame path algebras*, arXiv:2010.13996 (preprint).
- Igusa, Zhou, *Maximal green sequences of quivers with multiple edges*, arXiv:1902.07375 (preprint).

## License

Licensed under the [Apache License, Version 2.0](LICENSE). Unless you explicitly
state otherwise, any contribution you submit for inclusion in this work is
licensed under the same terms, without any additional terms or conditions
(section 5 of the license).
