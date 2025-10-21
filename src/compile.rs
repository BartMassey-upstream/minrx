//! Regular expression compilation
//!
//! Compiles pattern strings into internal node representations.

use crate::cset::CSet;
use crate::node::{Node, NodeType};
use crate::*;
use std::collections::VecDeque;
use std::iter::Peekable;
use std::str::Chars;

type Subexp = (VecDeque<Node>, usize, bool, RegexError);

pub struct Compiler<'a> {
    flags: CompileFlags,
    chars: Peekable<Chars<'a>>,
    current: Option<char>,
    csets: Vec<CSet>,
    dot: Option<usize>,
    esc_s: Option<usize>,
    esc_s_upper: Option<usize>,
    esc_w: Option<usize>,
    esc_w_upper: Option<usize>,
    icmap: HashMap<char, usize>,
    nmin: usize,
    nsub: usize,
}

impl<'a> Compiler<'a> {
    pub fn new(pattern: &'a str, flags: CompileFlags) -> Self {
        let mut chars = pattern.chars().peekable();
        let current = chars.next();

        Self {
            flags,
            chars,
            current,
            csets: Vec::new(),
            dot: None,
            esc_s: None,
            esc_s_upper: None,
            esc_w: None,
            esc_w_upper: None,
            icmap: HashMap::new(),
            nmin: 0,
            nsub: 0,
        }
    }

    fn next_char(&mut self) -> Option<char> {
        self.current = self.chars.next();
        self.current
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    pub fn compile(mut self) -> Result<Regex, RegexError> {
        if self.flags.contains(CompileFlags::MINDISABLE)
            && self.flags.contains(CompileFlags::MINIMAL)
        {
            return Err(RegexError::BadPat);
        }

        let (mut nodes_deque, nstk, _hasmin, err) = self.alt(false, 0);

        if err != RegexError::Success {
            return Err(err);
        }

        nodes_deque.push_back(Node::new(NodeType::Exit, [0, 0], 0));

        if self.nmin > 0 {
            for node in &mut nodes_deque {
                node.nstk += self.nmin;
            }
        }

        let nodes: Vec<Node> = nodes_deque.into_iter().collect();

        Ok(Regex {
            csets: self.csets,
            nodes,
            nmin: self.nmin,
            nstk,
            nsub: self.nsub + 1,
        })
    }

    fn alt(&mut self, nested: bool, nstk: usize) -> Subexp {
        let (mut lhs, mut lhmaxstk, lhasmin, err) = self.cat(nested, nstk);
        if err != RegexError::Success {
            return (lhs, lhmaxstk, lhasmin, err);
        }

        if self.current == Some('|') {
            for node in &mut lhs {
                node.nstk += 1;
            }

            let mut alts = Vec::new();
            while self.current == Some('|') {
                self.next_char();
                alts.push(self.cat(nested, nstk + 1));
            }

            let (mut rhs, mut rhmaxstk, mut rhasmin, err) = alts.pop().unwrap();
            if err != RegexError::Success {
                return (rhs, rhmaxstk, rhasmin, err);
            }

            // Add Goto that skips to the Join (rhs.len() to reach Join position)
            let rhs_len = rhs.len();
            // Goto: args[0] skips rhs to next branch, args[1] skips to Join
            let join_offset = rhs_len + 1;
            rhs.push_front(Node::new(NodeType::Goto, [rhs_len, join_offset], nstk + 1));

            while let Some((mhs, mhmaxstk, mhasmin, _)) = alts.pop() {
                let mhs_len = mhs.len();
                rhs = mhs.into_iter().chain(rhs).collect();
                rhmaxstk = rhmaxstk.max(mhmaxstk);
                rhasmin |= mhasmin;
                // Goto: args[0] skips mhs to next branch, args[1] skips to Join
                let join_offset = rhs.len() + 1;
                rhs.push_front(Node::new(NodeType::Goto, [mhs_len, join_offset], nstk + 1));
            }

            let lhs_len = lhs.len();
            // Fork's right branch skips over lhs to reach the first Goto
            lhs.push_front(Node::new(NodeType::Fork, [lhs_len, 0], nstk + 1));
            lhs.extend(rhs);
            lhmaxstk = lhmaxstk.max(rhmaxstk);
            let final_len = lhs.len();
            lhs.push_back(Node::new(NodeType::Join, [final_len - 1, 0], nstk + 1));
        }

        (lhs, lhmaxstk, lhasmin, RegexError::Success)
    }

    fn cat(&mut self, nested: bool, nstk: usize) -> Subexp {
        let (mut lhs, mut lhmaxstk, mut lhasmin, err) = self.rep(nested, nstk);
        if err != RegexError::Success {
            return (lhs, lhmaxstk, lhasmin, err);
        }

        while self.current.is_some()
            && self.current != Some('|')
            && (self.current != Some(')') || !nested)
        {
            let (rhs, rhmaxstk, rhasmin, err) = self.rep(nested, nstk);
            if err != RegexError::Success {
                return (rhs, rhmaxstk, rhasmin, err);
            }
            lhs.extend(rhs);
            lhmaxstk = lhmaxstk.max(rhmaxstk);
            lhasmin |= rhasmin;
        }

        (lhs, lhmaxstk, lhasmin, RegexError::Success)
    }

    fn rep(&mut self, nested: bool, nstk: usize) -> Subexp {
        let mut lh = self.chr(nested, nstk);
        if lh.3 != RegexError::Success {
            return lh;
        }

        loop {
            match self.current {
                Some('?') => {
                    self.next_char();
                    lh = self.mkrep(lh, true, false, nstk);
                }
                Some('*') => {
                    self.next_char();
                    lh = self.mkrep(lh, true, true, nstk);
                }
                Some('+') => {
                    self.next_char();
                    lh = self.mkrep(lh, false, true, nstk);
                }
                Some('{') => {
                    let debug = std::env::var("MINRX_DEBUG").is_ok();
                    let peek_val = self.peek_char();
                    if debug {
                        eprintln!(
                            "[rep] Found '{{' at position, current={:?}, peek={:?}",
                            self.current, peek_val
                        );
                    }
                    // Check if this should be treated as repetition or literal
                    if self.flags.contains(CompileFlags::BRACE_COMPAT) {
                        // Only treat as repetition if followed by digit
                        if let Some(next) = self.peek_char() {
                            if !next.is_ascii_digit() {
                                return lh;
                            }
                        } else {
                            return lh;
                        }
                    }

                    self.next_char(); // consume '{'
                    if debug {
                        eprintln!("[rep] After consuming '{{', current={:?}", self.current);
                    }
                    match self.parse_brace_repetition(lh, nstk) {
                        Ok(result) => {
                            if debug {
                                eprintln!("[rep] Brace repetition parsed successfully");
                            }
                            lh = result
                        }
                        Err(err) => {
                            if debug {
                                eprintln!("[rep] Brace repetition failed: {:?}", err.3);
                            }
                            return err;
                        }
                    }
                }
                _ => return lh,
            }
        }
    }

    fn parse_brace_repetition(&mut self, lh: Subexp, nstk: usize) -> Result<Subexp, Subexp> {
        if self.current.is_none() {
            return Err((VecDeque::new(), 0, false, RegexError::EBrace));
        }

        // Parse first number
        let m = match self.parse_number() {
            Some(n) => n,
            None => return Err((VecDeque::new(), 0, false, RegexError::BadBr)),
        };

        if self.current == Some('}') {
            // {m} - exact count
            self.next_char();
            Ok(self.mkrep_count(lh, Some(m), Some(m), nstk))
        } else if self.current == Some(',') {
            self.next_char();
            if self.current == Some('}') {
                // {m,} - minimum m, unbounded
                self.next_char();
                Ok(self.mkrep_count(lh, Some(m), None, nstk))
            } else {
                // {m,n} - range
                let n = match self.parse_number() {
                    Some(n) => n,
                    None => return Err((VecDeque::new(), 0, false, RegexError::BadBr)),
                };
                if self.current != Some('}') {
                    // If we hit EOF, it's an unbalanced brace, otherwise invalid contents
                    return Err((
                        VecDeque::new(),
                        0,
                        false,
                        if self.current.is_none() {
                            RegexError::EBrace
                        } else {
                            RegexError::BadBr
                        },
                    ));
                }
                self.next_char();
                Ok(self.mkrep_count(lh, Some(m), Some(n), nstk))
            }
        } else {
            // If we hit EOF, it's an unbalanced brace, otherwise invalid contents
            Err((
                VecDeque::new(),
                0,
                false,
                if self.current.is_none() {
                    RegexError::EBrace
                } else {
                    RegexError::BadBr
                },
            ))
        }
    }

    fn parse_number(&mut self) -> Option<usize> {
        let mut num = 0usize;
        let mut has_digits = false;

        while let Some(c) = self.current {
            if c.is_ascii_digit() {
                has_digits = true;
                num = num
                    .saturating_mul(10)
                    .saturating_add((c as usize) - ('0' as usize));
                self.next_char();
            } else {
                break;
            }
        }

        if has_digits { Some(num) } else { None }
    }

    // Create repetition for *, +, ?
    fn mkrep(&self, lh: Subexp, optional: bool, infinite: bool, nstk: usize) -> Subexp {
        let (mut lhs, lhmaxstk, lhasmin, _) = lh;

        if optional && !infinite {
            // ? operator: zero or one
            for node in &mut lhs {
                node.nstk += 2;
            }
            let lhsize = lhs.len();
            lhs.push_front(Node::new(NodeType::Skip, [lhsize, 0], nstk + 2));
            (lhs, lhmaxstk + 2, lhasmin, RegexError::Success)
        } else {
            // * or + operator: use Loop/Next
            for node in &mut lhs {
                node.nstk += 3;
            }
            let lhsize = lhs.len();
            lhs.push_front(Node::new(
                NodeType::Loop,
                [lhsize, if optional { 1 } else { 0 }],
                nstk + 3,
            ));
            lhs.push_back(Node::new(
                NodeType::Next,
                [lhsize, if infinite { 1 } else { 0 }],
                nstk,
            ));
            (lhs, lhmaxstk + 3, lhasmin, RegexError::Success)
        }
    }

    // Create repetition for {m,n} where None represents unbounded
    fn mkrep_count(
        &self,
        lh: Subexp,
        m_opt: Option<usize>,
        n_opt: Option<usize>,
        nstk: usize,
    ) -> Subexp {
        // Validate bounds
        if let Some(m) = m_opt {
            if m > RE_DUP_MAX {
                return (VecDeque::new(), 0, false, RegexError::BadBr);
            }
        }
        if let Some(n) = n_opt {
            if n > RE_DUP_MAX {
                return (VecDeque::new(), 0, false, RegexError::BadBr);
            }
        }
        // Check m <= n when both are Some
        if let (Some(m), Some(n)) = (m_opt, n_opt) {
            if m > n {
                return (VecDeque::new(), 0, false, RegexError::BadBr);
            }
        }

        let m = m_opt.unwrap_or(0);
        let n_bounded = n_opt;

        if let Some(0) = n_bounded {
            // {0,0} matches empty string
            return (VecDeque::new(), 0, false, RegexError::Success);
        }

        // Handle simple cases
        if m == 0 && n_bounded == Some(1) {
            return self.mkrep(lh, true, false, nstk); // equivalent to ?
        }
        if m == 0 && n_bounded.is_none() {
            return self.mkrep(lh, true, true, nstk); // equivalent to *
        }
        if m == 1 && n_bounded == Some(1) {
            return lh; // {1,1} is just the pattern itself
        }
        if m == 1 && n_bounded.is_none() {
            return self.mkrep(lh, false, true, nstk); // equivalent to +
        }

        let (lhs_orig, lhmaxstk, lhasmin, _) = lh.clone();
        let (rhs_orig, rhmaxstk, _rhasmin, _) = lh;

        let mut lhs = VecDeque::new();
        let mut maxstk = lhmaxstk;

        // Replicate the pattern m times (required part)
        for _ in 0..m {
            lhs.extend(lhs_orig.iter().cloned());
        }

        // Add optional repetitions for n-m times or infinite loop
        if let Some(n) = n_bounded {
            // Finite upper bound: add (n-m) optional copies
            let mut rhs = rhs_orig.clone();
            maxstk = maxstk.max(rhmaxstk + 2);
            for node in &mut rhs {
                node.nstk += 2;
            }
            let rhsize = rhs.len();
            rhs.push_front(Node::new(NodeType::Skip, [rhsize, 1], nstk + 2));

            for _ in m..n {
                lhs.extend(rhs.iter().cloned());
            }
        } else {
            // Unbounded: add infinite loop
            let mut rhs = rhs_orig.clone();
            maxstk = maxstk.max(rhmaxstk + 3);
            for node in &mut rhs {
                node.nstk += 3;
            }
            let rhsize = rhs.len();
            rhs.push_front(Node::new(NodeType::Loop, [rhsize, 1], nstk + 3));
            rhs.push_back(Node::new(NodeType::Next, [rhsize, 1], nstk));
            lhs.extend(rhs);
        }

        // If m == 0, wrap everything in a Skip to make it optional
        if m == 0 {
            return self.mkrep(
                (lhs, maxstk, lhasmin, RegexError::Success),
                true,
                false,
                nstk,
            );
        }

        (lhs, maxstk, lhasmin, RegexError::Success)
    }

    fn chr(&mut self, nested: bool, nstk: usize) -> Subexp {
        let mut lhs = VecDeque::new();
        let lhmaxstk = nstk;

        match self.current {
            Some(c)
                if c.is_alphanumeric()
                    || c.is_ascii_punctuation() && !"*+?|()[]{}^$.\\".contains(c) =>
            {
                // Regular character
                self.handle_char(c, &mut lhs, nstk);
                self.next_char();
            }
            Some('[') => {
                // Don't call next_char() - parse() will consume from self.chars directly
                lhs.push_back(Node::cset_node(self.csets.len(), nstk));
                let mut cset = CSet::new();
                if let Err(e) = cset.parse(self.flags, &mut self.chars) {
                    return (VecDeque::new(), 0, false, e);
                }
                self.csets.push(cset);
                // parse() consumed everything including ']', advance to next char
                self.next_char();
            }
            Some('.') => {
                if self.dot.is_none() {
                    self.dot = Some(self.csets.len());
                    let mut cset = CSet::new();
                    if self.flags.contains(CompileFlags::NEWLINE) {
                        cset.set_char('\n');
                    }
                    cset.invert();
                    self.csets.push(cset);
                }
                lhs.push_back(Node::cset_node(self.dot.unwrap(), nstk));
                self.next_char();
            }
            Some('^') => {
                let node_type = if !self.flags.contains(CompileFlags::NEWLINE) {
                    NodeType::ZBOB
                } else {
                    NodeType::ZBOL
                };
                lhs.push_back(Node::new(node_type, [0, 0], nstk));
                self.next_char();
            }
            Some('$') => {
                let node_type = if !self.flags.contains(CompileFlags::NEWLINE) {
                    NodeType::ZEOB
                } else {
                    NodeType::ZEOL
                };
                lhs.push_back(Node::new(node_type, [0, 0], nstk));
                self.next_char();
            }
            Some('(') => {
                self.nsub += 1;
                let n = self.nsub;
                self.next_char();
                let (mut sublhs, submaxstk, subhasmin, err) = self.alt(true, nstk + 1);
                if err != RegexError::Success {
                    return (sublhs, submaxstk, subhasmin, err);
                }
                if self.current != Some(')') {
                    return (VecDeque::new(), 0, false, RegexError::EParen);
                }
                sublhs.push_front(Node::new(NodeType::SubL, [n, self.nsub], nstk + 1));
                sublhs.push_back(Node::new(NodeType::SubR, [n, self.nsub], nstk));
                self.next_char();
                return (sublhs, submaxstk, subhasmin, RegexError::Success);
            }
            Some(')') if !nested => {
                // Treat as literal
                lhs.push_back(Node::char_node(')', nstk));
                self.next_char();
            }
            Some('\\') => {
                self.next_char();
                match self.current {
                    None => return (VecDeque::new(), 0, false, RegexError::EEscape),
                    Some('<') if self.flags.contains(CompileFlags::EXTENSIONS_BSD) => {
                        lhs.push_back(Node::new(NodeType::ZBOW, [0, 0], nstk));
                        self.next_char();
                    }
                    Some('>') if self.flags.contains(CompileFlags::EXTENSIONS_BSD) => {
                        lhs.push_back(Node::new(NodeType::ZEOW, [0, 0], nstk));
                        self.next_char();
                    }
                    Some('`') if self.flags.contains(CompileFlags::EXTENSIONS_GNU) => {
                        lhs.push_back(Node::new(NodeType::ZBOB, [0, 0], nstk));
                        self.next_char();
                    }
                    Some('\'') if self.flags.contains(CompileFlags::EXTENSIONS_GNU) => {
                        lhs.push_back(Node::new(NodeType::ZEOB, [0, 0], nstk));
                        self.next_char();
                    }
                    Some('b') if self.flags.contains(CompileFlags::EXTENSIONS_GNU) => {
                        lhs.push_back(Node::new(NodeType::ZXOW, [0, 0], nstk));
                        self.next_char();
                    }
                    Some('B') if self.flags.contains(CompileFlags::EXTENSIONS_GNU) => {
                        lhs.push_back(Node::new(NodeType::ZNWB, [0, 0], nstk));
                        self.next_char();
                    }
                    Some('s') if self.flags.contains(CompileFlags::EXTENSIONS_GNU) => {
                        if self.esc_s.is_none() {
                            self.esc_s = Some(self.csets.len());
                            let mut cset = CSet::new();
                            cset.add_char_class("space", self.flags);
                            self.csets.push(cset);
                        }
                        lhs.push_back(Node::cset_node(self.esc_s.unwrap(), nstk));
                        self.next_char();
                    }
                    Some('S') if self.flags.contains(CompileFlags::EXTENSIONS_GNU) => {
                        if self.esc_s_upper.is_none() {
                            self.esc_s_upper = Some(self.csets.len());
                            let mut cset = CSet::new();
                            cset.add_char_class("space", self.flags);
                            cset.invert();
                            self.csets.push(cset);
                        }
                        lhs.push_back(Node::cset_node(self.esc_s_upper.unwrap(), nstk));
                        self.next_char();
                    }
                    Some('w') if self.flags.contains(CompileFlags::EXTENSIONS_GNU) => {
                        if self.esc_w.is_none() {
                            self.esc_w = Some(self.csets.len());
                            let mut cset = CSet::new();
                            cset.add_char_class("alnum", self.flags);
                            cset.set_char('_');
                            self.csets.push(cset);
                        }
                        lhs.push_back(Node::cset_node(self.esc_w.unwrap(), nstk));
                        self.next_char();
                    }
                    Some('W') if self.flags.contains(CompileFlags::EXTENSIONS_GNU) => {
                        if self.esc_w_upper.is_none() {
                            self.esc_w_upper = Some(self.csets.len());
                            let mut cset = CSet::new();
                            cset.add_char_class("alnum", self.flags);
                            cset.set_char('_');
                            cset.invert();
                            self.csets.push(cset);
                        }
                        lhs.push_back(Node::cset_node(self.esc_w_upper.unwrap(), nstk));
                        self.next_char();
                    }
                    Some(c) => {
                        self.handle_char(c, &mut lhs, nstk);
                        self.next_char();
                    }
                }
            }
            Some('{') => {
                // Check if this should be treated as repetition or literal
                if self.flags.contains(CompileFlags::BRACE_COMPAT) {
                    // Only treat as error if followed by digit
                    if let Some(next) = self.peek_char() {
                        if next.is_ascii_digit() {
                            return (VecDeque::new(), 0, false, RegexError::BadRpt);
                        }
                    }
                    // Treat as literal
                    lhs.push_back(Node::char_node('{', nstk));
                    self.next_char();
                } else {
                    // Without BRACE_COMPAT, { at start is always an error
                    return (VecDeque::new(), 0, false, RegexError::BadRpt);
                }
            }
            Some('*') | Some('+') | Some('?') => {
                // Repetition operator without preceding pattern
                return (VecDeque::new(), 0, false, RegexError::BadRpt);
            }
            Some('|') | Some(')') | None => {
                // Empty expression
            }
            Some(c) => {
                // Other character
                self.handle_char(c, &mut lhs, nstk);
                self.next_char();
            }
        }

        (lhs, lhmaxstk, false, RegexError::Success)
    }

    fn handle_char(&mut self, c: char, lhs: &mut VecDeque<Node>, nstk: usize) {
        if !self.flags.contains(CompileFlags::ICASE) {
            lhs.push_back(Node::char_node(c, nstk));
        } else {
            let lower: Vec<char> = c.to_lowercase().collect();
            let upper: Vec<char> = c.to_uppercase().collect();

            // Check if case matters
            if lower.len() == 1 && upper.len() == 1 && (lower[0] != c || upper[0] != c) {
                let key = c.min(lower[0]).min(upper[0]);
                if !self.icmap.contains_key(&key) {
                    let idx = self.csets.len();
                    self.icmap.insert(key, idx);
                    let mut cs = CSet::new();
                    cs.set_char(c);
                    cs.set_char(lower[0]);
                    cs.set_char(upper[0]);
                    self.csets.push(cs);
                }
                lhs.push_back(Node::cset_node(self.icmap[&key], nstk));
            } else {
                lhs.push_back(Node::char_node(c, nstk));
            }
        }
    }
}

pub fn compile(pattern: &str, flags: CompileFlags) -> Result<Regex, RegexError> {
    let compiler = Compiler::new(pattern, flags);
    compiler.compile()
}
