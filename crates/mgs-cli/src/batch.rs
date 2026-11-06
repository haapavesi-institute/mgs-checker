//! What the commands that check many quivers share: running them on several
//! threads, and writing the rows out as Markdown, CSV or JSON.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use clap::ValueEnum;
use serde::Serialize;

#[derive(Clone, Copy, ValueEnum)]
pub enum Format {
    Markdown,
    Csv,
    Json,
}

/// f(i, item) for every item, on `jobs` threads, in the original order.
pub fn parallel<T: Sync, R: Send>(items: &[T], jobs: usize, f: impl Fn(usize, &T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Option<R>>> = Mutex::new((0..items.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(i) else { break };
                let r = f(i, item);
                out.lock().expect("no panics while holding the lock")[i] = Some(r);
            });
        }
    });
    out.into_inner().expect("threads finished").into_iter().map(|r| r.expect("every item ran")).collect()
}

/// The rows as text: `cells` gives each row's cells, in the order of `header`.
pub fn render<R: Serialize>(rows: &[R], header: &[&str], cells: impl Fn(&R) -> Vec<String>, format: Format) -> Result<String, String> {
    Ok(match format {
        Format::Markdown => {
            let mut s = format!("| {} |\n|{}\n", header.join(" | "), "---|".repeat(header.len()));
            for r in rows {
                s += &format!("| {} |\n", cells(r).join(" | "));
            }
            s
        }
        Format::Csv => {
            let quote = |c: &str| if c.contains([',', '"', '\n']) { format!("\"{}\"", c.replace('"', "\"\"")) } else { c.to_string() };
            let mut s = header.iter().map(|h| quote(h)).collect::<Vec<_>>().join(",") + "\n";
            for r in rows {
                s += &(cells(r).iter().map(|c| quote(c)).collect::<Vec<_>>().join(",") + "\n");
            }
            s
        }
        Format::Json => serde_json::to_string_pretty(rows).map_err(|e| e.to_string())? + "\n",
    })
}

/// Writes to the file, or to standard output.
pub fn write(out: Option<&Path>, text: &str) -> Result<(), String> {
    match out {
        Some(path) => std::fs::write(path, text).map_err(|e| format!("writing {}: {e}", path.display())),
        None => {
            print!("{text}");
            Ok(())
        }
    }
}
