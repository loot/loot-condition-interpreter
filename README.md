loot-condition-interpreter
==========================

An experimental library for parsing and evaluating LOOT's metadata condition
strings. This library improves upon LOOT's existing implementation in the
following ways:

- Condition expressions are parsed into an intermediate representation instead
  of being stringly typed, which allows parsing and evaluation to be separated.
- State is uncoupled by necessity, and Rusts's concurrency guarantees mean it
  can be accessed more efficiently.
- Results are cached with more granularity, per function instead of per
  expression, improving performance when expressions are not entirely different.
- Result caching is guided by benchmarks, so results aren't cached
  unnecessarily.
- Reading executable versions doesn't involve calling out to the shell and
  piping several commands together when on Linux.

The code is also not as much of a mess, it's got benchmarks, and probably better
test coverage.

The library is still experimental because it currently lacks:

- an FFI for LOOT to call it through.
- good error handling, details about parsing errors are not exposed.

The tests need the [testing-plugins](https://github.com/WrinklyNinja/testing-plugins)
and the [LOOT API v0.13.8](https://github.com/loot/loot-api/releases/tag/0.13.8)
Windows archives to be extracted and present in the repo root.
