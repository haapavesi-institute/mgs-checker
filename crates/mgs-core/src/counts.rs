//! Counts of maximal green sequences (or continuations) by length, exact.
//!
//! Stored from the first non-zero length on, as fixed-width little-endian
//! u64 limbs shared by the whole vector. The width starts at one limb and
//! grows when a sum carries out of it: E7 has about 3.7·10²³ MGS and Ẽ7 more
//! than 2¹²⁸, while most states need a single limb.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counts {
    start: u32,
    width: usize,
    data: Vec<u64>,
}

impl Default for Counts {
    fn default() -> Self {
        Self { start: 0, width: 1, data: Vec::new() }
    }
}

/// An exact non-negative integer as little-endian u64 limbs, with decimal output.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Big(pub Vec<u64>);

impl Big {
    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&x| x == 0)
    }

    fn add(&mut self, v: &[u64]) {
        if self.0.len() < v.len() {
            self.0.resize(v.len(), 0);
        }
        let mut carry = false;
        for (i, limb) in self.0.iter_mut().enumerate() {
            let (s1, c1) = limb.overflowing_add(v.get(i).copied().unwrap_or(0));
            let (s2, c2) = s1.overflowing_add(u64::from(carry));
            *limb = s2;
            carry = c1 || c2;
        }
        if carry {
            self.0.push(1);
        }
    }

    /// Converts to u128 if it fits.
    pub fn to_u128(&self) -> Option<u128> {
        let top = self.0.iter().rposition(|&x| x != 0).map_or(0, |i| i + 1);
        (top <= 2).then(|| self.0.iter().take(2).enumerate().map(|(i, &x)| u128::from(x) << (64 * i)).sum())
    }
}

impl std::fmt::Display for Big {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const TEN19: u64 = 10_000_000_000_000_000_000;
        let mut limbs = self.0.clone();
        let mut chunks = Vec::new();
        while limbs.iter().any(|&x| x != 0) {
            let mut rem = 0u128;
            for limb in limbs.iter_mut().rev() {
                let cur = (rem << 64) | u128::from(*limb);
                *limb = (cur / u128::from(TEN19)) as u64;
                rem = cur % u128::from(TEN19);
            }
            chunks.push(rem as u64);
        }
        match chunks.split_last() {
            None => write!(f, "0"),
            Some((top, rest)) => {
                write!(f, "{top}")?;
                rest.iter().rev().try_for_each(|c| write!(f, "{c:019}"))
            }
        }
    }
}

impl Counts {
    /// One continuation, of length 0: the state is already all red.
    pub fn done() -> Self {
        Self { start: 0, width: 1, data: vec![1] }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn len(&self) -> usize {
        self.data.len() / self.width
    }

    fn value(&self, i: usize) -> &[u64] {
        &self.data[i * self.width..(i + 1) * self.width]
    }

    fn nonzero(&self, i: usize) -> bool {
        self.value(i).iter().any(|&x| x != 0)
    }

    /// Whether some continuation has length ≤ limit.
    pub fn any_within(&self, limit: u32) -> bool {
        !self.data.is_empty() && self.start <= limit
    }

    /// The first and last lengths with a non-zero count, at most `limit`.
    pub fn range_within(&self, limit: u32) -> Option<(u32, u32)> {
        (0..self.len()).rev().find(|&i| self.start + i as u32 <= limit && self.nonzero(i)).map(|i| (self.start, self.start + i as u32))
    }

    /// Adds `other`, shifted by one step and truncated at `limit` before the shift.
    pub fn add_step(&mut self, other: &Counts, limit: u32) {
        for i in 0..other.len() {
            let l = other.start + i as u32;
            if l > limit {
                break;
            }
            if other.nonzero(i) {
                self.add_at(l + 1, other.value(i));
            }
        }
    }

    fn widen(&mut self, width: usize) {
        let mut data = vec![0; self.len() * width];
        for i in 0..self.len() {
            data[i * width..i * width + self.width].copy_from_slice(self.value(i));
        }
        (self.width, self.data) = (width, data);
    }

    fn add_at(&mut self, length: u32, v: &[u64]) {
        let need = v.iter().rposition(|&x| x != 0).map_or(1, |i| i + 1);
        if need > self.width {
            self.widen(need);
        }
        if self.data.is_empty() {
            self.start = length;
        } else if length < self.start {
            let shift = (self.start - length) as usize * self.width;
            self.data.splice(0..0, std::iter::repeat_n(0, shift));
            self.start = length;
        }
        let i = (length - self.start) as usize;
        if self.len() <= i {
            self.data.resize((i + 1) * self.width, 0);
        }
        let mut sum = Big(self.value(i).to_vec());
        sum.add(&v[..need]);
        if sum.0.len() > self.width {
            self.widen(sum.0.len());
        }
        let w = self.width;
        sum.0.resize(w, 0);
        self.data[i * w..(i + 1) * w].copy_from_slice(&sum.0);
    }

    /// (length, count) for every non-zero count with length ≤ limit.
    pub fn nonzero_within(&self, limit: u32) -> Vec<(u32, Big)> {
        (0..self.len())
            .filter(|&i| self.nonzero(i) && self.start + i as u32 <= limit)
            .map(|i| (self.start + i as u32, Big(self.value(i).to_vec())))
            .collect()
    }

    /// Total over lengths ≤ limit.
    pub fn total_within(&self, limit: u32) -> Big {
        let mut t = Big::default();
        for (_, v) in self.nonzero_within(limit) {
            t.add(&v.0);
        }
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small(c: &Counts) -> Vec<(u32, u128)> {
        c.nonzero_within(u32::MAX).into_iter().map(|(l, v)| (l, v.to_u128().unwrap())).collect()
    }

    #[test]
    fn shifting_and_truncating() {
        let mut a = Counts::default();
        assert!(!a.any_within(100));
        a.add_step(&Counts::done(), 5);
        let b = a.clone(); // one continuation of length 1
        a.add_step(&b, 5); // now also length 2
        a.add_step(&b, 0); // truncated: b's length 1 > limit 0, so nothing added
        assert_eq!(small(&a), vec![(1, 1), (2, 1)]);
        let mut c = Counts::default();
        c.add_step(&a, 1); // only a's length-1 entry, shifted to 2
        assert_eq!(small(&c), vec![(2, 1)]);
        c.add_step(&Counts::done(), 9); // length 1 < start 2: extends downward
        assert_eq!(small(&c), vec![(1, 1), (2, 1)]);
        assert_eq!(c.range_within(1), Some((1, 1)));
        assert_eq!(c.total_within(10).to_u128(), Some(2));
    }

    #[test]
    fn grows_past_64_and_128_bits() {
        let mut a = Counts { start: 1, width: 1, data: vec![u64::MAX] };
        a.add_step(&Counts::done(), 3);
        assert_eq!(a.total_within(9).to_string(), "18446744073709551616"); // 2^64
        let mut big = Counts { start: 0, width: 2, data: vec![u64::MAX, u64::MAX] };
        big.add_step(&big.clone(), 9); // 2^128 − 1 at length 1
        let mut twice = big.clone();
        twice.add_step(&Counts { start: 0, width: 2, data: vec![u64::MAX, u64::MAX] }, 9);
        let at1 = twice.nonzero_within(9).into_iter().find(|(l, _)| *l == 1).unwrap().1;
        assert_eq!(at1.to_string(), "680564733841876926926749214863536422910"); // 2·(2^128 − 1)
        assert_eq!(twice.total_within(9).to_string(), "1020847100762815390390123822295304634365"); // 3·(2^128 − 1)
        assert_eq!(Big(vec![0, 0, 1]).to_string(), "340282366920938463463374607431768211456");
        // 2^128
    }
}
