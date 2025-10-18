//! Character set operations
//!
//! Handles character classes, bracket expressions, and character ranges.

use crate::{CompileFlags, RegexError};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub struct Range {
    pub min: char,
    pub max: char,
}

impl Range {
    pub fn new(a: char, b: char) -> Self {
        Self {
            min: if a <= b { a } else { b },
            max: if a >= b { a } else { b },
        }
    }

    pub fn contains(&self, c: char) -> bool {
        c >= self.min && c <= self.max
    }
}

impl PartialOrd for Range {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Range {}

impl Ord for Range {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.min > other.max {
            std::cmp::Ordering::Greater
        } else if self.max < other.min {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    }
}

/// Character Set
///
/// Represents a set of characters for matching.
#[derive(Clone, Debug, Default)]
pub struct CSet {
    ranges: BTreeSet<Range>,
}

impl CSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_char(&mut self, c: char) {
        self.set_range(c, c);
    }

    pub fn set_range(&mut self, clo: char, chi: char) {
        // Find overlapping ranges by iterating
        let mut overlapping: Vec<Range> = Vec::new();
        for range in self.ranges.iter() {
            // Check if ranges overlap (with adjacent merging)
            if !(chi < char::from_u32(range.min as u32 - 1).unwrap_or('\0')
                || clo > char::from_u32(range.max as u32 + 1).unwrap_or(char::MAX))
            {
                overlapping.push(range.clone());
            }
        }

        if overlapping.is_empty() {
            self.ranges.insert(Range::new(clo, chi));
            return;
        }

        // Merge all overlapping ranges
        let mut new_min = clo;
        let mut new_max = chi;

        for range in &overlapping {
            new_min = new_min.min(range.min);
            new_max = new_max.max(range.max);
            self.ranges.remove(range);
        }

        self.ranges.insert(Range::new(new_min, new_max));
    }

    pub fn test(&self, c: char) -> bool {
        for range in &self.ranges {
            if range.contains(c) {
                return true;
            }
            if c < range.min {
                return false;
            }
        }
        false
    }

    pub fn invert(&mut self) {
        let mut new_ranges = BTreeSet::new();
        let mut lo = '\0';

        for range in &self.ranges {
            if lo < range.min {
                let end = char::from_u32(range.min as u32 - 1).unwrap_or(lo);
                if lo <= end {
                    new_ranges.insert(Range::new(lo, end));
                }
            }
            lo = char::from_u32(range.max as u32 + 1).unwrap_or(char::MAX);
        }

        if lo < char::MAX {
            new_ranges.insert(Range::new(lo, char::MAX));
        }

        self.ranges = new_ranges;
    }

    pub fn union(&mut self, other: &CSet) {
        for range in &other.ranges {
            self.set_range(range.min, range.max);
        }
    }

    pub fn add_char_class(&mut self, name: &str, flags: CompileFlags) -> bool {
        match name {
            "alnum" => self.add_alnum(flags),
            "alpha" => self.add_alpha(flags),
            "digit" => self.add_digit(),
            "lower" => self.add_lower(flags),
            "upper" => self.add_upper(flags),
            "space" => self.add_space(),
            "blank" => self.add_blank(),
            "punct" => self.add_punct(),
            "graph" => self.add_graph(flags),
            "print" => self.add_print(flags),
            "cntrl" => self.add_cntrl(),
            "xdigit" => self.add_xdigit(),
            _ => return false,
        }
        true
    }

    fn add_alnum(&mut self, flags: CompileFlags) {
        self.set_range('a', 'z');
        self.set_range('A', 'Z');
        self.set_range('0', '9');
        if flags.contains(CompileFlags::ICASE) {
            // Already covered by both upper and lower
        }
    }

    fn add_alpha(&mut self, flags: CompileFlags) {
        self.set_range('a', 'z');
        self.set_range('A', 'Z');
        if flags.contains(CompileFlags::ICASE) {
            // Already covered
        }
    }

    fn add_digit(&mut self) {
        self.set_range('0', '9');
    }

    fn add_lower(&mut self, flags: CompileFlags) {
        self.set_range('a', 'z');
        if flags.contains(CompileFlags::ICASE) {
            self.set_range('A', 'Z');
        }
    }

    fn add_upper(&mut self, flags: CompileFlags) {
        self.set_range('A', 'Z');
        if flags.contains(CompileFlags::ICASE) {
            self.set_range('a', 'z');
        }
    }

    fn add_space(&mut self) {
        self.set_char(' ');
        self.set_char('\t');
        self.set_char('\n');
        self.set_char('\r');
        self.set_char('\x0B'); // vertical tab
        self.set_char('\x0C'); // form feed
    }

    fn add_blank(&mut self) {
        self.set_char(' ');
        self.set_char('\t');
    }

    fn add_punct(&mut self) {
        for c in '!'..='/' {
            self.set_char(c);
        }
        for c in ':'..='@' {
            self.set_char(c);
        }
        for c in '['..='`' {
            self.set_char(c);
        }
        for c in '{'..='~' {
            self.set_char(c);
        }
    }

    fn add_graph(&mut self, flags: CompileFlags) {
        self.set_range('!', '~');
        if flags.contains(CompileFlags::ICASE) {
            self.add_lower(flags);
            self.add_upper(flags);
        }
    }

    fn add_print(&mut self, flags: CompileFlags) {
        self.set_range(' ', '~');
        if flags.contains(CompileFlags::ICASE) {
            self.add_lower(flags);
            self.add_upper(flags);
        }
    }

    fn add_cntrl(&mut self) {
        for c in '\0'..=' ' {
            self.set_char(c);
        }
        self.set_char('\x7F');
    }

    fn add_xdigit(&mut self) {
        self.set_range('0', '9');
        self.set_range('a', 'f');
        self.set_range('A', 'F');
    }

    pub fn parse(
        &mut self,
        flags: CompileFlags,
        chars: &mut std::iter::Peekable<std::str::Chars>,
    ) -> Result<(), RegexError> {
        let mut c_opt = chars.next();
        let inv = c_opt == Some('^');
        if inv {
            c_opt = chars.next();
        }

        let mut first = true;
        while first || c_opt != Some(']') {
            first = false;

            let c = match c_opt {
                Some(ch) => ch,
                None => return Err(RegexError::EBrack),
            };

            let mut clo = c;
            let mut chi = c;

            // Handle backslash escapes in bracket expressions
            if clo == '\\' && flags.contains(CompileFlags::BRACK_ESCAPE) {
                clo = chars.next().ok_or(RegexError::EEscape)?;
                chi = clo;
            } else if clo == '[' {
                c_opt = chars.next();
                // Handle [:class:], [=equiv=], [.collating.]
                if c_opt == Some(':') {
                    // Character class [:name:]
                    let mut class_name = String::new();
                    loop {
                        c_opt = chars.next();
                        if c_opt == Some(':') {
                            c_opt = chars.next();
                            if c_opt != Some(']') {
                                return Err(RegexError::ECType);
                            }
                            if !self.add_char_class(&class_name, flags) {
                                return Err(RegexError::ECType);
                            }
                            // Don't read next char here - let line 324 do it
                            break; // Exit inner loop after parsing character class
                        } else if let Some(ch) = c_opt {
                            class_name.push(ch);
                        } else {
                            return Err(RegexError::ECType);
                        }
                    }
                } else if c_opt == Some('.') {
                    // Collating element [.x.] - treat as literal
                    c_opt = chars.next();
                    clo = c_opt.ok_or(RegexError::ECollate)?;
                    chi = clo;
                    c_opt = chars.next();
                    if c_opt != Some('.') {
                        return Err(RegexError::ECollate);
                    }
                    c_opt = chars.next();
                    if c_opt != Some(']') {
                        return Err(RegexError::ECollate);
                    }
                    // Don't read next char here - let line 324 do it
                } else if c_opt == Some('=') {
                    // Equivalence class [=x=] - treat as literal
                    c_opt = chars.next();
                    clo = c_opt.ok_or(RegexError::ECollate)?;
                    chi = clo;
                    c_opt = chars.next();
                    if c_opt != Some('=') {
                        return Err(RegexError::ECollate);
                    }
                    c_opt = chars.next();
                    if c_opt != Some(']') {
                        return Err(RegexError::ECollate);
                    }
                    // Don't read next char here - let line 324 do it
                }
            }

            // Read next character to check for range
            c_opt = chars.next();

            // Handle range
            if c_opt == Some('-') {
                let next = chars.clone().next();
                if next != Some(']') && next.is_some() {
                    // chars already points past '-', so just read the range end
                    chi = chars.next().ok_or(RegexError::EBrack)?;

                    if chi == '\\' && flags.contains(CompileFlags::BRACK_ESCAPE) {
                        chi = chars.next().ok_or(RegexError::EEscape)?;
                    }
                    c_opt = chars.next();
                }
                // If '-' is at end (next is ']' or None), treat as literal
                // and c_opt is already set correctly
            }

            if clo > chi {
                return Err(RegexError::ERange);
            }

            self.set_range(clo, chi);
            if flags.contains(CompileFlags::ICASE) {
                for ch in clo as u32..=chi as u32 {
                    if let Some(c) = char::from_u32(ch) {
                        for lower in c.to_lowercase() {
                            self.set_char(lower);
                        }
                        for upper in c.to_uppercase() {
                            self.set_char(upper);
                        }
                    }
                }
            }
        }

        // Note: The closing ']' has already been consumed by the last chars.next() call
        // at line 323 (or 335 if we processed a range). We don't need to consume it again.

        if inv {
            if flags.contains(CompileFlags::NEWLINE) {
                self.set_char('\n');
            }
            self.invert();
        }

        Ok(())
    }

    pub fn first_bytes(&self) -> (Vec<bool>, Option<u8>) {
        let mut bytes = vec![false; 256];
        let mut count = 0;
        let mut unique = None;

        for range in &self.ranges {
            // For UTF-8, get the first byte of each character
            let lo = self.utf8_first_byte(range.min);
            let hi = self.utf8_first_byte(range.max);

            for (b, byte) in bytes
                .iter_mut()
                .enumerate()
                .skip(lo)
                .take(hi.min(255) - lo + 1)
            {
                if !*byte {
                    *byte = true;
                    count += 1;
                    unique = Some(b as u8);
                }
            }
        }

        if count == 1 {
            (bytes, unique)
        } else {
            (bytes, None)
        }
    }

    fn utf8_first_byte(&self, c: char) -> usize {
        let code = c as u32;
        if code < 0x80 {
            code as usize
        } else if code < 0x800 {
            0xC0 + ((code >> 6) as usize)
        } else if code < 0x10000 {
            0xE0 + ((code >> 12) as usize)
        } else {
            0xF0 + ((code >> 18) as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cset_basic() {
        let mut cset = CSet::new();
        cset.set_char('a');
        assert!(cset.test('a'));
        assert!(!cset.test('b'));
    }

    #[test]
    fn test_cset_range() {
        let mut cset = CSet::new();
        cset.set_range('a', 'z');
        assert!(cset.test('m'));
        assert!(!cset.test('5'));
    }

    #[test]
    fn test_cset_invert() {
        let mut cset = CSet::new();
        cset.set_range('a', 'z');
        cset.invert();
        assert!(!cset.test('m'));
        assert!(cset.test('5'));
    }

    #[test]
    fn test_char_class() {
        let mut cset = CSet::new();
        assert!(cset.add_char_class("digit", CompileFlags::empty()));
        assert!(cset.test('5'));
        assert!(!cset.test('a'));
    }
}
