//! WebAssembly bindings for the browser front end. Every function takes and
//! returns JSON strings, and errors come back as a thrown string.

use mgs_core::acyclic::admit;
use mgs_core::conjecture7;
use mgs_core::csv::{parse_csv, to_csv};
use mgs_core::explore::{plan, run, ExploreSpec, Item};
use mgs_core::known::facts;
use mgs_core::verify::verify;
use mgs_core::{check, CheckOptions, Exchange};
use wasm_bindgen::prelude::*;

fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn json<T: serde::Serialize>(v: &T) -> Result<String, JsValue> {
    serde_json::to_string(v).map_err(err)
}

/// A quiver from text: arrows ("1->2*2, 2->3") if it contains "->", else a CSV exchange matrix.
fn quiver(input: &str) -> Result<Exchange, JsValue> {
    if input.contains("->") {
        Exchange::from_arrow_text(input, None).map_err(err)
    } else {
        parse_csv(input).map_err(err)
    }
}

/// The parsed quiver: vertex count, arrows, CSV, what kind it is, and why it
/// is refused (an oriented cycle) or null.
#[wasm_bindgen]
pub fn describe(input: &str) -> Result<String, JsValue> {
    let q = quiver(input)?;
    let f = facts(&q);
    let rejected = admit(&q).err().map(|e| e.to_string());
    Ok(serde_json::json!({ "vertices": q.n(), "arrows": q.arrow_text(), "list": q.arrows(), "csv": to_csv(&q), "facts": f, "rejected": rejected })
        .to_string())
}

/// The full report (see `mgs_core::report::Report`). `options` is a JSON `CheckOptions`; missing fields take defaults.
#[wasm_bindgen]
pub fn check_quiver(input: &str, options: &str) -> Result<String, JsValue> {
    let q = quiver(input)?;
    let opts: CheckOptions =
        if options.trim().is_empty() { CheckOptions::default() } else { serde_json::from_str(options).map_err(err)? };
    json(&check(&q, &opts).map_err(err)?)
}

/// `sequence` is 1-based, separated by commas or spaces.
#[wasm_bindgen]
pub fn verify_sequence(input: &str, sequence: &str) -> Result<String, JsValue> {
    let q = quiver(input)?;
    let seq: Vec<usize> = sequence
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| t.parse().map_err(|_| err(format!("{t:?} is not a vertex number"))))
        .collect::<Result<_, _>>()?;
    json(&verify(&q, &seq).map_err(err)?)
}

/// The quivers an exploration will check. `spec` is a JSON `ExploreSpec`.
#[wasm_bindgen]
pub fn explore_plan(spec: &str) -> Result<String, JsValue> {
    let spec: ExploreSpec = serde_json::from_str(spec).map_err(err)?;
    json(&plan(&spec).map_err(err)?)
}

/// The quivers a conjecture #7 survey will check. `spec` is a JSON `conjecture7::Spec`.
#[wasm_bindgen]
pub fn conjecture7_plan(spec: &str) -> Result<String, JsValue> {
    let spec: conjecture7::Spec = serde_json::from_str(spec).map_err(err)?;
    json(&conjecture7::plan(&spec).map_err(err)?)
}

/// Checks one quiver of a conjecture #7 survey (one `Item` from `conjecture7_plan`).
#[wasm_bindgen]
pub fn conjecture7_row(item: &str, options: &str) -> Result<String, JsValue> {
    let item: conjecture7::Item = serde_json::from_str(item).map_err(err)?;
    let opts: CheckOptions = serde_json::from_str(options).map_err(err)?;
    json(&conjecture7::run(&item, &opts).map_err(err)?)
}

/// Checks one planned quiver (one `Item` from `explore_plan`).
#[wasm_bindgen]
pub fn explore_row(item: &str, options: &str) -> Result<String, JsValue> {
    let item: Item = serde_json::from_str(item).map_err(err)?;
    let opts: CheckOptions = serde_json::from_str(options).map_err(err)?;
    json(&run(&item, &opts).map_err(err)?)
}
