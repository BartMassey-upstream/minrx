//! Regular expression execution engine
//!
//! Executes compiled regular expressions against input text.

use crate::data_structures::{COWVec, QSet, QVec};
use crate::node::NodeType;
use crate::*;
use std::iter::Peekable;
use std::str::Chars;

struct NState {
    gen: usize,
    boff: usize,
    substack: COWVec<usize>,
}

impl NState {
    fn new(allocator_size: usize) -> Self {
        Self {
            gen: 0,
            boff: 0,
            substack: COWVec::new(allocator_size, usize::MAX),
        }
    }

    fn clone_state(&self) -> Self {
        Self {
            gen: self.gen,
            boff: self.boff,
            substack: self.substack.clone(),
        }
    }
}

pub struct Executor<'a> {
    regex: &'a Regex,
    flags: ExecFlags,
    suboff: usize,
    gen: usize,
    off: usize,
    chars: Peekable<Chars<'a>>,
    prev_char: Option<char>,
    best: Option<COWVec<usize>>,
    epsq: QSet,
    epsv: QVec<NState>,
}

impl<'a> Executor<'a> {
    pub fn new(regex: &'a Regex, text: &'a str, flags: ExecFlags) -> Self {
        let suboff = regex.nmin + regex.nstk;

        Self {
            regex,
            flags,
            suboff,
            gen: 0,
            off: 0,
            chars: text.chars().peekable(),
            prev_char: None,
            best: None,
            epsq: QSet::new(regex.nodes.len()),
            epsv: QVec::new(regex.nodes.len()),
        }
    }

    fn is_word_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    pub fn execute(&mut self) -> Result<Vec<RegMatch>, RegexError> {
        let mut mcsvs = [
            QVec::new(self.regex.nodes.len()),
            QVec::new(self.regex.nodes.len()),
        ];

        let mut next_char = self.chars.peek().copied();

        let mut nsinit = NState::new(self.suboff + 2 * self.regex.nsub);
        nsinit.boff = self.off;

        self.add(&mut mcsvs[0], 0, 0, &nsinit, next_char);
        if !self.epsq.is_empty() {
            self.epsclosure(&mut mcsvs[0], next_char);
        }

        let mut current = 0;
        loop {
            if next_char.is_none() {
                break;
            }

            self.gen += 1;
            self.prev_char = next_char;
            self.chars.next(); // consume the character
            self.off += self.prev_char.map(|c| c.len_utf8()).unwrap_or(0);
            next_char = self.chars.peek().copied();

            while !mcsvs[current].is_empty() {
                let (n, ns) = mcsvs[current].remove();
                if let Some(node) = self.regex.nodes.get(n) {
                    let nstk = node.nstk;
                    self.add(&mut mcsvs[1 - current], n + 1, nstk, &ns, next_char);
                }
            }

            if self.best.is_none() {
                nsinit.boff = self.off;
                self.add(&mut mcsvs[1 - current], 0, 0, &nsinit, next_char);
            }

            if !self.epsq.is_empty() {
                self.epsclosure(&mut mcsvs[1 - current], next_char);
            }

            if mcsvs[1 - current].is_empty() && self.best.is_some() {
                break;
            }

            current = 1 - current;
        }

        if let Some(ref best) = self.best {
            let mut matches = Vec::with_capacity(self.regex.nsub);
            for i in 0..self.regex.nsub {
                let start = best.get(self.suboff + i * 2) as isize;
                let end = best.get(self.suboff + i * 2 + 1) as isize;
                matches.push(RegMatch::new(start, end));
            }
            Ok(matches)
        } else {
            Err(RegexError::NoMatch)
        }
    }

    fn add(
        &mut self,
        ncsv: &mut QVec<NState>,
        k: usize,
        _nstk: usize,
        ns: &NState,
        next_char: Option<char>,
    ) {
        if k >= self.regex.nodes.len() {
            return;
        }

        let node = &self.regex.nodes[k];

        let debug = std::env::var("MINRX_DEBUG").is_ok();
        if debug {
            eprintln!(
                "[add] k={} nstk={} gen={} off={} node={:?} next_char={:?}",
                k, _nstk, self.gen, self.off, node.node_type, next_char
            );
        }

        match node.node_type {
            NodeType::Char(c) => {
                if Some(c) == next_char {
                    let (newly, slot) = ncsv.insert(k);
                    if newly {
                        let mut new_state = ns.clone_state();
                        new_state.gen = self.gen;
                        *slot = Some(new_state);
                        if debug {
                            eprintln!("  -> Char matched '{:?}', added to ncsv (newly)", c);
                        }
                    } else {
                        // Update if newer generation or better state
                        if let Some(ref existing) = *slot {
                            let existing_gen = existing.gen;
                            if self.gen > existing.gen
                                || (self.gen == existing.gen && ns.boff < existing.boff)
                            {
                                let mut new_state = ns.clone_state();
                                new_state.gen = self.gen;
                                *slot = Some(new_state);
                                if debug {
                                    eprintln!(
                                        "  -> Char matched '{:?}', updated in ncsv (gen {} > {})",
                                        c, self.gen, existing_gen
                                    );
                                }
                            } else if debug {
                                eprintln!(
                                    "  -> Char matched '{:?}', skipped (existing gen {})",
                                    c, existing_gen
                                );
                            }
                        }
                    }
                } else if debug {
                    eprintln!("  -> Char '{:?}' did not match '{:?}'", c, next_char);
                }
            }
            NodeType::CSet => {
                if let Some(nc) = next_char {
                    if let Some(cset) = self.regex.csets.get(node.args[0]) {
                        if cset.test(nc) {
                            let (newly, slot) = ncsv.insert(k);
                            if newly {
                                let mut new_state = ns.clone_state();
                                new_state.gen = self.gen;
                                *slot = Some(new_state);
                                if debug {
                                    eprintln!(
                                        "  -> CSet matched '{:?}', added to ncsv (newly)",
                                        nc
                                    );
                                }
                            } else {
                                // Update if newer generation or better state
                                if let Some(ref existing) = *slot {
                                    let existing_gen = existing.gen;
                                    if self.gen > existing.gen
                                        || (self.gen == existing.gen && ns.boff < existing.boff)
                                    {
                                        let mut new_state = ns.clone_state();
                                        new_state.gen = self.gen;
                                        *slot = Some(new_state);
                                        if debug {
                                            eprintln!("  -> CSet matched '{:?}', updated in ncsv (gen {} > {})", nc, self.gen, existing_gen);
                                        }
                                    } else if debug {
                                        eprintln!(
                                            "  -> CSet matched '{:?}', skipped (existing gen {})",
                                            nc, existing_gen
                                        );
                                    }
                                }
                            }
                        } else if debug {
                            eprintln!("  -> CSet did not match '{:?}'", nc);
                        }
                    }
                }
            }
            _ => {
                let (newly, slot) = self.epsv.insert(k);
                let should_process = if newly {
                    let mut new_state = ns.clone_state();
                    new_state.gen = self.gen;
                    *slot = Some(new_state);
                    if debug {
                        eprintln!("  -> Epsilon node added to epsv (newly)");
                    }
                    true
                } else {
                    // Update if newer generation or better state
                    if let Some(ref existing) = *slot {
                        let existing_gen = existing.gen;
                        if self.gen > existing.gen
                            || (self.gen == existing.gen && ns.boff < existing.boff)
                        {
                            let mut new_state = ns.clone_state();
                            new_state.gen = self.gen;
                            *slot = Some(new_state);
                            if debug {
                                eprintln!(
                                    "  -> Epsilon node updated in epsv (gen {} > {})",
                                    self.gen, existing_gen
                                );
                            }
                            true
                        } else {
                            if debug {
                                eprintln!(
                                    "  -> Epsilon node skipped (existing gen {})",
                                    existing_gen
                                );
                            }
                            false
                        }
                    } else {
                        false
                    }
                };

                if should_process {
                    self.epsq.insert(k);
                    if debug {
                        eprintln!("  -> Added to epsq for processing");
                    }
                }
            }
        }
    }

    fn epsclosure(&mut self, ncsv: &mut QVec<NState>, next_char: Option<char>) {
        while !self.epsq.is_empty() {
            let k = self.epsq.remove();
            let ns_clone = if let Some(ns) = self.epsv.lookup(k) {
                ns.clone_state()
            } else {
                continue;
            };

            if k >= self.regex.nodes.len() {
                continue;
            }

            let node = self.regex.nodes[k].clone();
            let nstk = node.nstk;

            match node.node_type {
                NodeType::Exit => {
                    let b = ns_clone.boff;
                    let e = self.off;
                    let should_update = if let Some(ref best) = self.best {
                        let best_b = best.get(self.suboff);
                        let best_e = best.get(self.suboff + 1);
                        // Accept new match if: earlier start (leftmost) OR same start but longer (greedy)
                        b < best_b || (b == best_b && e > best_e)
                    } else {
                        true // No previous match
                    };

                    if should_update {
                        let mut new_best = ns_clone.substack.clone();
                        new_best.put(self.suboff, b);
                        new_best.put(self.suboff + 1, e);
                        self.best = Some(new_best);
                    }
                }
                NodeType::Fork => {
                    // Fork: loop through branches until we find a Join
                    // Each iteration adds the next branch (kk+1) and advances kk
                    let mut kk = k;
                    loop {
                        let nscopy = ns_clone.clone_state();
                        self.add(ncsv, kk + 1, nstk, &nscopy, next_char);

                        // Advance to next branch using args[0]
                        kk = kk + 1 + self.regex.nodes[kk].args[0];

                        // Stop when we reach a Join node
                        if kk >= self.regex.nodes.len()
                            || matches!(self.regex.nodes[kk].node_type, NodeType::Join)
                        {
                            break;
                        }
                    }
                }
                NodeType::Goto => {
                    let nscopy = ns_clone.clone_state();
                    self.add(ncsv, k + 1 + node.args[1], nstk, &nscopy, next_char);
                }
                NodeType::Join => {
                    let nscopy = ns_clone.clone_state();
                    self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                }
                NodeType::SubL => {
                    let mut nscopy = ns_clone.clone_state();
                    nscopy.substack.put(nstk - 1, self.off);

                    // Reset subsequent submatches if NOSUBRESET is not set
                    if node.args[0] != usize::MAX && !self.flags.contains(ExecFlags::NOSUBRESET) {
                        for i in (node.args[0] + 1)..=node.args[1] {
                            nscopy.substack.put(self.suboff + i * 2, usize::MAX);
                            nscopy.substack.put(self.suboff + i * 2 + 1, usize::MAX);
                        }
                    }

                    self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                }
                NodeType::SubR => {
                    // Only save submatch if args[0] is valid and either FIRSTSUB is not set
                    // or the submatch hasn't been captured yet
                    if node.args[0] != usize::MAX
                        && (!self.flags.contains(ExecFlags::FIRSTSUB)
                            || ns_clone.substack.get(self.suboff + node.args[0] * 2) == usize::MAX)
                    {
                        let mut nscopy = ns_clone.clone_state();
                        nscopy
                            .substack
                            .put(self.suboff + node.args[0] * 2, ns_clone.substack.get(nstk));
                        nscopy
                            .substack
                            .put(self.suboff + node.args[0] * 2 + 1, self.off);
                        self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                    } else {
                        // Still need to continue execution even if we don't save the submatch
                        let nscopy = ns_clone.clone_state();
                        self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                    }
                }
                NodeType::ZBOB => {
                    if self.off == 0 && !self.flags.contains(ExecFlags::NOTBOL) {
                        let nscopy = ns_clone.clone_state();
                        self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                    }
                }
                NodeType::ZEOB => {
                    if next_char.is_none() && !self.flags.contains(ExecFlags::NOTEOL) {
                        let nscopy = ns_clone.clone_state();
                        self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                    }
                }
                NodeType::ZBOL => {
                    if (self.off == 0 && !self.flags.contains(ExecFlags::NOTBOL))
                        || self.prev_char == Some('\n')
                    {
                        let nscopy = ns_clone.clone_state();
                        self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                    }
                }
                NodeType::ZEOL => {
                    if (next_char.is_none() && !self.flags.contains(ExecFlags::NOTEOL))
                        || next_char == Some('\n')
                    {
                        let nscopy = ns_clone.clone_state();
                        self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                    }
                }
                NodeType::Loop => {
                    // Loop node: enter the loop and optionally skip it
                    // Store 3 values on stack: [nstk-3]=off, [nstk-2]=-1, [nstk-1]=off
                    let debug = std::env::var("MINRX_DEBUG").is_ok();
                    if debug {
                        eprintln!(
                            "[Loop] k={} args=[{},{}] entering loop",
                            k, node.args[0], node.args[1]
                        );
                    }
                    let mut nscopy1 = ns_clone.clone_state();
                    nscopy1.substack.put(nstk - 3, self.off);
                    nscopy1.substack.put(nstk - 2, usize::MAX); // -1
                    nscopy1.substack.put(nstk - 1, self.off);
                    self.add(ncsv, k + 1, nstk, &nscopy1, next_char);

                    // If args[1] is 1 (optional), also add branch that skips the loop
                    if node.args[1] == 1 {
                        if debug {
                            eprintln!(
                                "[Loop] k={} optional, also adding skip branch to k={}",
                                k,
                                k + 1 + node.args[0]
                            );
                        }
                        let mut nscopy2 = ns_clone.clone_state();
                        nscopy2.substack.put(nstk - 3, self.off);
                        nscopy2.substack.put(nstk - 2, 0); // Start with 0 for skip branch
                        nscopy2.substack.put(nstk - 1, self.off);
                        self.add(ncsv, k + 1 + node.args[0], nstk, &nscopy2, next_char);
                    }
                }
                NodeType::Next => {
                    // Next node: exit the loop or loop back if infinite and made progress
                    let debug = std::env::var("MINRX_DEBUG").is_ok();
                    if debug {
                        eprintln!(
                            "[Next] k={} args=[{},{}] exiting loop",
                            k, node.args[0], node.args[1]
                        );
                    }
                    let nscopy1 = ns_clone.clone_state();
                    self.add(ncsv, k + 1, nstk, &nscopy1, next_char);

                    // If args[1] is 1 (infinite) and we made progress, loop back
                    if node.args[1] == 1 {
                        let loop_start_off = ns_clone.substack.get(nstk + 2);
                        if debug {
                            eprintln!("[Next] k={} infinite loop, checking progress: off={} vs loop_start={}",
                                     k, self.off, loop_start_off);
                        }
                        if self.off > loop_start_off {
                            if debug {
                                eprintln!(
                                    "[Next] k={} made progress, looping back to k={}",
                                    k,
                                    k - node.args[0]
                                );
                            }
                            let mut nscopy2 = ns_clone.clone_state();
                            // Copy the three values and update the last one
                            let val0 = ns_clone.substack.get(nstk);
                            let val1 = ns_clone.substack.get(nstk + 1);
                            nscopy2.substack.put(nstk, val0);
                            nscopy2.substack.put(nstk + 1, val1.wrapping_sub(1));
                            nscopy2.substack.put(nstk + 2, self.off);
                            self.add(ncsv, k - node.args[0], nstk + 3, &nscopy2, next_char);
                        } else if debug {
                            eprintln!("[Next] k={} no progress, not looping back", k);
                        }
                    }
                }
                NodeType::Skip => {
                    // Skip node: two branches (execute or skip)
                    let nscopy1 = ns_clone.clone_state();
                    let nscopy2 = ns_clone.clone_state();
                    self.add(ncsv, k + 1, nstk, &nscopy1, next_char);
                    self.add(ncsv, k + 1 + node.args[0], nstk, &nscopy2, next_char);
                }
                NodeType::ZBOW => {
                    // Beginning of word: (at start OR not word before) AND (not at end AND word after)
                    let at_start = self.off == 0;
                    let prev_is_word = self.prev_char.map(Self::is_word_char).unwrap_or(false);
                    let at_end = next_char.is_none();
                    let next_is_word = next_char.map(Self::is_word_char).unwrap_or(false);

                    if (at_start || !prev_is_word) && (!at_end && next_is_word) {
                        let nscopy = ns_clone.clone_state();
                        self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                    }
                }
                NodeType::ZEOW => {
                    // End of word: (not at start AND word before) AND (at end OR not word after)
                    let at_start = self.off == 0;
                    let prev_is_word = self.prev_char.map(Self::is_word_char).unwrap_or(false);
                    let at_end = next_char.is_none();
                    let next_is_word = next_char.map(Self::is_word_char).unwrap_or(false);

                    if (!at_start && prev_is_word) && (at_end || !next_is_word) {
                        let nscopy = ns_clone.clone_state();
                        self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                    }
                }
                NodeType::ZXOW => {
                    // Word boundary: at beginning OR end of word
                    let at_start = self.off == 0;
                    let prev_is_word = self.prev_char.map(Self::is_word_char).unwrap_or(false);
                    let at_end = next_char.is_none();
                    let next_is_word = next_char.map(Self::is_word_char).unwrap_or(false);

                    let is_bow = (at_start || !prev_is_word) && (!at_end && next_is_word);
                    let is_eow = (!at_start && prev_is_word) && (at_end || !next_is_word);

                    if is_bow || is_eow {
                        let nscopy = ns_clone.clone_state();
                        self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                    }
                }
                NodeType::ZNWB => {
                    // Not word boundary: both word or both not word
                    let at_start = self.off == 0;
                    let prev_is_word = self.prev_char.map(Self::is_word_char).unwrap_or(false);
                    let at_end = next_char.is_none();
                    let next_is_word = next_char.map(Self::is_word_char).unwrap_or(false);

                    // Match if: (start AND end) OR (start AND not word after) OR (end AND not word before) OR (both same type)
                    let matches = at_start && (at_end || !next_is_word)
                        || at_end && !prev_is_word
                        || (!at_start && !at_end && prev_is_word == next_is_word);

                    if matches {
                        let nscopy = ns_clone.clone_state();
                        self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                    }
                }
                _ => {
                    // Other node types not yet implemented
                    let nscopy = ns_clone.clone_state();
                    self.add(ncsv, k + 1, nstk, &nscopy, next_char);
                }
            }
        }
    }
}

pub fn execute(regex: &Regex, text: &str, flags: ExecFlags) -> Result<Vec<RegMatch>, RegexError> {
    let mut executor = Executor::new(regex, text, flags);
    executor.execute()
}
