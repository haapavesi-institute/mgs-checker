//! Exchange matrices as CSV: one row of the matrix per line.
//!
//! Entries may be separated by commas, semicolons, tabs or spaces. Blank
//! lines and lines starting with `#` are skipped, and so is a header line
//! whose entries are all non-numeric (for example `v1,v2,v3`). Entry (i, j)
//! is b_ij = #(i → j) − #(j → i), so the CSV of 1 ⇉ 2 → 3 is
//!
//! ```text
//! 0,2,0
//! -2,0,1
//! 0,-1,0
//! ```

use crate::matrix::{Exchange, MatrixError};

fn tokens(line: &str) -> Vec<&str> {
    let parts: Vec<&str> = if line.contains([',', ';']) {
        line.split([',', ';']).map(str::trim).collect()
    } else if line.contains('\t') {
        line.split('\t').map(str::trim).collect()
    } else {
        line.split_whitespace().collect()
    };
    let mut parts = parts;
    while parts.last() == Some(&"") {
        parts.pop(); // a trailing separator
    }
    parts
}

/// Parses a CSV exchange matrix and checks it (square, skew-symmetric).
pub fn parse_csv(text: &str) -> Result<Exchange, MatrixError> {
    let mut rows = Vec::new();
    let mut header_allowed = true;
    for (index, raw) in text.trim_start_matches('\u{feff}').lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts = tokens(line);
        if header_allowed && !parts.is_empty() && parts.iter().all(|t| t.parse::<i64>().is_err()) {
            header_allowed = false;
            continue;
        }
        header_allowed = false;
        let mut row = Vec::with_capacity(parts.len());
        for (column, token) in parts.iter().enumerate() {
            let value =
                token.parse::<i64>().map_err(|_| MatrixError::Parse { line: index + 1, column: column + 1, token: (*token).into() })?;
            row.push(value);
        }
        rows.push(row);
    }
    Exchange::from_rows(rows)
}

/// The matrix as CSV, one row per line.
pub fn to_csv(q: &Exchange) -> String {
    q.rows().iter().map(|r| r.iter().map(i64::to_string).collect::<Vec<_>>().join(",")).collect::<Vec<_>>().join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_usual_shapes() {
        let q = parse_csv("0,2,0\n-2,0,1\n0,-1,0\n").unwrap();
        assert_eq!(q.arrow_text(), "1->2*2, 2->3");
        for text in [
            "# the quiver 1 => 2 -> 3\nv1,v2,v3\n0,2,0\n-2,0,1\n0,-1,0",
            "0 2 0\n-2 0 1\n0 -1 0",
            "0;2;0;\n-2;0;1;\n0;-1;0;",
            "\u{feff}0,\t2, 0\r\n-2,0,1\r\n\r\n0,-1,0",
        ] {
            assert_eq!(parse_csv(text).unwrap(), q, "{text:?}");
        }
        assert_eq!(parse_csv(&to_csv(&q)).unwrap(), q);
    }

    #[test]
    fn reports_where_it_went_wrong() {
        assert_eq!(parse_csv("0,1\n-1,x").unwrap_err(), MatrixError::Parse { line: 2, column: 2, token: "x".into() });
        assert!(matches!(parse_csv("0,1\n1,0"), Err(MatrixError::NotSkew { .. })));
        assert_eq!(parse_csv("# nothing\n"), Err(MatrixError::Empty));
    }
}
