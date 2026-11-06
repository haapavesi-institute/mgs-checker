//! Maximal green sequences of quivers given by their exchange matrices.
//!
//! - [`report::check`] counts every MGS by length. It walks the oriented
//!   exchange graph one length deeper at a time. It stops when every green
//!   sequence has ended, or when BDP's No Gap Conjecture, relaxed by a chosen
//!   `max_gap`, says no longer MGS can exist. It says which of the two
//!   happened, and lists the banned prefixes: those BHIT 3.3.2 rules out,
//!   and the empirically infinite ones.
//! - [`verify::verify`] checks one given sequence.
//! - [`explore`] runs the fixed explorations around tame quivers.
//!
//! Convention: Brüstle–Dupont–Pérotin framing (green means c-vector ≥ 0), and
//! b_ij = #(i → j) − #(j → i). Vertices are 1-based in every input and output.

pub mod bhit;
pub mod canonical;
pub mod classify;
pub mod counts;
pub mod csv;
pub mod engine;
pub mod explore;
pub mod families;
pub mod framed;
pub mod known;
pub mod matrix;
pub mod modify;
pub mod report;
pub mod verify;
pub mod walk;

pub use engine::SearchOptions;
pub use matrix::{Exchange, MatrixError};
pub use report::{check, CheckOptions, Report, Status};
