//! Maximal green sequences of acyclic quivers given by their exchange
//! matrices. A quiver with an oriented cycle is refused ([`acyclic::admit`]).
//!
//! - [`report::check`] counts every MGS by length. It walks the oriented
//!   exchange graph one length deeper at a time. It stops when every green
//!   sequence has ended, or when BDP's No Gap Conjecture, relaxed by a chosen
//!   `max_gap`, says no longer MGS can exist. It says which of the two
//!   happened, and lists the banned prefixes: those BHIT 3.3.2 rules out,
//!   and the empirically infinite ones.
//!   It also reports what the MGS pass through ([`seeds`]): their step
//!   c-vectors and the maximal ones under the entrywise order.
//! - [`verify::verify`] checks one given sequence.
//! - [`explore`] runs the fixed explorations around tame quivers.
//! - [`conjecture7`] checks Quiver Quest conjecture #7 ([`cutting`]) on
//!   every small acyclic quiver with a multiple arrow.
//!
//! Convention: Brüstle–Dupont–Pérotin framing (green means c-vector ≥ 0), and
//! b_ij = #(i → j) − #(j → i). Vertices are 1-based in every input and output.

pub mod acyclic;
pub mod bhit;
pub mod canonical;
pub mod classify;
pub mod conjecture7;
pub mod counts;
pub mod csv;
pub mod cutting;
pub mod engine;
pub mod enumerate;
pub mod explore;
pub mod families;
pub mod framed;
pub mod known;
pub mod matrix;
pub mod modify;
pub mod report;
pub mod seeds;
pub mod verify;
pub mod walk;

pub use acyclic::OrientedCycle;
pub use engine::SearchOptions;
pub use matrix::{Exchange, MatrixError};
pub use report::{check, CheckOptions, Report, Status};
