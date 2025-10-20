//! Copy-on-Write Vector
//!
//! Provides efficient cloning with deferred copying until modification.

use std::rc::Rc;

/// Copy-on-Write Vector
///
/// Simple wrapper around `Rc<Vec<T>>` that clones on write.
/// We use Rc instead of std::borrow::Cow because:
/// 1. Multiple owners can share the same data (Rc reference counting)
/// 2. No lifetime parameters needed (important for state machine)
/// 3. Clone-on-write only when refcount > 1 (automatic sharing detection)
#[derive(Clone)]
pub struct COWVec<T: Clone> {
    storage: Rc<Vec<T>>,
}

impl<T: Clone> COWVec<T> {
    pub fn new(size: usize, init: T) -> Self {
        Self {
            storage: Rc::new(vec![init; size]),
        }
    }

    pub fn get(&self, idx: usize) -> T {
        self.storage[idx].clone()
    }

    pub fn put(&mut self, idx: usize, val: T) {
        // Clone-on-write: Rc::make_mut clones only if refcount > 1
        Rc::make_mut(&mut self.storage)[idx] = val;
    }


}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cowvec_basic() {
        let mut v1 = COWVec::new(10, 0usize);
        v1.put(5, 42);
        assert_eq!(v1.get(5), 42);
        assert_eq!(v1.get(3), 0);
    }

    #[test]
    fn test_cowvec_clone() {
        let mut v1 = COWVec::new(10, 0usize);
        v1.put(5, 42);

        let v2 = v1.clone();
        assert_eq!(v2.get(5), 42);

        v1.put(5, 99);
        assert_eq!(v1.get(5), 99);
        assert_eq!(v2.get(5), 42); // v2 should be unchanged
    }
}
