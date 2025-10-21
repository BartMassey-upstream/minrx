//! Queue-based Vector
//!
//! Combines QSet with data storage for sparse vectors.

use super::qset::QSet;

/// Queue-based Vector
///
/// Combines QSet with data storage for sparse vectors.
pub struct QVec<T> {
    qset: QSet,
    storage: Vec<Option<T>>,
}

impl<T> QVec<T> {
    pub fn new(limit: usize) -> Self {
        Self {
            qset: QSet::new(limit),
            storage: (0..limit).map(|_| None).collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.qset.is_empty()
    }

    /// Insert an element, returning (was_new, mutable_reference)
    /// If was_new is true, the caller must initialize the reference
    pub fn insert(&mut self, k: usize) -> (bool, &mut Option<T>) {
        let newly = self.qset.insert(k);
        (newly, &mut self.storage[k])
    }

    pub fn lookup(&self, k: usize) -> Option<&T> {
        self.storage[k].as_ref()
    }

    #[cfg(test)]
    pub fn contains(&self, k: usize) -> bool {
        self.qset.contains(k)
    }

    pub fn remove(&mut self) -> (usize, T) {
        let k = self.qset.remove();
        let data = self.storage[k].take().expect("QVec invariant violated");
        (k, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qvec_basic() {
        let mut qvec: QVec<String> = QVec::new(100);
        assert!(qvec.is_empty());

        let (newly, slot) = qvec.insert(42);
        assert!(newly);
        *slot = Some("hello".to_string());

        assert!(qvec.contains(42));
        assert_eq!(qvec.lookup(42), Some(&"hello".to_string()));

        let (k, val) = qvec.remove();
        assert_eq!(k, 42);
        assert_eq!(val, "hello".to_string());
        assert!(qvec.is_empty());
    }
}
