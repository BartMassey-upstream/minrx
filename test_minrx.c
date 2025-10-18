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

// Test helper function - match/no match only
static void
test_match(const char *name, const char *pattern, const char *text,
	   int should_match, int flags)
{
	minrx_regex_t rx = {0};
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

// Test helper function with position verification
static void
test_match_pos(const char *name, const char *pattern, const char *text,
	       int expected_start, int expected_end, int flags)
{
	minrx_regex_t rx = {0};
	minrx_regmatch_t rm[1];
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

	exec_result = minrx_regexec(&rx, text, 1, rm, 0);
	minrx_regfree(&rx);

	if (expected_start == -1 && expected_end == -1)
	{
		// Expecting no match
		if (exec_result != 0)
		{
			printf("PASS: %s\n", name);
			tests_passed++;
		}
		else
		{
			printf("FAIL: %s - expected no match, got match at (%d,%d)\n",
			       name, (int)rm[0].rm_so, (int)rm[0].rm_eo);
			tests_failed++;
		}
	}
	else
	{
		// Expecting a match at specific position
		if (exec_result != 0)
		{
			printf("FAIL: %s - expected match at (%d,%d), got no match\n",
			       name, expected_start, expected_end);
			tests_failed++;
		}
		else if (rm[0].rm_so != expected_start || rm[0].rm_eo != expected_end)
		{
			printf("FAIL: %s - expected match at (%d,%d), got (%d,%d)\n",
			       name, expected_start, expected_end,
			       (int)rm[0].rm_so, (int)rm[0].rm_eo);
			tests_failed++;
		}
		else
		{
			printf("PASS: %s\n", name);
			tests_passed++;
		}
	}
}

// Test helper for subexpression capture
static void
test_submatch(const char *name, const char *pattern, const char *text,
	      int nsubs, const int *expected_offsets, int flags)
{
	minrx_regex_t rx = {0};
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
	minrx_regex_t rx = {0};
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

	test_match_pos("literal char match", "a", "a", 0, 1, 0);
	test_match_pos("literal char no match", "a", "b", -1, -1, 0);
	test_match_pos("literal string match", "abc", "abc", 0, 3, 0);
	test_match_pos("literal string in middle", "abc", "xabcy", 1, 4, 0);
	test_match_pos("literal string no match", "abc", "abd", -1, -1, 0);
	test_match_pos("empty pattern", "", "anything", 0, 0, 0);
	test_match_pos("empty text", "a", "", -1, -1, 0);
	test_match_pos("empty both", "", "", 0, 0, 0);
}

static void
test_character_classes(void)
{
	printf("\n=== Character Classes ===\n");

	test_match_pos("dot matches char", ".", "a", 0, 1, 0);
	test_match_pos("dot matches any", "a.c", "abc", 0, 3, 0);
	test_match_pos("dot matches digit", ".", "5", 0, 1, 0);
	test_match_pos("dot doesn't match newline (with REG_NEWLINE)",
		   ".", "\n", -1, -1, MINRX_REG_NEWLINE);
	test_match_pos("dot matches newline (without REG_NEWLINE)",
		   ".", "\n", 0, 1, 0);

	test_match_pos("bracket single char", "[a]", "a", 0, 1, 0);
	test_match_pos("bracket multiple chars", "[abc]", "b", 0, 1, 0);
	test_match_pos("bracket range", "[a-z]", "m", 0, 1, 0);
	test_match_pos("bracket range no match", "[a-z]", "5", -1, -1, 0);
	test_match_pos("bracket negation", "[^a-z]", "5", 0, 1, 0);
	test_match_pos("bracket negation no match", "[^a-z]", "m", -1, -1, 0);
	test_match_pos("bracket with dash at end", "[a-]", "-", 0, 1, 0);
	test_match_pos("bracket with dash at start", "[-a]", "-", 0, 1, 0);

	test_match_pos("bracket [:alnum:]", "[[:alnum:]]", "a", 0, 1, 0);
	test_match_pos("bracket [:alnum:] digit", "[[:alnum:]]", "5", 0, 1, 0);
	test_match_pos("bracket [:alpha:]", "[[:alpha:]]", "a", 0, 1, 0);
	test_match_pos("bracket [:alpha:] no match", "[[:alpha:]]", "5", -1, -1, 0);
	test_match_pos("bracket [:digit:]", "[[:digit:]]", "5", 0, 1, 0);
	test_match_pos("bracket [:digit:] no match", "[[:digit:]]", "a", -1, -1, 0);
	test_match_pos("bracket [:lower:]", "[[:lower:]]", "a", 0, 1, 0);
	test_match_pos("bracket [:upper:]", "[[:upper:]]", "A", 0, 1, 0);
	test_match_pos("bracket [:space:]", "[[:space:]]", " ", 0, 1, 0);
	test_match_pos("bracket [:space:] tab", "[[:space:]]", "\t", 0, 1, 0);
}

static void
test_alternation(void)
{
	printf("\n=== Alternation ===\n");

	test_match_pos("simple alternation left", "a|b", "a", 0, 1, 0);
	test_match_pos("simple alternation right", "a|b", "b", 0, 1, 0);
	test_match_pos("simple alternation no match", "a|b", "c", -1, -1, 0);
	test_match_pos("multi alternation", "a|b|c", "b", 0, 1, 0);
	test_match_pos("alternation with concat", "ab|cd", "ab", 0, 2, 0);
	test_match_pos("alternation with concat right", "ab|cd", "cd", 0, 2, 0);
	test_match_pos("nested alternation", "(a|b)c", "ac", 0, 2, 0);
	test_match_pos("nested alternation right", "(a|b)c", "bc", 0, 2, 0);
	test_match_pos("empty alternation left", "|a", "a", 0, 1, 0);
	test_match_pos("empty alternation right", "a|", "a", 0, 1, 0);
}

static void
test_grouping(void)
{
	printf("\n=== Grouping and Subexpressions ===\n");

	test_match_pos("simple group", "(abc)", "abc", 0, 3, 0);
	test_match_pos("nested groups", "((a))", "a", 0, 1, 0);
	test_match_pos("group with alternation", "(a|b)", "a", 0, 1, 0);
	test_match_pos("multiple groups", "(a)(b)", "ab", 0, 2, 0);

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
	test_match_pos("star zero times", "a*", "", 0, 0, 0);
	test_match_pos("star once", "a*", "a", 0, 1, 0);
	test_match_pos("star multiple", "a*", "aaa", 0, 3, 0);
	test_match_pos("star with prefix", "ba*", "b", 0, 1, 0);
	test_match_pos("star with prefix multiple", "ba*", "baaa", 0, 4, 0);

	// Plus (+)
	test_match_pos("plus zero times", "a+", "", -1, -1, 0);
	test_match_pos("plus once", "a+", "a", 0, 1, 0);
	test_match_pos("plus multiple", "a+", "aaa", 0, 3, 0);
	test_match_pos("plus with prefix", "ba+", "ba", 0, 2, 0);
	test_match_pos("plus with prefix multiple", "ba+", "baaa", 0, 4, 0);

	// Question (?)
	test_match_pos("question zero times", "a?", "", 0, 0, 0);
	test_match_pos("question once", "a?", "a", 0, 1, 0);
	test_match_pos("question with prefix none", "ba?", "b", 0, 1, 0);
	test_match_pos("question with prefix one", "ba?", "ba", 0, 2, 0);

	// Braces {m,n}
	test_match_pos("brace exact {3}", "a{3}", "aaa", 0, 3, 0);
	test_match_pos("brace exact {3} too few", "a{3}", "aa", -1, -1, 0);
	test_match_pos("brace exact {3} too many", "a{3}", "aaaa", 0, 3, 0);
	test_match_pos("brace range {2,4} min", "a{2,4}", "aa", 0, 2, 0);
	test_match_pos("brace range {2,4} mid", "a{2,4}", "aaa", 0, 3, 0);
	test_match_pos("brace range {2,4} max", "a{2,4}", "aaaa", 0, 4, 0);
	test_match_pos("brace range {2,4} too few", "a{2,4}", "a", -1, -1, 0);
	test_match_pos("brace min {2,}", "a{2,}", "aaaaaa", 0, 6, 0);
	test_match_pos("brace min {2,} exact", "a{2,}", "aa", 0, 2, 0);
	test_match_pos("brace min {2,} too few", "a{2,}", "a", -1, -1, 0);

	// Greedy vs non-greedy matching
	test_match_pos("greedy star", "a*a", "aaa", 0, 3, 0);
	test_match_pos("greedy plus", "a+a", "aaa", 0, 3, 0);
	test_match_pos("greedy question", "a?a", "aa", 0, 2, 0);
}

static void
test_anchors(void)
{
	printf("\n=== Anchors ===\n");

	// Beginning of line (^)
	test_match_pos("anchor BOL match", "^abc", "abc", 0, 3, 0);
	test_match_pos("anchor BOL no match", "^abc", "xabc", -1, -1, 0);
	test_match_pos("anchor BOL in middle", "x\\^a", "x^a", 0, 3, 0);	// escaped ^ literal

	// End of line ($)
	test_match_pos("anchor EOL match", "abc$", "abc", 0, 3, 0);
	test_match_pos("anchor EOL no match", "abc$", "abcx", -1, -1, 0);
	test_match_pos("anchor EOL in middle", "a\\$x", "a$x", 0, 3, 0);	// escaped $ literal

	// Both anchors
	test_match_pos("anchor BOL+EOL exact", "^abc$", "abc", 0, 3, 0);
	test_match_pos("anchor BOL+EOL too long", "^abc$", "abcd", -1, -1, 0);
	test_match_pos("anchor BOL+EOL prefix", "^abc$", "xabc", -1, -1, 0);

	// With REG_NEWLINE
	test_match_pos("anchor BOL with newline", "^b", "a\nb", 2, 3,
		   MINRX_REG_NEWLINE);
	test_match_pos("anchor EOL with newline", "a$", "a\nb", 0, 1,
		   MINRX_REG_NEWLINE);

	// Empty string anchors
	test_match_pos("anchor empty BOL", "^$", "", 0, 0, 0);
	test_match_pos("anchor empty BOL no match", "^$", "a", -1, -1, 0);
}

static void
test_backslash_escapes(void)
{
	printf("\n=== Backslash Escapes ===\n");

	// Meta characters
	test_match_pos("escape dot", "\\.", ".", 0, 1, 0);
	test_match_pos("escape star", "\\*", "*", 0, 1, 0);
	test_match_pos("escape plus", "\\+", "+", 0, 1, 0);
	test_match_pos("escape question", "\\?", "?", 0, 1, 0);
	test_match_pos("escape pipe", "\\|", "|", 0, 1, 0);
	test_match_pos("escape open paren", "\\(", "(", 0, 1, 0);
	test_match_pos("escape close paren", "\\)", ")", 0, 1, 0);
	test_match_pos("escape open bracket", "\\[", "[", 0, 1, 0);
	test_match_pos("escape caret", "\\^", "^", 0, 1, 0);
	test_match_pos("escape dollar", "\\$", "$", 0, 1, 0);

	// Backslash itself
	test_match_pos("escape backslash", "\\\\", "\\", 0, 1, 0);
}

static void
test_extensions_bsd(void)
{
	printf("\n=== BSD Extensions ===\n");

	test_match_pos("word boundary begin \\<", "\\<word", "a word here", 2, 6,
		   MINRX_REG_EXTENSIONS_BSD);
	test_match_pos("word boundary begin \\< no match", "\\<ord", "word", -1, -1,
		   MINRX_REG_EXTENSIONS_BSD);
	test_match_pos("word boundary end \\>", "word\\>", "word here", 0, 4,
		   MINRX_REG_EXTENSIONS_BSD);
	test_match_pos("word boundary end \\> no match", "wor\\>", "word", -1, -1,
		   MINRX_REG_EXTENSIONS_BSD);
	test_match_pos("word boundaries both", "\\<word\\>", "a word here", 2, 6,
		   MINRX_REG_EXTENSIONS_BSD);
	test_match_pos("word boundaries both no match", "\\<word\\>", "words", -1, -1,
		   MINRX_REG_EXTENSIONS_BSD);
}

static void
test_extensions_gnu(void)
{
	printf("\n=== GNU Extensions ===\n");

	test_match_pos("GNU \\b word boundary", "\\bword\\b", "a word here", 2, 6,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match_pos("GNU \\b no match", "\\bword\\b", "words", -1, -1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match_pos("GNU \\B not boundary", "\\Bor\\B", "word", 1, 3,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match_pos("GNU \\B not boundary no match", "\\Bwo", "word", -1, -1,
		   MINRX_REG_EXTENSIONS_GNU);

	test_match_pos("GNU \\s space", "\\s", " ", 0, 1, MINRX_REG_EXTENSIONS_GNU);
	test_match_pos("GNU \\s tab", "\\s", "\t", 0, 1, MINRX_REG_EXTENSIONS_GNU);
	test_match_pos("GNU \\s no match", "\\s", "a", -1, -1,
		   MINRX_REG_EXTENSIONS_GNU);

	test_match_pos("GNU \\S non-space", "\\S", "a", 0, 1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match_pos("GNU \\S no match", "\\S", " ", -1, -1,
		   MINRX_REG_EXTENSIONS_GNU);

	test_match_pos("GNU \\w word char", "\\w", "a", 0, 1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match_pos("GNU \\w digit", "\\w", "5", 0, 1, MINRX_REG_EXTENSIONS_GNU);
	test_match_pos("GNU \\w underscore", "\\w", "_", 0, 1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match_pos("GNU \\w no match", "\\w", " ", -1, -1,
		   MINRX_REG_EXTENSIONS_GNU);

	test_match_pos("GNU \\W non-word", "\\W", " ", 0, 1,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match_pos("GNU \\W no match", "\\W", "a", -1, -1,
		   MINRX_REG_EXTENSIONS_GNU);

	test_match_pos("GNU \\` buffer start", "\\`abc", "abc", 0, 3,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match_pos("GNU \\` no match", "\\`bc", "abc", -1, -1,
		   MINRX_REG_EXTENSIONS_GNU);

	test_match_pos("GNU \\' buffer end", "abc\\'", "abc", 0, 3,
		   MINRX_REG_EXTENSIONS_GNU);
	test_match_pos("GNU \\' no match", "ab\\'", "abc", -1, -1,
		   MINRX_REG_EXTENSIONS_GNU);
}

static void
test_case_insensitive(void)
{
	printf("\n=== Case Insensitive Matching ===\n");

	test_match_pos("icase simple lower", "abc", "ABC", 0, 3, MINRX_REG_ICASE);
	test_match_pos("icase simple upper", "ABC", "abc", 0, 3, MINRX_REG_ICASE);
	test_match_pos("icase mixed", "AbC", "aBc", 0, 3, MINRX_REG_ICASE);
	test_match_pos("icase in bracket", "[a-z]", "M", 0, 1, MINRX_REG_ICASE);
	test_match_pos("icase bracket upper", "[A-Z]", "m", 0, 1, MINRX_REG_ICASE);
	test_match_pos("icase with class", "[[:lower:]]", "M", 0, 1, MINRX_REG_ICASE);
}

static void
test_complex_patterns(void)
{
	printf("\n=== Complex Patterns ===\n");

	// Email-like pattern
	test_match_pos("email pattern", "[a-z]+@[a-z]+\\.[a-z]+",
		   "user@example.com", 0, 16, 0);

	// Phone-like pattern
	test_match_pos("phone pattern", "[0-9]{3}-[0-9]{4}", "555-1234", 0, 8, 0);

	// URL-like pattern
	test_match_pos("url pattern", "(http|https)://[a-z]+\\.[a-z]+",
		   "http://example.com", 0, 18, 0);

	// Nested groups with repetition
	test_match_pos("nested repeat", "(a(b*))+", "ababbba", 0, 7, 0);

	// Alternation with repetition
	test_match_pos("alt with repeat", "(ab|cd)+", "ababcd", 0, 6, 0);

	// Complex bracket expression
	test_match_pos("complex bracket", "[a-zA-Z0-9_-]+", "Test_123-foo", 0, 12, 0);

	// Multiple anchors and groups
	test_match_pos("complex anchored", "^([a-z]+):([0-9]+)$", "user:1234", 0,
		   9, 0);
}

static void
test_edge_cases(void)
{
	printf("\n=== Edge Cases ===\n");

	// Very long repetition
	test_match_pos("long repetition", "a{50}", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", 0, 50, 0);

	// Deep nesting
	test_match_pos("deep nesting", "((((a))))", "a", 0, 1, 0);

	// Many alternations
	test_match_pos("many alternations", "a|b|c|d|e|f|g|h|i|j", "f", 0, 1, 0);

	// Empty subexpressions
	test_match_pos("empty subexpr", "()", "", 0, 0, 0);
	test_match_pos("empty subexpr in concat", "a()b", "ab", 0, 2, 0);

	// Special characters in brackets
	test_match_pos("bracket with ]", "[]]", "]", 0, 1, 0);
	test_match_pos("bracket with ^", "[\\^]", "^", 0, 1, 0);
	test_match_pos("bracket negated with ]", "[^]]", "a", 0, 1, 0);  // matches anything except ]
	test_match_pos("bracket negated with ] no match", "[^]]", "]", -1, -1, 0);

	// Multiple dots
	test_match_pos("multiple dots", "...", "abc", 0, 3, 0);

	// Zero-width matches
	test_match_pos("zero-width star", "a*", "b", 0, 0, 0);
	test_match_pos("zero-width question", "a?", "b", 0, 0, 0);
}

static void
test_error_cases(void)
{
	printf("\n=== Error Handling ===\n");

	test_compile_error("error unbalanced (", "(abc", MINRX_REG_EPAREN, 0);
	test_match_pos("unbalanced ) treated as literal - match", "abc)", "abc)", 0, 4, 0);
	test_match_pos("unbalanced ) treated as literal - no match", "abc)", "abc", -1, -1, 0);
	test_compile_error("error unbalanced [", "[abc", MINRX_REG_EBRACK, 0);
	test_compile_error("error invalid [^]", "[^]", MINRX_REG_EBRACK, 0);
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

static void
test_uninitialized_regex(void)
{
	minrx_regex_t rx1, rx2;
	minrx_regmatch_t rm[10];

	printf("\n=== Uninitialized Regex ===\n");

	// Test 1: Uninitialized regex_t (like user's code might have)
	tests_run++;
	if (minrx_regcomp(&rx1, "hello", MINRX_REG_EXTENDED) == 0) {
		if (minrx_regexec(&rx1, "hello world", 10, rm, 0) == 0) {
			if (rm[0].rm_so == 0 && rm[0].rm_eo == 5) {
				printf("PASS: uninitialized regex_t\n");
				tests_passed++;
			} else {
				printf("FAIL: uninitialized regex_t - wrong match offsets\n");
				tests_failed++;
			}
		} else {
			printf("FAIL: uninitialized regex_t - no match found\n");
			tests_failed++;
		}
		minrx_regfree(&rx1);
	} else {
		printf("FAIL: uninitialized regex_t - compilation failed\n");
		tests_failed++;
	}

	// Test 2: Initialized regex_t (explicit zero initialization)
	rx2 = (minrx_regex_t){0};
	tests_run++;
	if (minrx_regcomp(&rx2, "world", MINRX_REG_EXTENDED) == 0) {
		if (minrx_regexec(&rx2, "hello world", 10, rm, 0) == 0) {
			if (rm[0].rm_so == 6 && rm[0].rm_eo == 11) {
				printf("PASS: initialized regex_t\n");
				tests_passed++;
			} else {
				printf("FAIL: initialized regex_t - wrong match offsets\n");
				tests_failed++;
			}
		} else {
			printf("FAIL: initialized regex_t - no match found\n");
			tests_failed++;
		}
		minrx_regfree(&rx2);
	} else {
		printf("FAIL: initialized regex_t - compilation failed\n");
		tests_failed++;
	}
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
	test_uninitialized_regex();

	printf("\n=== Summary ===\n");
	printf("Total tests: %d\n", tests_run);
	printf("Passed: %d\n", tests_passed);
	printf("Failed: %d\n", tests_failed);

	return tests_failed == 0 ? EXIT_SUCCESS : EXIT_FAILURE;
}
