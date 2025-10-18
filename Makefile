# default build (requires meson):
#	make compile PREFIX=/dest/dir TYPE={release,debug}
#	make install PREFIX=/dest/dir TYPE={release,debug}
#	make uninstall PREFIX=/dest/dir TYPE={release,debug}
PREFIX=/usr/local
TYPE=release
.PHONY: compile install uninstall
compile install:: builds/$(TYPE)/meson-info
	cd builds/$(TYPE) && meson $@
compile::
	ln -sf builds/$(TYPE)/rxgrep .
	ln -sf builds/$(TYPE)/tryit .
uninstall: builds/$(TYPE)/meson-info
	cd builds/$(TYPE) && ninja $@
builds/$(TYPE)/meson-info:
	meson setup builds/$(TYPE) --prefix=$(PREFIX) --buildtype=$(TYPE)

# traditional build (requires only make)
CFLAGS=-O3 -Wall -Wextra
CXXFLAGS=-std=c++20 -O3 -Wall

# C versions (using minrx.c)
minrx_c.o: minrx.c minrx.h
	$(CC) $(CFLAGS) -c -o $@ $<

rxgrep_c: minrx_c.o rxgrep.o
	$(CC) -o $@ $^

tryit_c: minrx_c.o tryit.o
	$(CC) -o $@ $^

test_minrx_c: minrx_c.o test_minrx.o
	$(CC) -o $@ $^

test_memory_leaks: minrx_c.o test_memory_leaks.o
	$(CC) -o $@ $^

# C++ versions (using minrx.cpp)
minrx_cpp.o: minrx.cpp minrx.h
	$(CXX) $(CXXFLAGS) -c -o $@ $<

rxgrep_cpp: minrx_cpp.o rxgrep.o
	$(CXX) -o $@ $^

tryit_cpp: minrx_cpp.o tryit.o
	$(CXX) -o $@ $^

test_minrx_cpp: minrx_cpp.o test_minrx.o
	$(CXX) -o $@ $^

# Default targets (C++ versions for backward compatibility)
rxgrep: rxgrep_cpp
	ln -sf $< $@

tryit: tryit_cpp
	ln -sf $< $@

# Rust version (using cargo)
.PHONY: rust-lib
rust-lib:
	cargo build --release

test_minrx: test_minrx.c rust-lib
	$(CC) $(CFLAGS) -I. -o $@ $< -L./target/release -lminrx -Wl,-rpath,./target/release

test_minrx_rust: test_minrx
	ln -sf $< $@

# Build all versions
.PHONY: all all-c all-cpp all-rust
all: all-c all-cpp

all-c: rxgrep_c tryit_c test_minrx_c test_memory_leaks

all-cpp: rxgrep_cpp tryit_cpp test_minrx_cpp

all-rust: test_minrx test_minrx_rust

# removes both default and traditional build artifacts
.PHONY: clean
clean:
	rm -fr builds *.o rxgrep tryit rxgrep_c rxgrep_cpp tryit_c tryit_cpp test_minrx test_minrx_rust test_minrx_c test_minrx_cpp test_memory_leaks
	cargo clean
