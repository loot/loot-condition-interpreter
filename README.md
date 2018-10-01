loot-condition-interpreter
==========================

An experimental library for parsing and evaluating LOOT's metadata condition
strings. Written in Rust. Very incomplete.

Goals:

- Explore parsing conditions into an intermediate representation, then
  evaluating from that representation. LOOT's current parser evaluates
  conditions while parsing them, tightly coupling the two operations.
- Explore more granular result caching. LOOT's current caching maps each
  condition string to its result, which can't handle case insensitivity (some
  parts of the condition string are case sensitive) or re-use results in
  compound conditions.
- Explore round trip serialisation from the intermediate representation.
- Evaluate the cost/benefit of integrating into LOOT, replacing the existing
  code.

Currently only condition parsing is complete. Evaluation is partially done, the
rest hasn't yet been started.

The tests need the [testing-plugins](https://github.com/WrinklyNinja/testing-plugins)
directory to be present in the repo root.
