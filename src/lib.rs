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
//! assert_eq!(matches[0].start, 0);
//! assert_eq!(matches[0].end, 5);
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
//! assert_eq!(matches[0].start, 0);
//! assert_eq!(matches[0].end, 6);
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
//! assert_eq!(matches[0].start, 0);
//! assert_eq!(matches[0].end, 12);
//!
//! // matches[1] is first capture group
//! assert_eq!(matches[1].start, 0);
//! assert_eq!(matches[1].end, 4);
//!
//! // matches[2] is second capture group
//! assert_eq!(matches[2].start, 5);
//! assert_eq!(matches[2].end, 12);
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

// Allow dead code for internal types and methods that are part of the implementation
// but may not be used in all code paths (e.g., some data structure methods, node variants)
#![allow(dead_code)]

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
    Success = 0,
    BadPat = 1,
    BadBr = 2,
    BadRpt = 3,
    EBrace = 4,
    EBrack = 5,
    ECollate = 6,
    ECType = 7,
    EEscape = 8,
    EParen = 9,
    ERange = 10,
    ESpace = 11,
    ESubreg = 12,
    NoMatch = 13,
    Unknown = 14,
}

impl RegexError {
    pub fn message(&self) -> &'static str {
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

bitflags::bitflags! {
    /// Flags for regex compilation
    ///
    /// These flags control how the pattern is interpreted during compilation.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct CompileFlags: u32 {
        const EXTENDED = 1;
        const ICASE = 2;
        const MINIMAL = 4;
        const NEWLINE = 8;
        const NOSUB = 16;
        const BRACE_COMPAT = 32;
        const BRACK_ESCAPE = 64;
        const EXTENSIONS_BSD = 128;
        const EXTENSIONS_GNU = 256;
        const NATIVE1B = 512;
        const MINDISABLE = 1024;
    }
}

bitflags::bitflags! {
    /// Flags for regex execution
    ///
    /// These flags control how the match is performed.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct ExecFlags: u32 {
        const NOTBOL = 1;
        const NOTEOL = 2;
        const FIRSTSUB = 4;
        const NOSUBRESET = 8;
        const RESUME = 16;
        const NOFIRSTBYTES = 32;
    }
}

/// A match result containing start and end positions
///
/// The positions are byte offsets into the input text.
/// A value of -1 indicates an invalid/unmatched position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegMatch {
    pub start: isize,
    pub end: isize,
}

impl RegMatch {
    pub fn new(start: isize, end: isize) -> Self {
        Self { start, end }
    }

    pub fn invalid() -> Self {
        Self { start: -1, end: -1 }
    }
}

/// A compiled regular expression
///
/// # Examples
///
/// ```
/// use minrx::{Regex, CompileFlags, ExecFlags};
///
/// let regex = Regex::new("hello", CompileFlags::EXTENDED).unwrap();
/// let matches = regex.exec("hello world", ExecFlags::empty()).unwrap();
/// assert_eq!(matches[0].start, 0);
/// assert_eq!(matches[0].end, 5);
/// ```
pub struct Regex {
    pub(crate) err: RegexError,
    pub(crate) csets: Vec<CSet>,
    pub(crate) nodes: Vec<Node>,
    pub(crate) firstcset: Option<CSet>,
    pub(crate) firstbytes: Option<Vec<bool>>,
    pub(crate) firstunique: Option<u8>,
    pub(crate) nmin: usize,
    pub(crate) nstk: usize,
    pub(crate) nsub: usize,
}

impl Regex {
    pub fn new(pattern: &str, flags: CompileFlags) -> Result<Self, RegexError> {
        compile::compile(pattern, flags)
    }

    pub fn exec(&self, text: &str, flags: ExecFlags) -> Result<Vec<RegMatch>, RegexError> {
        execute::execute(self, text, flags)
    }

    pub fn nsub(&self) -> usize {
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
