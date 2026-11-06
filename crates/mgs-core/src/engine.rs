//! The search: counting maximal green sequences by length on the oriented
//! exchange graph, deepening one length at a time until a stopping rule fires.
//!
//! BDP Proposition 2.13: the MGS of Q are the oriented paths from the framed
//! quiver to the all-red end of the oriented exchange graph, whose vertices
//! are framed quivers up to isomorphism fixing the frozen vertices; that
//! graph is acyclic (Prop. 2.14). So the number of continuations from a
//! state depends only on its class, and is memoised per class.
//!
//! The walk to length L fires one of three stopping rules:
//! - every green sequence has ended (all red, or broken by BHIT 3.3.2) by
//!   length L: the count is exact and unconditional;
//! - some MGS has been found, and the last `max_gap + 1` lengths have none.
//!   BDP Conjecture 2.22 (the No Gap Conjecture) says the lengths of MGS form
//!   an interval, which is `max_gap = 0`. A larger gap is a more cautious
//!   version of the same assumption. Under it, no longer MGS exist, the
//!   count is exact, and every green sequence still going is infinite. Those
//!   sequences are the "empirically infinite" ones, tentatively banned;
//! - the length cap or the state budget runs out.

use std::rc::Rc;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::bhit::banned_332;
use crate::counts::Counts;
use crate::framed::{Framed, Key};
use crate::matrix::Exchange;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchOptions {
    /// Longest run of empty lengths tolerated below the longest MGS. 0 is BDP's No Gap Conjecture.
    pub max_gap: u32,
    /// Never search beyond this length.
    pub max_length: u32,
    /// Stop when this many exchange-graph classes have been stored.
    pub max_states: usize,
    /// Skip steps that break BHIT 3.3.2 (a theorem, so this never loses an MGS).
    pub prune: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self { max_gap: 0, max_length: 80, max_states: 1_000_000, prune: true }
    }
}

/// What is known below one exchange-graph class, for continuations of length ≤ `r_done`.
#[derive(Debug)]
pub struct Entry {
    pub counts: Counts,
    pub r_done: u32,
    /// Some(h): every continuation ends within h steps, so `counts` is complete for every length.
    pub height: Option<u32>,
    /// Some branch below was abandoned because an entry outgrew 64 bits.
    pub overflowed: bool,
}

impl Entry {
    fn covers(&self, r: u32) -> bool {
        self.r_done >= r || self.height.is_some_and(|h| h <= r)
    }

    fn finished(counts: Counts) -> Self {
        Self { counts, r_done: u32::MAX, height: Some(0), overflowed: false }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutOfStates;

pub struct Engine {
    pub(crate) opts: SearchOptions,
    memo: FxHashMap<Key, (u64, Rc<Entry>)>,
}

impl Engine {
    pub fn new(opts: SearchOptions) -> Self {
        Self { opts, memo: FxHashMap::default() }
    }

    /// Exchange-graph classes stored so far.
    pub fn states(&self) -> usize {
        self.memo.len()
    }

    /// The stored entry for this state's class, if any.
    pub fn lookup(&self, s: &Framed) -> Option<Rc<Entry>> {
        self.memo.get(&s.class_key().0).map(|(_, e)| e.clone())
    }

    /// The steps from s worth taking: green, and (when pruning) not ruled out by BHIT 3.3.2.
    pub fn moves(&self, s: &Framed) -> Vec<usize> {
        s.greens().into_iter().filter(|&k| !self.opts.prune || banned_332(s, k).is_none()).collect()
    }

    /// Continuations from s of length ≤ r.
    pub fn query(&mut self, s: &Framed, r: u32) -> Result<Rc<Entry>, OutOfStates> {
        let (key, bhash) = s.class_key();
        let known = self.memo.get(&key);
        if let Some((h, e)) = known {
            assert_eq!(
                *h, bhash,
                "two states share their c-vectors but not their exchange matrix, against FZ IV (6.14); please report this quiver"
            );
            if e.covers(r) {
                return Ok(e.clone());
            }
        } else if self.memo.len() >= self.opts.max_states {
            return Err(OutOfStates);
        }
        let entry = if s.all_red() {
            Entry::finished(Counts::done())
        } else {
            let moves = self.moves(s);
            if moves.is_empty() {
                Entry::finished(Counts::default()) // every green step breaks BHIT 3.3.2: proved dead
            } else if r == 0 {
                Entry { counts: Counts::default(), r_done: 0, height: None, overflowed: false }
            } else {
                self.expand(s, &moves, r)?
            }
        };
        let e = Rc::new(entry);
        self.memo.insert(key, (bhash, e.clone()));
        Ok(e)
    }

    fn expand(&mut self, s: &Framed, moves: &[usize], r: u32) -> Result<Entry, OutOfStates> {
        let mut e = Entry { counts: Counts::default(), r_done: r, height: Some(0), overflowed: false };
        for &k in moves {
            let Ok(child) = s.mutate(k) else {
                e.overflowed = true;
                e.height = None;
                continue;
            };
            let c = self.query(&child, r - 1)?;
            e.counts.add_step(&c.counts, r - 1);
            e.overflowed |= c.overflowed;
            e.height = match (e.height, c.height) {
                (Some(a), Some(b)) => Some(a.max(b + 1)),
                _ => None,
            };
        }
        // A subtree taller than r had its counts truncated at r, so they are not complete.
        e.height = e.height.filter(|&h| h <= r);
        Ok(e)
    }
}

/// How the deepening stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    Exhausted,
    Gap,
    NoneFound,
    LengthCap,
    Budget,
    Overflow,
}

pub struct Outcome {
    pub stop: Stop,
    /// The last length L for which the walk finished; counts are exact up to it.
    pub searched_to: u32,
    pub root: Option<Rc<Entry>>,
}

/// Deepens from length 0 until a stopping rule fires.
pub fn search(q: &Exchange, opts: &SearchOptions) -> (Engine, Outcome) {
    let mut engine = Engine::new(opts.clone());
    let start = Framed::new(q);
    let mut last: Option<(u32, Rc<Entry>)> = None;
    let outcome = |stop, last: Option<(u32, Rc<Entry>)>| {
        let (searched_to, root) = last.map_or((0, None), |(l, e)| (l, Some(e)));
        Outcome { stop, searched_to, root }
    };
    for l in 0..=opts.max_length {
        let Ok(e) = engine.query(&start, l) else {
            return (engine, outcome(Stop::Budget, last));
        };
        last = Some((l, e.clone()));
        if e.overflowed {
            return (engine, outcome(Stop::Overflow, last));
        }
        if e.height.is_some_and(|h| h <= l) {
            return (engine, outcome(Stop::Exhausted, last));
        }
        if let Some((_, longest)) = e.counts.range_within(l) {
            if l - longest > opts.max_gap {
                return (engine, outcome(Stop::Gap, last));
            }
        }
    }
    let found = last.as_ref().is_some_and(|(l, e)| e.counts.any_within(*l));
    (engine, outcome(if found { Stop::LengthCap } else { Stop::NoneFound }, last))
}
