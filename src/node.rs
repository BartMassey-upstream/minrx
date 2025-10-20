//! Regular expression AST nodes
//!
//! Defines the internal representation of compiled regular expressions.

pub type NInt = usize;

// Allow uppercase acronyms to match C++ implementation naming (ZBOB = Zero-width Beginning Of Buffer, etc.)
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    // Character-matching nodes
    Char(char),
    CSet,

    // Epsilon-matching nodes
    Exit,
    Fork,
    Goto,
    Join,
    Loop,
    Next,
    Skip,
    SubL,
    SubR,

    // Zero-width assertions
    ZBOB, // Beginning of buffer
    ZEOB, // End of buffer
    ZBOL, // Beginning of line
    ZEOL, // End of line
    ZBOW, // Beginning of word
    ZEOW, // End of word
    ZXOW, // Either end of word (word boundary)
    ZNWB, // Not word boundary
}

#[derive(Debug, Clone)]
pub struct Node {
    pub node_type: NodeType,
    pub args: [NInt; 2],
    pub nstk: NInt,
}

impl Node {
    pub fn new(node_type: NodeType, args: [NInt; 2], nstk: NInt) -> Self {
        Self {
            node_type,
            args,
            nstk,
        }
    }

    pub fn char_node(c: char, nstk: NInt) -> Self {
        Self::new(NodeType::Char(c), [0, 0], nstk)
    }

    pub fn cset_node(idx: NInt, nstk: NInt) -> Self {
        Self::new(NodeType::CSet, [idx, 0], nstk)
    }
}
