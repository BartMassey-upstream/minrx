//! Integration tests for MinRX regex engine
//!
//! Based on the original C test suite

use minrx::{CompileFlags, ExecFlags, Regex, RegexError};

// Test helper function
fn test_match(name: &str, pattern: &str, text: &str, should_match: bool, flags: CompileFlags) {
    let result = Regex::new(pattern, flags | CompileFlags::EXTENDED);

    match result {
        Ok(regex) => {
            let exec_result = regex.exec(text, ExecFlags::empty());
            let matched = exec_result.is_ok();

            if matched == should_match {
                println!("PASS: {}", name);
            } else {
                panic!(
                    "FAIL: {} - expected {}, got {}",
                    name,
                    if should_match { "match" } else { "no match" },
                    if matched { "match" } else { "no match" }
                );
            }
        }
        Err(e) => {
            panic!("FAIL: {} - compilation failed: {:?}", name, e);
        }
    }
}

#[test]
fn test_basic_matching() {
    println!("\n=== Basic Character Matching ===");

    test_match("literal char match", "a", "a", true, CompileFlags::empty());
    test_match(
        "literal char no match",
        "a",
        "b",
        false,
        CompileFlags::empty(),
    );
    test_match(
        "literal string match",
        "abc",
        "abc",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "literal string in middle",
        "abc",
        "xabcy",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "literal string no match",
        "abc",
        "abd",
        false,
        CompileFlags::empty(),
    );
    test_match("empty pattern", "", "anything", true, CompileFlags::empty());
    test_match("empty text", "a", "", false, CompileFlags::empty());
    test_match("empty both", "", "", true, CompileFlags::empty());
}

#[test]
fn test_character_classes() {
    println!("\n=== Character Classes ===");

    test_match("dot matches char", ".", "a", true, CompileFlags::empty());
    test_match("dot matches any", "a.c", "abc", true, CompileFlags::empty());
    test_match("dot matches digit", ".", "5", true, CompileFlags::empty());
    test_match(
        "dot doesn't match newline (with REG_NEWLINE)",
        ".",
        "\n",
        false,
        CompileFlags::NEWLINE,
    );
    test_match(
        "dot matches newline (without REG_NEWLINE)",
        ".",
        "\n",
        true,
        CompileFlags::empty(),
    );

    test_match(
        "bracket single char",
        "[a]",
        "a",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "bracket multiple chars",
        "[abc]",
        "b",
        true,
        CompileFlags::empty(),
    );
    test_match("bracket range", "[a-z]", "m", true, CompileFlags::empty());
    test_match(
        "bracket range no match",
        "[a-z]",
        "5",
        false,
        CompileFlags::empty(),
    );
    test_match(
        "bracket negation",
        "[^a-z]",
        "5",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "bracket negation no match",
        "[^a-z]",
        "m",
        false,
        CompileFlags::empty(),
    );
}

#[test]
fn test_alternation() {
    println!("\n=== Alternation ===");

    test_match(
        "simple alternation left",
        "a|b",
        "a",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "simple alternation right",
        "a|b",
        "b",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "simple alternation no match",
        "a|b",
        "c",
        false,
        CompileFlags::empty(),
    );
    test_match(
        "multi alternation",
        "a|b|c",
        "b",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "alternation with concat",
        "ab|cd",
        "ab",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "alternation with concat right",
        "ab|cd",
        "cd",
        true,
        CompileFlags::empty(),
    );
}

#[test]
fn test_grouping() {
    println!("\n=== Grouping and Subexpressions ===");

    test_match("simple group", "(abc)", "abc", true, CompileFlags::empty());
    test_match("nested groups", "((a))", "a", true, CompileFlags::empty());
    test_match(
        "group with alternation",
        "(a|b)",
        "a",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "multiple groups",
        "(a)(b)",
        "ab",
        true,
        CompileFlags::empty(),
    );
}

#[test]
fn test_anchors() {
    println!("\n=== Anchors ===");

    test_match(
        "anchor BOL match",
        "^abc",
        "abc",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "anchor BOL no match",
        "^abc",
        "xabc",
        false,
        CompileFlags::empty(),
    );

    test_match(
        "anchor EOL match",
        "abc$",
        "abc",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "anchor EOL no match",
        "abc$",
        "abcx",
        false,
        CompileFlags::empty(),
    );

    test_match(
        "anchor BOL+EOL exact",
        "^abc$",
        "abc",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "anchor BOL+EOL too long",
        "^abc$",
        "abcd",
        false,
        CompileFlags::empty(),
    );
    test_match(
        "anchor BOL+EOL prefix",
        "^abc$",
        "xabc",
        false,
        CompileFlags::empty(),
    );

    test_match("anchor empty BOL", "^$", "", true, CompileFlags::empty());
    test_match(
        "anchor empty BOL no match",
        "^$",
        "a",
        false,
        CompileFlags::empty(),
    );
}

#[test]
fn test_case_insensitive() {
    println!("\n=== Case Insensitive Matching ===");

    test_match(
        "icase simple lower",
        "abc",
        "ABC",
        true,
        CompileFlags::ICASE,
    );
    test_match(
        "icase simple upper",
        "ABC",
        "abc",
        true,
        CompileFlags::ICASE,
    );
    test_match("icase mixed", "AbC", "aBc", true, CompileFlags::ICASE);
}

#[test]
fn test_complex_patterns() {
    println!("\n=== Complex Patterns ===");

    // Simple patterns that should work with basic implementation
    test_match("simple concat", "abc", "abc", true, CompileFlags::empty());
    test_match(
        "alternation in group",
        "(a|b)c",
        "ac",
        true,
        CompileFlags::empty(),
    );
    test_match(
        "alternation in group 2",
        "(a|b)c",
        "bc",
        true,
        CompileFlags::empty(),
    );
}

#[test]
fn test_edge_cases() {
    println!("\n=== Edge Cases ===");

    test_match("empty subexpr", "()", "", true, CompileFlags::empty());
    test_match(
        "empty subexpr in concat",
        "a()b",
        "ab",
        true,
        CompileFlags::empty(),
    );

    test_match("multiple dots", "...", "abc", true, CompileFlags::empty());

    // Bracket edge cases - simplified for basic implementation
    test_match(
        "bracket with dash at end",
        "[a-]",
        "-",
        true,
        CompileFlags::empty(),
    );
}

#[test]
fn test_compilation_errors() {
    println!("\n=== Compilation Error Handling ===");

    // Test that invalid patterns return errors
    assert!(matches!(
        Regex::new("(abc", CompileFlags::EXTENDED),
        Err(RegexError::EParen)
    ));

    assert!(matches!(
        Regex::new("[abc", CompileFlags::EXTENDED),
        Err(RegexError::EBrack)
    ));
}

#[test]
fn test_simple_patterns() {
    println!("\n=== Simple Pattern Tests ===");

    // Very basic patterns to ensure core functionality works
    let re = Regex::new("hello", CompileFlags::EXTENDED).expect("Failed to compile");
    assert!(re.exec("hello", ExecFlags::empty()).is_ok());
    assert!(re.exec("hello world", ExecFlags::empty()).is_ok());
    assert!(re.exec("goodbye", ExecFlags::empty()).is_err());

    let re = Regex::new("a", CompileFlags::EXTENDED).expect("Failed to compile");
    assert!(re.exec("a", ExecFlags::empty()).is_ok());
    assert!(re.exec("banana", ExecFlags::empty()).is_ok());
    assert!(re.exec("b", ExecFlags::empty()).is_err());
}
