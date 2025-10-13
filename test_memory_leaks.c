//
// Comprehensive memory leak test for minrx
// Tests various code paths including error cases
//

#include <locale.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "minrx.h"

static int test_count = 0;
static int leak_test_count = 0;

// Test successful compilation and execution
static void test_normal_path(void)
{
	minrx_regex_t rx;
	minrx_regmatch_t rm[10];

	test_count++;
	if (minrx_regcomp(&rx, "a+b*", MINRX_REG_EXTENDED) == 0) {
		minrx_regexec(&rx, "aaabbb", 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}
}

// Test compilation errors - these should not leak
static void test_compile_errors(void)
{
	minrx_regex_t rx;

	// Unbalanced parenthesis
	test_count++;
	if (minrx_regcomp(&rx, "(abc", MINRX_REG_EXTENDED) != 0) {
		minrx_regfree(&rx);  // Should be safe to call even on error
		leak_test_count++;
	}

	// Unbalanced bracket
	test_count++;
	if (minrx_regcomp(&rx, "[abc", MINRX_REG_EXTENDED) != 0) {
		minrx_regfree(&rx);
		leak_test_count++;
	}

	// Bad repetition
	test_count++;
	if (minrx_regcomp(&rx, "*abc", MINRX_REG_EXTENDED) != 0) {
		minrx_regfree(&rx);
		leak_test_count++;
	}

	// Unbalanced brace
	test_count++;
	if (minrx_regcomp(&rx, "a{2", MINRX_REG_EXTENDED) != 0) {
		minrx_regfree(&rx);
		leak_test_count++;
	}

	// Invalid range
	test_count++;
	if (minrx_regcomp(&rx, "[z-a]", MINRX_REG_EXTENDED) != 0) {
		minrx_regfree(&rx);
		leak_test_count++;
	}

	// Trailing backslash
	test_count++;
	if (minrx_regcomp(&rx, "abc\\", MINRX_REG_EXTENDED) != 0) {
		minrx_regfree(&rx);
		leak_test_count++;
	}
}

// Test no-match cases
static void test_no_match(void)
{
	minrx_regex_t rx;
	minrx_regmatch_t rm[10];

	test_count++;
	if (minrx_regcomp(&rx, "xyz", MINRX_REG_EXTENDED) == 0) {
		minrx_regexec(&rx, "abc", 10, rm, 0);  // No match
		minrx_regfree(&rx);
		leak_test_count++;
	}
}

// Test complex patterns
static void test_complex_patterns(void)
{
	minrx_regex_t rx;
	minrx_regmatch_t rm[10];

	// Nested groups
	test_count++;
	if (minrx_regcomp(&rx, "((a+)(b*))+", MINRX_REG_EXTENDED) == 0) {
		minrx_regexec(&rx, "aaabbbaaabbb", 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}

	// Alternation
	test_count++;
	if (minrx_regcomp(&rx, "(abc|def|ghi)+", MINRX_REG_EXTENDED) == 0) {
		minrx_regexec(&rx, "abcdefghi", 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}

	// Complex bracket expressions
	test_count++;
	if (minrx_regcomp(&rx, "[a-zA-Z0-9_]+@[a-z]+\\.[a-z]+", MINRX_REG_EXTENDED) == 0) {
		minrx_regexec(&rx, "test@example.com", 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}

	// Anchors
	test_count++;
	if (minrx_regcomp(&rx, "^abc$", MINRX_REG_EXTENDED) == 0) {
		minrx_regexec(&rx, "abc", 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}
}

// Test with extensions
static void test_with_extensions(void)
{
	minrx_regex_t rx;
	minrx_regmatch_t rm[10];
	int flags = MINRX_REG_EXTENDED | MINRX_REG_EXTENSIONS_BSD | MINRX_REG_EXTENSIONS_GNU;

	// BSD word boundaries
	test_count++;
	if (minrx_regcomp(&rx, "\\<word\\>", flags) == 0) {
		minrx_regexec(&rx, "a word here", 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}

	// GNU extensions
	test_count++;
	if (minrx_regcomp(&rx, "\\bword\\b", flags) == 0) {
		minrx_regexec(&rx, "a word here", 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}

	test_count++;
	if (minrx_regcomp(&rx, "\\w+\\s+\\w+", flags) == 0) {
		minrx_regexec(&rx, "hello world", 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}
}

// Test with various flags
static void test_with_flags(void)
{
	minrx_regex_t rx;
	minrx_regmatch_t rm[10];

	// Case insensitive
	test_count++;
	if (minrx_regcomp(&rx, "ABC", MINRX_REG_EXTENDED | MINRX_REG_ICASE) == 0) {
		minrx_regexec(&rx, "abc", 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}

	// NEWLINE flag
	test_count++;
	if (minrx_regcomp(&rx, ".", MINRX_REG_EXTENDED | MINRX_REG_NEWLINE) == 0) {
		minrx_regexec(&rx, "\n", 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}

	// NOSUB flag
	test_count++;
	if (minrx_regcomp(&rx, "(a+)(b+)", MINRX_REG_EXTENDED | MINRX_REG_NOSUB) == 0) {
		minrx_regexec(&rx, "aaabbb", 0, NULL, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}
}

// Test multiple compilations without freeing (should not be done, but test it)
static void test_recompile(void)
{
	minrx_regex_t rx;
	minrx_regmatch_t rm[10];

	// Compile, use, compile again without free (BAD PRACTICE but test it)
	test_count++;
	if (minrx_regcomp(&rx, "abc", MINRX_REG_EXTENDED) == 0) {
		minrx_regexec(&rx, "abc", 10, rm, 0);
		// Don't free here - deliberately test recompile
		// This should NOT leak if minrx is robust
		if (minrx_regcomp(&rx, "def", MINRX_REG_EXTENDED) == 0) {
			minrx_regexec(&rx, "def", 10, rm, 0);
			minrx_regfree(&rx);  // Only free once at end
		}
		leak_test_count++;
	}
}

// Test with empty patterns and strings
static void test_edge_cases(void)
{
	minrx_regex_t rx;
	minrx_regmatch_t rm[10];

	// Empty pattern
	test_count++;
	if (minrx_regcomp(&rx, "", MINRX_REG_EXTENDED) == 0) {
		minrx_regexec(&rx, "anything", 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}

	// Empty string
	test_count++;
	if (minrx_regcomp(&rx, "a*", MINRX_REG_EXTENDED) == 0) {
		minrx_regexec(&rx, "", 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}

	// Very long pattern
	test_count++;
	char long_pattern[1000];
	for (int i = 0; i < 999; i++) long_pattern[i] = 'a';
	long_pattern[999] = '\0';
	if (minrx_regcomp(&rx, long_pattern, MINRX_REG_EXTENDED) == 0) {
		minrx_regexec(&rx, long_pattern, 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}
}

// Test regncomp and regnexec
static void test_nfunctions(void)
{
	minrx_regex_t rx;
	minrx_regmatch_t rm[10];
	const char *pattern = "abc";
	const char *text = "xyzabcdef";

	test_count++;
	if (minrx_regncomp(&rx, strlen(pattern), pattern, MINRX_REG_EXTENDED) == 0) {
		minrx_regnexec(&rx, strlen(text), text, 10, rm, 0);
		minrx_regfree(&rx);
		leak_test_count++;
	}
}

// Test multiple regexes simultaneously
static void test_multiple_regexes(void)
{
	#define NUM_RX 10
	minrx_regex_t rxs[NUM_RX];
	minrx_regmatch_t rm[10];
	int i;

	// Compile multiple regexes
	for (i = 0; i < NUM_RX; i++) {
		test_count++;
		if (minrx_regcomp(&rxs[i], "test[0-9]+", MINRX_REG_EXTENDED) == 0) {
			leak_test_count++;
		}
	}

	// Use them
	for (i = 0; i < NUM_RX; i++) {
		minrx_regexec(&rxs[i], "test123", 10, rm, 0);
	}

	// Free them all
	for (i = 0; i < NUM_RX; i++) {
		minrx_regfree(&rxs[i]);
	}
}

// Test RESUME flag
static void test_resume_flag(void)
{
	minrx_regex_t rx;
	minrx_regmatch_t rm[10];

	test_count++;
	if (minrx_regcomp(&rx, "a+", MINRX_REG_EXTENDED) == 0) {
		if (minrx_regexec(&rx, "aaabbbaaacc", 10, rm, 0) == 0) {
			// Try to find next match using RESUME
			minrx_regexec(&rx, "aaabbbaaacc", 10, rm, MINRX_REG_RESUME);
		}
		minrx_regfree(&rx);
		leak_test_count++;
	}
}

int
main(void)
{
	setlocale(LC_ALL, "");

	printf("Running comprehensive memory leak tests...\n\n");

	test_normal_path();
	printf("Normal path tests: %d\n", leak_test_count);

	test_compile_errors();
	printf("Compile error tests: %d\n", leak_test_count);

	test_no_match();
	printf("No-match tests: %d\n", leak_test_count);

	test_complex_patterns();
	printf("Complex pattern tests: %d\n", leak_test_count);

	test_with_extensions();
	printf("Extension tests: %d\n", leak_test_count);

	test_with_flags();
	printf("Flag tests: %d\n", leak_test_count);

	test_recompile();
	printf("Recompile tests: %d\n", leak_test_count);

	test_edge_cases();
	printf("Edge case tests: %d\n", leak_test_count);

	test_nfunctions();
	printf("N-function tests: %d\n", leak_test_count);

	test_multiple_regexes();
	printf("Multiple regex tests: %d\n", leak_test_count);

	test_resume_flag();
	printf("Resume flag tests: %d\n", leak_test_count);

	printf("\nTotal tests: %d/%d completed\n", leak_test_count, test_count);
	printf("\nRun with valgrind to check for memory leaks:\n");
	printf("  valgrind --leak-check=full ./test_memory_leaks\n");

	return 0;
}
