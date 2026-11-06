//! Checking one given mutation sequence: is it green, is it maximal, and
//! which step (if any) BHIT 3.3.2 rules out.

use serde::Serialize;

use crate::bhit::banned_332;
use crate::framed::Framed;
use crate::matrix::Exchange;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    /// Every step green, and every vertex red at the end.
    Maximal,
    /// Every step green, but some vertex is still green.
    GreenNotMaximal,
    /// Some step mutates a red vertex.
    NotGreen,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Step {
    /// 1-based.
    pub vertex: usize,
    pub green: bool,
    /// The c-vector of the vertex just before the step.
    pub c_vector: Vec<i64>,
    /// (from, arrows): the step breaks BHIT 3.3.2.
    pub bhit: Option<(usize, i64)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Verification {
    pub verdict: Verdict,
    pub steps: Vec<Step>,
    /// Vertices still green at the end (1-based).
    pub green_at_end: Vec<usize>,
    /// For an MGS: σ with final c-vector of i equal to −e_σ(i), 1-based.
    pub final_permutation: Option<Vec<usize>>,
}

/// `seq` is 1-based.
pub fn verify(q: &Exchange, seq: &[usize]) -> Result<Verification, String> {
    let n = q.n();
    let mut f = Framed::new(q);
    let mut steps = Vec::with_capacity(seq.len());
    for (t, &v) in seq.iter().enumerate() {
        if v == 0 || v > n {
            return Err(format!("step {}: there is no vertex {v} (the quiver has {n})", t + 1));
        }
        let k = v - 1;
        steps.push(Step {
            vertex: v,
            green: f.is_green(k),
            c_vector: f.c_vector(k).to_vec(),
            bhit: banned_332(&f, k).map(|(i, m)| (i + 1, m)),
        });
        f = f.mutate(k).map_err(|_| format!("step {}: a matrix entry outgrew 64 bits", t + 1))?;
    }
    let green_at_end: Vec<usize> = f.greens().into_iter().map(|i| i + 1).collect();
    let verdict = if steps.iter().any(|s| !s.green) {
        Verdict::NotGreen
    } else if green_at_end.is_empty() {
        Verdict::Maximal
    } else {
        Verdict::GreenNotMaximal
    };
    let final_permutation =
        (verdict == Verdict::Maximal).then(|| f.final_permutation()).flatten().map(|p| p.into_iter().map(|i| i + 1).collect());
    Ok(Verification { verdict, steps, green_at_end, final_permutation })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdicts() {
        let q = Exchange::from_arrow_text("1->2*2, 2->3", None).unwrap();
        assert_eq!(verify(&q, &[1, 2, 3]).unwrap().verdict, Verdict::Maximal);
        assert_eq!(verify(&q, &[1, 2]).unwrap().verdict, Verdict::GreenNotMaximal);
        assert_eq!(verify(&q, &[1, 1]).unwrap().verdict, Verdict::NotGreen);
        let v = verify(&q, &[2]).unwrap();
        assert_eq!(v.steps[0].bhit, Some((1, 2)));
        assert!(verify(&q, &[4]).is_err());
    }
}
