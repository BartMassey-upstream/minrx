# MinRX Conversions
Claude Code with Bart Massey 2025

## Overview

This repository now contains **three implementations** of the MinRX library:

1. **C++ (original)**: `minrx.cpp` - The original C++20 implementation
2. **C (conversion)**: `minrx.c` - A faithful C99 conversion
3. **Rust (conversion)**: `src/` - A modern Rust implementation

All three implementations:
- Export the same C-compatible API defined in `minrx.h`
- Pass the same comprehensive test suite (163 tests)
- Implement the same POSIX ERE matching algorithm
- Have zero memory leaks on the test suite (verified with valgrind)

## Repository Structure

### Source Files

```
minrx/
├── minrx.h              # Common C API header for all implementations
├── minrx.cpp            # Original C++20 implementation
├── minrx.c              # C99 conversion
├── src/                 # Rust implementation
│   ├── lib.rs           # Main library module
│   ├── ffi.rs           # C FFI layer (exports minrx.h API)
│   ├── compile.rs       # Pattern compilation
│   ├── execute.rs       # NFA execution engine
│   ├── node.rs          # NFA node types
│   ├── cset.rs          # Character set operations
│   └── data_structures/ # Custom data structures
│       ├── cowvec.rs    # Copy-on-write vector
│       ├── qvec.rs      # Queue-based vector
│       └── qset.rs      # Queue-based set
├── test_minrx.c         # Comprehensive test suite (works with all)
├── rxgrep.c             # Example: minimal egrep implementation
├── tryit.c              # Example: simple pattern tester
└── Cargo.toml           # Rust package configuration
```

### Build System Files

- `Makefile` - Supports building all three versions
- `meson.build` - Meson build for C++ version only
- `Cargo.toml` - Cargo build for Rust version

## The C Conversion

The C conversion (`minrx.c`) was created to improve portability and
reduce dependencies. It maintains the same algorithm and API as the
C++ version while using only C99 features.

### Key Differences from C++

- Manual memory management instead of RAII
- Explicit cleanup functions instead of destructors
- C-style string handling instead of `std::string`
- Manual dynamic arrays instead of `std::vector`
- Function pointers instead of member functions

### Building the C Version

```bash
# Build C version of test suite
make test_minrx_c

# Build C version of rxgrep
make rxgrep_c

# Build C version of tryit
make tryit_c

# Build all C programs
make all-c
```

## The Rust Conversion

The Rust conversion provides a memory-safe, modern implementation
while maintaining full C API compatibility through FFI.

### Rust Implementation Highlights

**Memory Safety**: Rust's ownership system eliminates entire classes
of bugs:
- No dangling pointers
- No use-after-free
- No buffer overflows
- No data races

**Custom Data Structures**: The Rust version includes specialized
data structures optimized for the NFA algorithm:
- `CowVec<T>`: Copy-on-write vector for efficient cloning
- `QVec<T>`: Queue-based vector with O(1) append
- `QSet<T>`: Queue-based set with O(1) membership testing

**FFI Layer**: The `ffi.rs` module provides a complete C-compatible
API that wraps the safe Rust implementation. It includes:
- C-compatible struct layout (`#[repr(C)]`)
- Safe conversion between C and Rust types
- Proper handling of null pointers
- Memory leak prevention via `Drop` trait

### Building the Rust Version

```bash
# Build the Rust library
cargo build --release

# Build with Makefile (includes library + test suite)
make all-rust

# Run Rust unit tests
cargo test

# Run C test suite against Rust library
make test_minrx
./test_minrx
```

The Rust library is built as a cdylib (C-compatible dynamic library)
at `target/release/libminrx.so` (Linux) or `libminrx.dylib` (macOS).

### Using the Rust Library from C

The Rust version exports the same API as the C/C++ versions. Here's
a simple example:

```c
#include "minrx.h"
#include <stdio.h>

int main(void) {
    minrx_regex_t regex = {0};
    int ret;

    // Compile pattern
    ret = minrx_regcomp(&regex, "hello|world", MINRX_REG_EXTENDED);
    if (ret != 0) {
        fprintf(stderr, "Compilation failed\n");
        return 1;
    }

    // Execute match
    ret = minrx_regexec(&regex, "hello there", 0, NULL, 0);
    if (ret == 0) {
        printf("Match found!\n");
    }

    // Clean up
    minrx_regfree(&regex);
    return 0;
}
```

Compile against the Rust library:

```bash
gcc -o myprogram myprogram.c -L./target/release -lminrx \
    -Wl,-rpath,./target/release
```

### Using the Rust Library from Rust

The library provides a native Rust API that is fully documented with rustdoc.

#### Viewing the Documentation

To build and view the Rust API documentation:

```bash
cargo doc --no-deps --open
```

This will build the documentation and open it in your web browser.

#### Native Rust API Example

```rust
use minrx::{Regex, CompileFlags, ExecFlags};

fn main() -> Result<(), minrx::RegexError> {
    // Compile a regex
    let pattern = "hello|world";
    let regex = Regex::new(pattern, CompileFlags::EXTENDED)?;

    // Match against text
    let text = "hello there";
    let matches = regex.exec(text, ExecFlags::empty())?;

    // Extract matched substring
    let matched = &text[matches[0].start as usize..matches[0].end as usize];
    println!("Matched: {}", matched);

    Ok(())
}
```

The Rust API provides:
- Type-safe regex compilation and execution
- Error handling via `Result` types
- Zero-cost abstractions with no performance penalty
- Full documentation with examples

## Test Suite

The file `test_minrx.c` contains 163 comprehensive tests covering:

- Basic character matching
- Character classes and bracket expressions
- Alternation and grouping
- Repetition operators (*, +, ?, {m,n})
- Anchors (^, $)
- Backslash escapes
- BSD extensions (\<, \>)
- GNU extensions (\b, \B, \s, \S, \w, \W, \`, \')
- Case-insensitive matching
- Complex patterns
- Edge cases
- Error handling

All three implementations pass this entire test suite.

### Running Tests

```bash
# Test C++ version
make test_minrx_cpp
./test_minrx_cpp

# Test C version
make test_minrx_c
./test_minrx_c

# Test Rust version
make test_minrx
./test_minrx

# Or build and run in one step
make all-rust && ./test_minrx
```

### Verifying Memory Safety

All versions have been verified with valgrind:

```bash
# Build test against Rust library
make all-rust

# Run with valgrind
valgrind --leak-check=full ./test_minrx
```

Expected output should show:
```
All heap blocks were freed -- no leaks are possible
ERROR SUMMARY: 0 errors from 0 contexts
```

## Performance Characteristics

All three implementations share the same algorithmic complexity:

- **Time**: O(M × N) where M is pattern length, N is text length
- **Space**: O(M × P) where P is max parentheses nesting depth
- **Non-backtracking**: Single forward scan through input

The relative performance:
- C++ version: Original implementation, moderate speed
- C version: Similar to C++, slightly less abstraction overhead
- Rust version: Competitive performance with zero-cost abstractions

## Build Targets Summary

```bash
# Clean all build artifacts
make clean

# C++ versions (default)
make all-cpp          # rxgrep_cpp, tryit_cpp, test_minrx_cpp
make rxgrep_cpp
make tryit_cpp
make test_minrx_cpp

# C versions
make all-c            # rxgrep_c, tryit_c, test_minrx_c
make rxgrep_c
make tryit_c
make test_minrx_c

# Rust version
make all-rust         # test_minrx (linked to Rust lib)
make rust-lib         # Just build libminrx.so/dylib
cargo build --release # Rust native build
cargo test            # Run Rust unit tests

# Install (C++ version only, uses meson)
make install PREFIX=/usr/local
```

## Migration Guide

### Switching Between Implementations

All three implementations are **drop-in replacements** for each other.
They export identical C APIs, so existing code doesn't need changes.

Simply link against the desired library:

```bash
# Link against C++ version
g++ -o prog prog.c -L. -lminrx

# Link against C version
gcc -o prog prog.c minrx.c

# Link against Rust version
gcc -o prog prog.c -L./target/release -lminrx -Wl,-rpath,./target/release
```

### Choosing an Implementation

**Use C++** if:
- You're already using C++ in your project
- You want the original, most-tested implementation

**Use C** if:
- You need maximum portability
- You're working in a pure C environment
- You need minimal dependencies

**Use Rust** if:
- You want memory safety guarantees
- You're building new Rust projects
- You want modern language features
- You value comprehensive compile-time checking

## Future Work

Potential future enhancements to the conversions:

- **Rust**: Add async support for non-blocking matching
- **Rust**: Publish to crates.io as a library
- **All**: Continued performance optimization
- **All**: Extended benchmarking suite comparing C, C++, and Rust implementations

## Credits

- **Original C++ implementation**: Mike Haertel
- **C conversion**: Claude (AI assistant)
- **Rust conversion**: Claude (AI assistant)
- **Test suite**: Based on comprehensive testing by Douglas McIlroy,
  Glenn Fowler, and others
