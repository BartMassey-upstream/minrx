//! Queue-based Set
//!
//! Efficiently tracks which elements are present in a sparse set using a bit vector hierarchy.

/// Queue-based Set using a bit vector hierarchy
///
/// Efficiently tracks which elements are present in a sparse set.
pub struct QSet {
    bits: Vec<Vec<u64>>,
    depth: usize,
}

impl QSet {
    pub fn new(limit: usize) -> Self {
        let mut depth = 0;
        let mut sizes = Vec::new();
        let mut current = limit;

        loop {
            current = (current + 63) / 64; // Ceiling division
            sizes.push(current);
            depth += 1;
            if current <= 1 {
                break;
            }
        }

        let mut bits = Vec::with_capacity(depth);
        bits.push(vec![0u64; 1]);
        for i in 1..depth {
            bits.push(vec![0u64; sizes[depth - 1 - i]]);
        }

        Self { bits, depth }
    }

    #[inline]
    fn bit(k: usize) -> u64 {
        1u64 << (k & 0x3F)
    }

    pub fn is_empty(&self) -> bool {
        self.bits[0][0] == 0
    }

    #[cfg(test)]
    pub fn contains(&self, k: usize) -> bool {
        let mut i = 0;
        let mut s = 6 * self.depth;
        let mut j = 0;

        while i < self.depth {
            s -= 6;
            j = (j << 6) | ((k >> s) & 0x3f);
            if (self.bits[i][j >> 6] & (1 << (j & 0x3f))) == 0 {
                return false;
            }
            i += 1;
        }
        true
    }


    pub fn insert(&mut self, k: usize) -> bool {
        let mut newly_inserted = false;
        let mut i = 0;
        let mut s = 6 * self.depth;
        let mut j = 0;

        while i < self.depth {
            let x = self.bits[i][j];
            s -= 6;
            let next_j = k >> s;
            let w = Self::bit(next_j);

            if (x & w) == 0 {
                if i + 1 < self.depth {
                    self.bits[i + 1][next_j] = 0;
                } else {
                    newly_inserted = true;
                }
            }
            self.bits[i][j] |= w;
            j = next_j;
            i += 1;
        }
        newly_inserted
    }

    pub fn remove(&mut self) -> usize {
        let mut k = 0;
        let mut i = 0;

        // Find the first set bit
        while i < self.depth {
            let bit_pos = self.bits[i][k].trailing_zeros() as usize;
            k = (k << 6) | bit_pos;
            i += 1;
        }

        let result = k;

        // Remove the bit
        loop {
            i -= 1;
            let w = Self::bit(k);
            k >>= 6;
            self.bits[i][k] &= !w;
            if self.bits[i][k] != 0 || i == 0 {
                break;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qset_basic() {
        let mut qset = QSet::new(100);
        assert!(qset.is_empty());

        assert!(qset.insert(42));
        assert!(!qset.is_empty());
        assert!(qset.contains(42));
        assert!(!qset.contains(43));

        let removed = qset.remove();
        assert_eq!(removed, 42);
        assert!(qset.is_empty());
    }
}
