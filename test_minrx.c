//
// Test suite for MinRX - tests all traditional egrep functionality
//

#include <locale.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "minrx.h"

static int tests_run = 0;
static int tests_passed = 0;
static int tests_failed = 0;

// Test helper function
static void
test_match(const char *name, const char *pattern, const char *text,
	   int should_match, int flags)
{
	minrx_regex_t rx;
	int compile_err, exec_result;
	char errmsg[100];

	tests_run++;

	compile_err = minrx_regcomp(&rx, pattern, flags | MINRX_REG_EXTENDED);
	if (compile_err != 0)
	{
		minrx_regerror(compile_err, &rx, errmsg, sizeof(errmsg));
		printf("FAIL: %s - compilation failed: %s\n", name, errmsg);
		tests_failed++;
		return;
	}

	exec_result = minrx_regexec(&rx, text, 0, NULL, 0);
	minrx_regfree(&rx);

	if (should_match && exec_result == 0)
	{
		printf("PASS: %s\n", name);
		tests_passed++;
	}
	else if (!should_match && exec_result != 0)
	{
		printf("PASS: %s\n", name);
		tests_passed++;
	}
	else
	{
		printf("FAIL: %s - expected %s, got %s\n", name,
		       should_match ? "match" : "no match",
		       exec_result == 0 ? "match" : "no match");
		tests_failed++;
	}
}

// Test helper for subexpression capture
static void
test_submatch(const char *name, const char *pattern, const char *text,
	      int nsubs, const int *expected_offsets, int flags)
{
	minrx_regex_t rx;
	minrx_regmatch_t rm[10];
	int compile_err, exec_result;
	char errmsg[100];
	int i;

	tests_run++;

	compile_err = minrx_regcomp(&rx, pattern, flags | MINRX_REG_EXTENDED);
	if (compile_err != 0)
	{
		minrx_regerror(compile_err, &rx, errmsg, sizeof(errmsg));
		printf("FAIL: %s - compilation failed: %s\n", name, errmsg);
		tests_failed++;
		return;
	}

	exec_result = minrx_regexec(&rx, text, 10, rm, 0);
	minrx_regfree(&rx);

	if (exec_result != 0)
	{
		printf("FAIL: %s - no match found\n", name);
		tests_failed++;
		return;
	}

	for (i = 0; i < nsubs; i++)
	{
		if (rm[i].rm_so != expected_offsets[i * 2] ||
		    rm[i].rm_eo != expected_offsets[i * 2 + 1])
		{
			printf("FAIL: %s - submatch %d: expected (%d,%d), got (%d,%d)\n",
			       name, i, expected_offsets[i * 2],
			       expected_offsets[i * 2 + 1], (int) rm[i].rm_so,
			       (int) rm[i].rm_eo);
			tests_failed++;
			return;
		}
	}

	printf("PASS: %s\n", name);
	tests_passed++;
}

// Test helper for compile errors
static void
test_compile_error(const char *name, const char *pattern,
		   minrx_result_t expected_error, int flags)
{
	minrx_regex_t rx;
	int err;

	tests_run++;

	err = minrx_regcomp(&rx, pattern, flags | MINRX_REG_EXTENDED);

	if (err == (int) expected_error)
	{
		printf("PASS: %s\n", name);
		tests_passed++;
	}
	else
	{
		printf("FAIL: %s - expected error %d, got %d\n", name,
		       (int) expected_error, err);
		tests_failed++;
	}

	if (err == 0)
		minrx_regfree(&rx);
}

static void
test_basic_matching(void)
{
	printf("\n=== Basic Character Matching ===\n");

	test_match("literal char match", "a", "a", 1, 0);
	test_match("literal char no match", "a", "b", 0, 0);
	test_match("literal string match", "abc", "abc", 1, 0);
	test_match("literal string in middle", "abc", "xabcy", 1, 0);
	test_match("literal string no match", "abc", "abd", 0, 0);
	test_match("empty pattern", "", "anything", 1, 0);
	test_match("empty text", "a", "", 0, 0);
	test_match("empty both", "", "", 1, 0);
}

static void
test_character_classes(void)
{
	printf("\n=== Character Classes ===\n");

	test_match("dot matches char", ".", "a", 1, 0);
	test_match("dot matches any", "a.c", "abc", 1, 0);
	test_match("dot matches digit", ".", "5", 1, 0);
	test_match("dot doesn't match newline (with REG_NEWLINE)",
		   ".", "\n", 0, MINRX_REG_NEWLINE);
	test_match("dot matches newline (without REG_NEWLINE)",
		   ".", "\n", 1, 0);

	test_match("bracket single char", "[a]", "a", 1, 0);
	test_match("bracket multiple chars", "[abc]", "b", 1, 0);
	test_match("bracket range", "[a-z]", "m", 1, 0);
	test_match("bracket range no match", "[a-z]", "5", 0, 0);
	test_match("bracket negation", "[^a-z]", "5", 1, 0);
	test_match("bracket negation no match", "[^a-z]", "m", 0, 0);
	test_match("bracket with dash at end", "[a-]", "-", 1, 0);
	test_match("bracket with dash at start", "[-a]", "-", 1, 0);

	test_match("bracket [:alnum:]", "[[:alnum:]]", "a", 1, 0);
	test_match("bracket [:alnum:] digit", "[[:alnum:]]", "5", 1, 0);
	test_match("bracket [:alpha:]", "[[:alpha:]]", "a", 1, 0);
	test_match("bracket [:alpha:] no match", "[[:alpha:]]", "5", 0, 0);
	test_match("bracket [:digit:]", "[[:digit:]]", "5", 1, 0);
	test_match("bracket [:digit:] no match", "[[:digit:]]", "a", 0, 0);
	test_match("bracket [:lower:]", "[[:lower:]]", "a", 1, 0);
	test_match("bracket [:upper:]", "[[:upper:]]", "A", 1, 0);
	test_match("bracket [:space:]", "[[:space:]]", " ", 1, 0);
	test_match("bracket [:space:] tab", "[[:space:]]", "\t", 1, 0);
}

static void
test_alternation(void)
{
	printf("\n=== Alternation ===\n");

	test_match("simple alternation left", "a|b", "a", 1, 0);
	test_match("simple alternation right", "a|b", "b", 1, 0);
	test_match("simple alternation no match", "a|b", "c", 0, 0);
	test_match("multi alternation", "a|b|c", "b", 1, 0);
	test_match("alternation with concat", "ab|cd", "ab", 1, 0);
	test_match("alternation with concat right", "ab|cd", "cd", 1, 0);
	test_match("nested alternation", "(a|b)c", "ac", 1, 0);
	test_match("nested alternation right", "(a|b)c", "bc", 1, 0);
	test_match("empty alternation left", "|a", "a", 1, 0);
	test_match("empty alternation right", "a|", "a", 1, 0);
}

static void
test_grouping(void)
{
	printf("\n=== Grouping and Subexpressions ===\n");

	test_match("simple group", "(abc)", "abc", 1, 0);
	test_match("nested groups", "((a))", "a", 1, 0);
	test_match("group with alternation", "(a|b)", "a", 1, 0);
	test_match("multiple groups", "(a)(b)", "ab", 1, 0);

	// Test subexpression capture
	int offsets1[] = { 0, 3, 0, 1, 1, 2, 2, 3 };
	test_submatch("submatch three groups", "(a)(b)(c)", "abc", 4,
		      offsets1, 0);

	int offsets2[] = { 0, 2, 0, 1 };
	test_submatch("submatch nested", "((a))b", "ab", 2, offsets2, 0);

	int offsets3[] = { 0, 3, 1, 2 };
	test_submatch("submatch in middle", "x(a)y", "xay", 2, offsets3, 0);
}

static void
test_repetition(void)
{
	printf("\n=== Repetition Operators ===\n");

	// Star (*)
	test_match("star zero times", "a*", "", 1, 0);
	test_match("star once", "a*", "a", 1, 0);
	test_match("star multiple", "a*", "aaa", 1, 0);
	test_match("star with prefix", "ba*", "b", 1, 0);
	test_match("star with prefix multiple", "ba*", "baaa", 1, 0);

	// Plus (+)
	test_match("plus zero times", "a+", "", 0, 0);
	test_match("plus once", "a+", "a", 1, 0);
	test_match("plus multiple", "a+", "aaa", 1, 0);
	test_match("plus with prefix", "ba+", "ba", 1, 0);
	test_match("plus with prefix multiple", "ba+", "baaa", 1, 0);

	// Question (?)
	test_match("question zero times", "a?", "", 1, 0);
	test_match("question once", "a?", "a", 1, 0);
	test_match("question with prefix none", "ba?", "b", 1, 0);
	test_match("question with prefix one", "ba?", "ba", 1, 0);

	// Braces {m,n}
	test_match("brace exact {3}", "a{3}", "aaa", 1, 0);
	test_match("brace exact {3} too few", "a{3}", "aa", 0, 0);
	test_match("brace exact {3} too many", "a{3}", "aaaa", 1, 0);
	test_match("brace range {2,4} min", "a{2,4}", "aa", 1, 0);
	test_match("brace range {2,4} mid", "a{2,4}", "aaa", 1, 0);
	test_match("brace range {2,4} max", "a{2,4}", "aaaa", 1, 0);
	test_match("brace range {2,4} too few", "a{2,4}", "a", 0, 0);
	test_match("brace min {2,}", "a{2,}", "aaaaaa", 1, 0);
	test_match("brace min {2,} exact", "a{2,}", "aa", 1, 0);
	test_match("brace min {2,} too few", "a{2,}", "a", 0, 0);

	// Greedy vs non-greedy matching
	test_match("greedy star", "a*a", "aaa", 1, 0);
	test_match("greedy plus", "a+a", "aaa", 1, 0);
	test_match("greedy question", "a?a", "aa", 1, 0);
}

static void
test_anchors(void)
{
	printf("\n=== Anchors ===\n");

	// Beginning of line (^)
	test_match("anchor BOL match", "^abc", "abc", 1, 0);
	test_match("anchor BOL no match", "^abc", "xabc", 0, 0);
	test_match("anchor BOL in middle", "x^a", "x^a", 1, 0);	// ^ literal in middle

	// End of line ($)
	test_match("anchor EOL match", "abc$", "abc", 1, 0);
	test_match("anchor EOL no match", "abc$", "abcx", 0, 0);
	test_match("anchor EOL in middle", "a$x", "a$x", 1, 0);	// $ literal in middle

	// Both anchors
	test_match("anchor BOL+EOL exact", "^abc$", "abc", 1, 0);
	test_match("anchor BOL+EOL too long", "^abc$", "abcd", 0, 0);
	test_match("anchor BOL+EOL prefix", "^abc$", "xabc", 0, 0);

	// With REG_NEWLINE
	test_match("anchor BOL with newline", "^b", "a\nb", 1,
		   MINRX_REG_NEWLINE);
	test_match("anchor EOL with newline", "a$", "a\nb", 1,
		   MINRX_REG_NEWLINE);

	// Empty string anchors
	test_match("anchor empty BOL", "^$", "", 1, 0);
	test_match("anchor empty BOL no match", "^$", "a", 0, 0);
}

static void
test_backslash_escapes(void)
{
	printf("\n=== Backslash Escapes ===\n");

	// Meta characters
	test_match("escape dot", "\\.", ".", 1, 0);
	test_match("escape star", "\\*", "*", 1, 0);
	test_match("escape plus", "\\+", "+", 1, 0);
	test_match("escape question", "\\?", "?", 1, 0);
	test_match("escape pipe", "\\|", "|", 1, 0);
	test_match("escape open paren", "\\(", "(", 1, 0);
	test_match("escape close paren", "\\)", ")", 1, 0);
	test_match("escape open bracket", "\\[", "[", 1, 0);
	test_match("escape caret", "\\^", "^", 1, 0);
	test_match("escape dollar", "\\$", "$", 1, 0);

	// Backslash itself
	test_match("escape backslash", "\\\\", "\\", 1, 0);
}

static void
test_extensions_bsd(void)
{
	printf("\n=== BSD Extensions ===\n");

	test_match("word boundary begin \\<", "\\<word", "a word here", 1,
		   MINRX_REG_EXTENSIONS_BSD);
	test_match("word boundary begin \\< no match", "\\<ord", "word", 0,
		   MINRX_REG_EXTENSIONS_BSD);
	test_match("word boundary end \\>", "word\\>", "word here", 1,
		   MINRX_REG_EXTENSIONS_BSD);
	test_match("word boundary end \\> no match", "wor\\>", "word", 0,
		   MINRX_REG_EXTENSIONS_BSD);
	test_match("word boundaries both", "\\<word\\>", "a word here", 1,
		   MINRX_REG_EXTENSIONS_BSD);
	test_match("word boundaries both no match", "\\<word\\>", "words", 0,
		   MINRX_REG_EXTENSIONS_BSD);
}

static void
test_extensions_gnu(void)
{
	printf("\n=== GNU Extensions ===\n");

	test_match("GNU \\b word boundary", "\\bword\\b", "a word here", 1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match("GNU \\b no match", "\\bword\\b", "words", 0,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match("GNU \\B not boundary", "\\Bor\\B", "word", 1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match("GNU \\B not boundary no match", "\\Bwo", "word", 0,
		   MINRX_REG_EXTENSIONS_GNU);

	test_match("GNU \\s space", "\\s", " ", 1, MINRX_REG_EXTENSIONS_GNU);
	test_match("GNU \\s tab", "\\s", "\t", 1, MINRX_REG_EXTENSIONS_GNU);
	test_match("GNU \\s no match", "\\s", "a", 0,
		   MINRX_REG_EXTENSIONS_GNU);

	test_match("GNU \\S non-space", "\\S", "a", 1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match("GNU \\S no match", "\\S", " ", 0,
		   MINRX_REG_EXTENSIONS_GNU);

	test_match("GNU \\w word char", "\\w", "a", 1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match("GNU \\w digit", "\\w", "5", 1, MINRX_REG_EXTENSIONS_GNU);
	test_match("GNU \\w underscore", "\\w", "_", 1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match("GNU \\w no match", "\\w", " ", 0,
		   MINRX_REG_EXTENSIONS_GNU);

	test_match("GNU \\W non-word", "\\W", " ", 1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match("GNU \\W no match", "\\W", "a", 0,
		   MINRX_REG_EXTENSIONS_GNU);

	test_match("GNU \\` buffer start", "\\`abc", "abc", 1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match("GNU \\` no match", "\\`bc", "abc", 0,
		   MINRX_REG_EXTENSIONS_GNU);

	test_match("GNU \\' buffer end", "abc\\'", "abc", 1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match("GNU \\' no match", "ab\\'", "abc", 0,
		   MINRX_REG_EXTENSIONS_GNU);
}

static void
test_case_insensitive(void)
{
	printf("\n=== Case Insensitive Matching ===\n");

	test_match("icase simple lower", "abc", "ABC", 1, MINRX_REG_ICASE);
	test_match("icase simple upper", "ABC", "abc", 1, MINRX_REG_ICASE);
	test_match("icase mixed", "AbC", "aBc", 1, MINRX_REG_ICASE);
	test_match("icase in bracket", "[a-z]", "M", 1, MINRX_REG_ICASE);
	test_match("icase bracket upper", "[A-Z]", "m", 1, MINRX_REG_ICASE);
	test_match("icase with class", "[[:lower:]]", "M", 1, MINRX_REG_ICASE);
}

static void
test_complex_patterns(void)
{
	printf("\n=== Complex Patterns ===\n");

	// Email-like pattern
	test_match("email pattern", "[a-z]+@[a-z]+\\.[a-z]+",
		   "user@example.com", 1, 0);

	// Phone-like pattern
	test_match("phone pattern", "[0-9]{3}-[0-9]{4}", "555-1234", 1, 0);

	// URL-like pattern
	test_match("url pattern", "(http|https)://[a-z]+\\.[a-z]+",
		   "http://example.com", 1, 0);

	// Nested groups with repetition
	test_match("nested repeat", "(a(b*))+", "ababbba", 1, 0);

	// Alternation with repetition
	test_match("alt with repeat", "(ab|cd)+", "ababcd", 1, 0);

	// Complex bracket expression
	test_match("complex bracket", "[a-zA-Z0-9_-]+", "Test_123-foo", 1, 0);

	// Multiple anchors and groups
	test_match("complex anchored", "^([a-z]+):([0-9]+)$", "user:1234", 1,
		   0);
}

static void
test_edge_cases(void)
{
	printf("\n=== Edge Cases ===\n");

	// Very long repetition
	test_match("long repetition", "a{50}", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", 1, 0);

	// Deep nesting
	test_match("deep nesting", "((((a))))", "a", 1, 0);

	// Many alternations
	test_match("many alternations", "a|b|c|d|e|f|g|h|i|j", "f", 1, 0);

	// Empty subexpressions
	test_match("empty subexpr", "()", "", 1, 0);
	test_match("empty subexpr in concat", "a()b", "ab", 1, 0);

	// Special characters in brackets
	test_match("bracket with ]", "[]]", "]", 1, 0);
	test_match("bracket with ^", "[^]", "^", 1, 0);

	// Multiple dots
	test_match("multiple dots", "...", "abc", 1, 0);

	// Zero-width matches
	test_match("zero-width star", "a*", "b", 1, 0);
	test_match("zero-width question", "a?", "b", 1, 0);
}

static void
test_error_cases(void)
{
	printf("\n=== Error Handling ===\n");

	test_compile_error("error unbalanced (", "(abc", MINRX_REG_EPAREN, 0);
	test_compile_error("error unbalanced )", "abc)", MINRX_REG_EPAREN, 0);
	test_compile_error("error unbalanced [", "[abc", MINRX_REG_EBRACK, 0);
	test_compile_error("error unbalanced {", "a{2", MINRX_REG_EBRACE, 0);
	test_compile_error("error bad repetition", "*", MINRX_REG_BADRPT, 0);
	test_compile_error("error bad repetition +", "+", MINRX_REG_BADRPT, 0);
	test_compile_error("error bad repetition ?", "?", MINRX_REG_BADRPT, 0);
	test_compile_error("error trailing backslash", "abc\\",
			   MINRX_REG_EESCAPE, 0);
	test_compile_error("error invalid brace", "a{,}", MINRX_REG_BADBR, 0);
	test_compile_error("error invalid range", "[z-a]", MINRX_REG_ERANGE,
			   0);
}

int
main(void)
{
	setlocale(LC_ALL, "");

	printf("MinRX Test Suite\n");
	printf("================\n");

	test_basic_matching();
	test_character_classes();
	test_alternation();
	test_grouping();
	test_repetition();
	test_anchors();
	test_backslash_escapes();
	test_extensions_bsd();
	test_extensions_gnu();
	test_case_insensitive();
	test_complex_patterns();
	test_edge_cases();
	test_error_cases();

	printf("\n=== Summary ===\n");
	printf("Total tests: %d\n", tests_run);
	printf("Passed: %d\n", tests_passed);
	printf("Failed: %d\n", tests_failed);

	return tests_failed == 0 ? EXIT_SUCCESS : EXIT_FAILURE;
}
