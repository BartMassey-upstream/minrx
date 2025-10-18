//! Data structures for MinRX regex engine
//!
//! This module provides specialized data structures optimized for
//! regex matching, including copy-on-write vectors and queue-based sets.

mod cowvec;
mod qset;
mod qvec;

pub use cowvec::COWVec;
pub use qset::QSet;
pub use qvec::QVec;
