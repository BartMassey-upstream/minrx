//
// MinRX: a minimal matcher for POSIX Extended Regular Expressions.
// Copyright (C) 2023, 2024, 2025 Michael J. Haertel.
// Rust port by Claude.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
// 1. Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in the
//    documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS "AS IS" AND
// ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
// OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
// HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
// LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
// OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
// SUCH DAMAGE.
//

#![allow(dead_code)]

use std::collections::HashMap;

mod compile;
mod cset;
mod data_structures;
mod execute;
pub mod ffi;
mod node;

pub use cset::CSet;
pub use data_structures::{COWVec, QSet, QVec};
pub use node::{Node, NodeType};

const RE_DUP_MAX: usize = 32767;

// Error types matching POSIX regex
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

// Compilation flags
bitflags::bitflags! {
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

// Execution flags
bitflags::bitflags! {
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

// Match result
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

// Compiled regex
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
