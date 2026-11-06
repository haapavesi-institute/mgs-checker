//! Reading results back out of a finished search: a sample of MGS, and the
//! banned prefixes (green prefixes that can never be finished), with the
//! reason each is banned and an example of the green sequence running on.
//!
//! A prefix the search only shows to have no continuation within the length
//! bound is "tentatively" banned. Before saying so, a separate search from
//! that prefix, with no length bound but a small state budget, tries to prove
//! more: that every continuation reaches a step BHIT 3.3.2 rules out (then
//! the ban is a theorem), or that some continuation is a maximal green
//! sequence longer than the bound (then the gap assumption is false here).

use serde::{Deserialize, Serialize};

use crate::bhit::banned_332;
use crate::engine::{Engine, SearchOptions};
use crate::framed::Framed;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum BanReason {
    /// The last step breaks BHIT 3.3.2: `vertex` is the target of `arrows` arrows from `from`.
    Bhit332 { vertex: usize, from: usize, arrows: i64 },
    /// Every continuation reaches a step that breaks BHIT 3.3.2, within `within` more steps.
    DeadEnd { within: u32 },
    /// No continuation finishes within the bound, and a No Gap theorem makes the bound sharp.
    NoGapTheorem,
    /// No continuation finishes within the bound: banned only if the lengths of MGS have no gap longer than `max_gap`.
    Tentative { max_gap: u32 },
    /// Some continuation is an MGS of total length `length`, beyond the bound: the gap assumption FAILS for this quiver.
    LongerMgs { length: u32 },
    /// A matrix entry outgrew 64 bits on this branch; nothing is known.
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TailEnd {
    /// Still green after the whole tail.
    StillGreen,
    /// Every green step left breaks BHIT 3.3.2.
    Dead,
    /// Reached all red.
    AllRed,
    Overflow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Banned {
    /// 1-based.
    pub prefix: Vec<usize>,
    pub reason: BanReason,
    /// A green continuation, 1-based (see `Engine::tail`).
    pub tail: Vec<usize>,
    pub tail_end: TailEnd,
}

/// How tentative bans are labelled, from the search's stopping rule.
#[derive(Clone, Copy, Debug)]
pub enum Bound {
    Theorem,
    Assumed { max_gap: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BanOptions {
    /// List prefixes up to this length.
    pub depth: usize,
    pub limit: usize,
    /// Length of the green continuation shown for each.
    pub tail: usize,
    /// State budget for trying to prove each tentative ban (0: don't try).
    pub prove_states: usize,
    /// Longest continuation that proof search looks at.
    pub prove_length: u32,
}

impl Engine {
    /// Up to `limit` MGS of length ≤ bound, in lexicographic order, 1-based.
    pub fn sample(&self, start: &Framed, bound: u32, limit: usize) -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        self.sample_from(start, bound, limit, &mut Vec::new(), &mut out);
        out
    }

    fn sample_from(&self, s: &Framed, r: u32, limit: usize, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if out.len() >= limit {
            return;
        }
        if s.all_red() {
            out.push(path.iter().map(|v| v + 1).collect());
            return;
        }
        if r == 0 {
            return;
        }
        for k in self.moves(s) {
            let Ok(child) = s.mutate(k) else { continue };
            if self.lookup(&child).is_some_and(|e| e.counts.any_within(r - 1)) {
                path.push(k);
                self.sample_from(&child, r - 1, limit, path, out);
                path.pop();
            }
        }
    }

    /// Minimal banned prefixes of length ≤ `opts.depth`: every proper prefix can still be finished.
    pub fn banned(&self, start: &Framed, bound: u32, kind: Bound, opts: &BanOptions) -> Vec<Banned> {
        let mut out = Vec::new();
        self.banned_from(start, bound, kind, opts, &mut Vec::new(), &mut out);
        out
    }

    fn banned_from(&self, s: &Framed, r: u32, kind: Bound, opts: &BanOptions, path: &mut Vec<usize>, out: &mut Vec<Banned>) {
        if s.all_red() || r == 0 {
            return;
        }
        for k in s.greens() {
            if out.len() >= opts.limit {
                return;
            }
            path.push(k);
            let found = match (banned_332(s, k), s.mutate(k)) {
                (Some((from, arrows)), _) => Some((BanReason::Bhit332 { vertex: k + 1, from: from + 1, arrows }, None)),
                (None, Err(_)) => Some((BanReason::Unresolved, None)),
                (None, Ok(child)) => match self.lookup(&child) {
                    Some(e) if e.counts.any_within(r - 1) => {
                        if path.len() < opts.depth {
                            self.banned_from(&child, r - 1, kind, opts, path, out);
                        }
                        None
                    }
                    Some(e) if e.height.is_some() && e.counts.is_empty() => {
                        Some((BanReason::DeadEnd { within: e.height.unwrap_or(0) }, Some((child, None))))
                    }
                    Some(_) => {
                        let (reason, local) = self.settle(&child, path.len() as u32, kind, opts);
                        Some((reason, Some((child, local))))
                    }
                    None => Some((BanReason::Unresolved, None)),
                },
            };
            if let Some((reason, child)) = found {
                let (tail, tail_end) = child.map_or((Vec::new(), TailEnd::Dead), |(c, local)| self.tail(&c, opts.tail, local.as_ref()));
                out.push(Banned { prefix: path.iter().map(|v| v + 1).collect(), reason, tail, tail_end });
            }
            path.pop();
        }
    }

    /// A prefix with no continuation within the bound: try to prove more.
    /// Also returns the local search, whose memo knows more dead ends.
    fn settle(&self, child: &Framed, depth: u32, kind: Bound, opts: &BanOptions) -> (BanReason, Option<Engine>) {
        let fallback = match kind {
            Bound::Theorem => BanReason::NoGapTheorem,
            Bound::Assumed { max_gap } => BanReason::Tentative { max_gap },
        };
        if opts.prove_states == 0 {
            return (fallback, None);
        }
        let mut local = Engine::new(SearchOptions { max_states: opts.prove_states, prune: true, ..self.opts.clone() });
        let reason = match local.query(child, opts.prove_length) {
            Ok(e) => match (e.counts.range_within(opts.prove_length), e.height) {
                (Some((first, _)), _) => BanReason::LongerMgs { length: depth + first },
                (None, Some(within)) => BanReason::DeadEnd { within },
                (None, None) => fallback,
            },
            Err(_) => fallback,
        };
        (reason, Some(local))
    }

    /// Runs on from s for up to `steps` steps, mutating the first green vertex
    /// that BHIT 3.3.2 allows and that the search has not proved a dead end,
    /// so the tail shows a green sequence that keeps going if there is one.
    pub fn tail(&self, s: &Framed, steps: usize, local: Option<&Engine>) -> (Vec<usize>, TailEnd) {
        let dead = |f: &Framed| {
            [Some(self), local].into_iter().flatten().any(|g| g.lookup(f).is_some_and(|e| e.height.is_some() && e.counts.is_empty()))
        };
        let mut cur = s.clone();
        let mut tail = Vec::new();
        for _ in 0..steps {
            if cur.all_red() {
                return (tail, TailEnd::AllRed);
            }
            let (mut pick, mut fallback) = (None, None);
            for k in cur.greens().into_iter().filter(|&k| banned_332(&cur, k).is_none()) {
                let Ok(next) = cur.mutate(k) else {
                    return (tail, TailEnd::Overflow);
                };
                if dead(&next) {
                    fallback.get_or_insert((k, next));
                } else {
                    pick = Some((k, next));
                    break;
                }
            }
            let Some((k, next)) = pick.or(fallback) else {
                return (tail, TailEnd::Dead);
            };
            cur = next;
            tail.push(k + 1);
        }
        if cur.all_red() {
            (tail, TailEnd::AllRed)
        } else {
            (tail, TailEnd::StillGreen)
        }
    }
}
