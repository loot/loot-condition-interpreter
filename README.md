loot-condition-interpreter
==========================

![CI](https://github.com/loot/loot-condition-interpreter/workflows/CI/badge.svg?branch=master)
[![Coverage Status](https://coveralls.io/repos/github/loot/loot-condition-interpreter/badge.svg?branch=master)](https://coveralls.io/github/loot/loot-condition-interpreter?branch=master)

A library for parsing and evaluating LOOT's metadata condition strings. It
provides:

- Support for metadata syntax v0.15 condition strings.
- Condition string parsing without evaluation, for checking syntax.
- Evaluation of parsed condition strings.
- Efficient and safe concurrent condition evaluation thanks to Rust's safety
  guarantees.
- Caching of individual function evaluation results and calculated CRCs.
- Executable version parsing without any external runtime dependencies.
- Lots of tests, and benchmarks.
- A C FFI library that wraps the Rust library.

## Build

Make sure you have [Rust](https://www.rust-lang.org/) installed.

To build the Rust and C FFI libraries, run:

```
cargo build --release --package loot-condition-interpreter-ffi
```

To generate a C++ header file as part of the build, instead run:

```
cargo build --release --package loot-condition-interpreter-ffi --all-features
```

This will create a file at `ffi/include/loot_condition_interpreter.h`.
However, this involves building [cbindgen](https://github.com/eqrion/cbindgen)
as a dependency, which is relatively slow. If you have cbindgen installed
separately, it's faster to generate the headers using its CLI:

```
cbindgen ffi/ -o ffi/include/loot_condition_interpreter.h
```

## Tests & Benchmarks

The tests and benchmarks need the [testing-plugins](https://github.com/WrinklyNinja/testing-plugins)
and the [LOOT API v0.13.8](https://github.com/loot/loot-api/releases/tag/0.13.8)
Windows archives to be extracted and present in the repo root. See the GitHub
Actions CI workflow for examples on what should be extracted where.

To run the Rust tests:

```
cargo test --all
```

To run the benchmarks:

```
cargo bench
```

There are also C++ tests for the FFI library, they require a C++ toolchain and
[CMake](https://cmake.org/) to be installed. To run the C++ tests:

```
mkdir ffi/build
cd ffi/build
cmake ..
cmake --build .
ctest
```
