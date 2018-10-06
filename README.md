loot-condition-interpreter
==========================

An experimental library for parsing and evaluating LOOT's metadata condition
strings. Written in Rust. Very incomplete.

Goals:

- [x] Explore parsing conditions into an intermediate representation, then
  evaluating from that representation. LOOT's current parser evaluates
  conditions while parsing them, tightly coupling the two operations.
- [ ] Explore more granular result caching. LOOT's current caching maps each
  condition string to its result, which can't re-use results in compound
  conditions.
- [x] Explore round trip serialisation from the intermediate representation.
- [ ] Evaluate the cost/benefit of integrating into LOOT, replacing the existing
  code.

The tests need the [testing-plugins](https://github.com/WrinklyNinja/testing-plugins)
and the [LOOT API v0.13.8](https://github.com/loot/loot-api/releases/tag/0.13.8)
Windows archives to be extracted and present in the repo root.
