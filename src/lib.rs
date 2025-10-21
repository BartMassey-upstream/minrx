#![doc(html_root_url = "https://docs.rs/minrx/0.1.0")]

//! MinRX: A minimal matcher for POSIX Extended Regular Expressions
//!
//! MinRX provides a fast, memory-safe implementation of POSIX Extended Regular
//! Expression (ERE) matching. The library uses a non-backtracking NFA-based
//! algorithm that guarantees linear time complexity.
//!
//! # Quick Start
//!
//! ```
//! use minrx::{Regex, CompileFlags, ExecFlags};
//!
//! let pattern = "hello|world";
//! let regex = Regex::new(pattern, CompileFlags::EXTENDED).unwrap();
//!
//! let text = "hello there";
//! let matches = regex.exec(text, ExecFlags::empty()).unwrap();
//!
//! let m = matches[0].as_ref().unwrap();
//! assert_eq!(m.start, 0);
//! assert_eq!(m.end, 5);
//! ```
//!
//! # Pattern Syntax
//!
//! MinRX supports POSIX Extended Regular Expression syntax:
//!
//! - **Literals**: `abc` matches "abc"
//! - **Dot**: `.` matches any character (except newline with `NEWLINE` flag)
//! - **Brackets**: `[abc]` matches a, b, or c; `[^abc]` matches anything except a, b, c
//! - **Ranges**: `[a-z]` matches lowercase letters; `[0-9]` matches digits
//! - **Character classes**: `[:alnum:]`, `[:alpha:]`, `[:digit:]`, `[:lower:]`, `[:upper:]`, `[:space:]`
//! - **Alternation**: `a|b` matches a or b
//! - **Grouping**: `(abc)` creates a capture group
//! - **Repetition**:
//!   - `*` matches 0 or more times (greedy)
//!   - `+` matches 1 or more times (greedy)
//!   - `?` matches 0 or 1 time (greedy)
//!   - `{n}` matches exactly n times
//!   - `{n,m}` matches n to m times
//!   - `{n,}` matches n or more times
//! - **Anchors**:
//!   - `^` matches beginning of line
//!   - `$` matches end of line
//! - **Escapes**: `\` escapes special characters
//!
//! # Extensions
//!
//! MinRX also supports some common extensions:
//!
//! - **BSD**: `\<` and `\>` for word boundaries
//! - **GNU**: `\b` (word boundary), `\B` (not word boundary), `\w` (word char),
//!   `\W` (non-word), `\s` (whitespace), `\S` (non-whitespace)
//!
//! # Performance
//!
//! - **Time complexity**: O(M × N) where M is pattern length, N is text length
//! - **Space complexity**: O(M × P) where P is max parentheses nesting depth
//! - **Non-backtracking**: Single forward scan through input, no catastrophic backtracking
//! - **POSIX semantics**: Finds leftmost-longest match
//!
//! # Examples
//!
//! ## Basic Matching
//!
//! ```
//! use minrx::{Regex, CompileFlags, ExecFlags};
//!
//! let regex = Regex::new("a+b*", CompileFlags::EXTENDED).unwrap();
//! let matches = regex.exec("aaabbb", ExecFlags::empty()).unwrap();
//! let m = matches[0].as_ref().unwrap();
//! assert_eq!(m.start, 0);
//! assert_eq!(m.end, 6);
//! ```
//!
//! ## Capture Groups
//!
//! ```
//! use minrx::{Regex, CompileFlags, ExecFlags};
//!
//! let regex = Regex::new("([a-z]+)@([a-z]+)", CompileFlags::EXTENDED).unwrap();
//! let matches = regex.exec("user@example", ExecFlags::empty()).unwrap();
//!
//! // matches[0] is the full match
//! assert_eq!(matches[0].as_ref().unwrap().start, 0);
//! assert_eq!(matches[0].as_ref().unwrap().end, 12);
//!
//! // matches[1] is first capture group
//! assert_eq!(matches[1].as_ref().unwrap().start, 0);
//! assert_eq!(matches[1].as_ref().unwrap().end, 4);
//!
//! // matches[2] is second capture group
//! assert_eq!(matches[2].as_ref().unwrap().start, 5);
//! assert_eq!(matches[2].as_ref().unwrap().end, 12);
//! ```
//!
//! ## Case-Insensitive Matching
//!
//! ```
//! use minrx::{Regex, CompileFlags, ExecFlags};
//!
//! let regex = Regex::new("hello", CompileFlags::EXTENDED | CompileFlags::ICASE).unwrap();
//! assert!(regex.exec("HELLO", ExecFlags::empty()).is_ok());
//! assert!(regex.exec("Hello", ExecFlags::empty()).is_ok());
//! ```
//!
//! # Copyright
//!
//! MinRX: a minimal matcher for POSIX Extended Regular Expressions.
//! Copyright (C) 2023, 2024, 2025 Michael J. Haertel.
//! Rust port by Claude.
//!
//! Redistribution and use in source and binary forms, with or without
//! modification, are permitted provided that the following conditions
//! are met:
//!
//! 1. Redistributions of source code must retain the above copyright
//!    notice, this list of conditions and the following disclaimer.
//!
//! 2. Redistributions in binary form must reproduce the above copyright
//!    notice, this list of conditions and the following disclaimer in the
//!    documentation and/or other materials provided with the distribution.
//!
//! THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS "AS IS" AND
//! ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
//! IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
//! ARE DISCLAIMED. IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
//! FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
//! DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
//! OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
//! HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
//! LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
//! OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
//! SUCH DAMAGE.

use std::collections::HashMap;

mod compile;
mod cset;
mod data_structures;
mod execute;
pub mod ffi;
mod node;

pub(crate) use cset::CSet;
pub(crate) use node::Node;

const RE_DUP_MAX: usize = 32767;

/// Error types for regex compilation and execution
///
/// These error codes match POSIX regex error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum RegexError {
    /// Operation completed successfully (not an error)
    Success = 0,
    /// Invalid regular expression pattern
    BadPat = 1,
    /// Invalid contents of `{}`
    BadBr = 2,
    /// `?`, `*`, `+`, or `{interval}` not preceded by valid subpattern
    BadRpt = 3,
    /// Unbalanced `{`
    EBrace = 4,
    /// Unbalanced `[`
    EBrack = 5,
    /// Invalid collating element
    ECollate = 6,
    /// Invalid character class name
    ECType = 7,
    /// Trailing backslash `\`
    EEscape = 8,
    /// Unbalanced `(`
    EParen = 9,
    /// Invalid range endpoint in bracket expression
    ERange = 10,
    /// Out of memory
    ESpace = 11,
    /// Invalid backreference number
    ESubreg = 12,
    /// No match found
    NoMatch = 13,
    /// Unknown error code
    Unknown = 14,
}

impl RegexError {
    /// Returns a human-readable error message for this error code
    pub const fn message(&self) -> &'static str {
        match self {
            RegexError::Success => "success",
            RegexError::BadPat => "bad pattern",
            RegexError::BadBr => "invalid contents of {}",
            RegexError::BadRpt => "? * + or {interval} not preceded by valid subpattern",
            RegexError::EBrace => "unbalanced {",
            RegexError::EBrack => "unbalanced [",
            RegexError::ECollate => "invalid collating element",
            RegexError::ECType => "invalid character class name",
            RegexError::EEscape => "invalid trailing backslash",
            RegexError::EParen => "unbalanced (",
            RegexError::ERange => "invalid range endpoint",
            RegexError::ESpace => "memory allocation failed",
            RegexError::ESubreg => "invalid \\digit",
            RegexError::NoMatch => "match not found",
            RegexError::Unknown => "unknown error code",
        }
    }
}

impl std::fmt::Display for RegexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for RegexError {}

bitflags::bitflags! {
    /// Flags for regex compilation
    ///
    /// These flags control how the pattern is interpreted during compilation.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct CompileFlags: u32 {
        /// Use POSIX Extended Regular Expression syntax
        const EXTENDED = 1;
        /// Case-insensitive matching
        const ICASE = 2;
        /// Minimal (non-greedy) matching (not yet implemented)
        const MINIMAL = 4;
        /// Treat newline specially: `.` doesn't match `\n`, `^` matches after `\n`, `$` matches before `\n`
        const NEWLINE = 8;
        /// Don't track subexpressions (not yet implemented)
        const NOSUB = 16;
        /// Enable brace compatibility mode
        const BRACE_COMPAT = 32;
        /// Allow backslash escapes in bracket expressions
        const BRACK_ESCAPE = 64;
        /// Enable BSD extensions (`\<`, `\>`)
        const EXTENSIONS_BSD = 128;
        /// Enable GNU extensions (`\b`, `\B`, `\w`, `\W`, `\s`, `\S`, `` \` ``, `\'`)
        const EXTENSIONS_GNU = 256;
        /// Internal: native 1-byte encoding
        const NATIVE1B = 512;
        /// Internal: disable minimal matching
        const MINDISABLE = 1024;
    }
}

bitflags::bitflags! {
    /// Flags for regex execution
    ///
    /// These flags control how the match is performed.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct ExecFlags: u32 {
        /// First character is not at beginning of line (don't match `^`)
        const NOTBOL = 1;
        /// Last character is not at end of line (don't match `$`)
        const NOTEOL = 2;
        /// Only capture first submatch for each group
        const FIRSTSUB = 4;
        /// Don't reset later submatches when entering new group
        const NOSUBRESET = 8;
        /// Resume matching from previous position (not yet implemented)
        const RESUME = 16;
        /// Internal: disable first-byte optimization
        const NOFIRSTBYTES = 32;
    }
}

/// A match result representing a byte range in the input text
///
/// This is a standard Rust [`Range<usize>`](std::ops::Range) where `start` is the
/// beginning offset and `end` is one past the last matched byte.
///
/// Capture groups that didn't participate in a match are represented as `None`
/// in the results vector returned by [`Regex::exec`].
pub type RegMatch = std::ops::Range<usize>;

/// A compiled regular expression
///
/// # Examples
///
/// ```
/// use minrx::{Regex, CompileFlags, ExecFlags};
///
/// let regex = Regex::new("hello", CompileFlags::EXTENDED).unwrap();
/// let matches = regex.exec("hello world", ExecFlags::empty()).unwrap();
/// let m = matches[0].as_ref().unwrap();
/// assert_eq!(m.start, 0);
/// assert_eq!(m.end, 5);
/// ```
pub struct Regex {
    pub(crate) csets: Vec<CSet>,
    pub(crate) nodes: Vec<Node>,
    pub(crate) nmin: usize,
    pub(crate) nstk: usize,
    pub(crate) nsub: usize,
}

impl Regex {
    /// Compiles a regular expression pattern
    ///
    /// # Arguments
    ///
    /// * `pattern` - The regular expression pattern string
    /// * `flags` - Compilation flags controlling pattern interpretation
    ///
    /// # Returns
    ///
    /// Returns `Ok(Regex)` on success, or `Err(RegexError)` if the pattern is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use minrx::{Regex, CompileFlags};
    ///
    /// let regex = Regex::new("a+b*", CompileFlags::EXTENDED).unwrap();
    /// ```
    pub fn new(pattern: &str, flags: CompileFlags) -> Result<Self, RegexError> {
        compile::compile(pattern, flags)
    }

    /// Executes the regex against input text
    ///
    /// # Arguments
    ///
    /// * `text` - The text to search
    /// * `flags` - Execution flags controlling match behavior
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<Option<RegMatch>>)` with match positions on success.
    /// The first element (index 0) is always `Some` containing the overall match.
    /// Subsequent elements are captured subexpressions, which may be `None` if
    /// that group didn't participate in the match.
    ///
    /// Returns `Err(RegexError::NoMatch)` if no match is found.
    ///
    /// # Examples
    ///
    /// ```
    /// use minrx::{Regex, CompileFlags, ExecFlags};
    ///
    /// let regex = Regex::new("(\\w+)", CompileFlags::EXTENDED | CompileFlags::EXTENSIONS_GNU).unwrap();
    /// let matches = regex.exec("hello world", ExecFlags::empty()).unwrap();
    /// let m = matches[0].as_ref().unwrap();
    /// assert_eq!(m.start, 0);
    /// assert_eq!(m.end, 5);
    /// ```
    pub fn exec(&self, text: &str, flags: ExecFlags) -> Result<Vec<Option<RegMatch>>, RegexError> {
        execute::execute(self, text, flags)
    }

    /// Returns the number of capturing groups in the pattern
    ///
    /// This count does not include the overall match (group 0).
    pub const fn nsub(&self) -> usize {
        if self.nsub > 0 {
            self.nsub - 1
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_match() {
        let re = Regex::new("abc", CompileFlags::EXTENDED).unwrap();
        let result = re.exec("abc", ExecFlags::empty());
        assert!(result.is_ok());
    }
}
