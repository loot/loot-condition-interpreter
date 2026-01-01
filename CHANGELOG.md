# Changelog

## [5.4.0] - 2026-01-01

### Changed

- Removed the pelite dependency in favour of a new built-in PE version parsing
  implementation.
- Removed the FFI wrapper's direct dependency on libc: `usize` is now used in
  place of the `libc::size_t` type alias (which is `usize` on all supported
  platforms).
- Updated dependency versions in Cargo.lock:

  - Updated esplugin to v6.1.4.
  - Updated regex to v1.12.2.

## [5.3.3] - 2025-09-29

### Fixed

- The repository link in Cargo.toml pointed to the wrong Git repository.

### Changed

- Releases are now published to crates.io using Trusted Publishing.
- The accepted dependency version ranges have been widened to include older
  versions. The oldest accepted version for each dependency is the oldest that
  loot-condition-interpreter will build and pass its tests with on x86-64
  Windows and Linux.
- Updated dependency versions in Cargo.lock:

  - Updated crc32fast to v1.5.0.
  - Updated libc to v0.2.175.
  - Updated regex to v1.11.2.

- Most of the Cargo.toml package fields are now inherited from workspace fields
  that are now shared between loot-condition-interpreter and
  loot-condition-interpreter-ffi.
- Most of the lint configuration has been moved from Rust attributes to
  workspace configuration, leaving only the configuration that is specific to
  the relevant crate.

## [5.3.2] - 2025-04-28

### Fixed

- If an FFI error message contains a nul byte, instead of using it and causing
  the message to be truncated and leaking the rest of the string, the nul byte
  will be escaped and that escaped message will be used instead.
- Compilation would fail when targeting platforms for which `pelite` does not
  provide memory map support. Instead, reading an executable now falls back to
  reading it into memory before parsing its header.
- The "not equal" version comparison operator was serialised as `==` instead of
  `!=`.
- The `description_contains()` condition did not read the description field of
  OpenMW plugins.

### Changed

- Paths in conditions are no longer restricted to staying within the directory
  tree that starts one level above the game's data path, as the restriction
  didn't add any real value and did not work correctly for OpenMW or games with
  additional data paths.
- Paths in error messages will now be output as ASCII strings with character
  escapes instead of unicode strings with surrogates escaped.
- Some code that used functions that could panic has been rewritten to avoid
  the risk of changes accidentally introducing panics.
- Updated esplugin to v6.1.3.
- Updated libc to v0.2.172.

## [5.3.1] - 2025-03-26

### Added

- Metadata fields in `Cargo.toml` so that the crate can be published to
  crates.io.

## [5.3.0] - 2025-03-26

### Added

- Implemented `Ord` and `PartialOrd` on `GameType`.
- Implemented `Ord`, `PartialOrd` and `Hash` on `MoreDataNeeded`.
- Implemented `Clone`, `Eq` and `PartialEq` on `ParsingError` and
  `ParsingErrorKind`.

### Changed

- The `Box<dyn error::Error>` held by `Error::PeParsingError` is now a
  `Box<dyn error::Error + Send + Sync + 'static>`.
- If the state's CRC or conditions cache are in a poisoned state when they are
  accessed, they will now automatically be recreated in an empty non-poisoned
  state before continuing, and the PoisonError will no longer be returned to the
  caller.
- Updated libc to v0.2.171.

## [5.2.0] - 2025-02-20

### Changed

- Line breaks are now accepted as whitespace when parsing condition strings, so
  long expressions can be split over multiple lines.

## [5.1.0] - 2025-02-16

### Added

- Support for a `file_size()` condition function that returns true if the given
  file's size matches the given number of bytes, and false otherwise (including
  if the file doesn't exist).
- Support for a `filename_version()` condition function that takes a regex path
  with a single capture group, a version string and a comparison operator and
  returns true if there is a path that matches the regex path and the value
  captured by the regex is a version string for which the comparison against the
  given version is true. Unlike the other version functions, it always returns
  false if it cannot find a version to compare against the given version,
  irrespective of the given comparison operator.
- Support for a `description_contains()` condition function that takes a path
  and a regex and returns true if the given path is a plugin with a description
  that contains text that matches the given regex, and false otherwise
  (including if the path does not exist, is not a plugin, or has no
  description).

## [5.0.0] - 2025-02-01

### Added

- `GameType::OpenMW` and `LCI_GAME_OPENMW` as the game code to use for OpenMW.
  Unlike other games:

  - `.omwgame`, `.omwaddon` and `.omwscripts` are valid plugin file extensions.
  - Ghosted plugins are not supported.
  - The `is_master()` condition function returns `false` for all OpenMW plugins.
  - When resolving filenames, the additional data paths are checked in reverse
    order.
- Support for an `is_executable(path)` condition function that returns true if
  the given path is a Windows executable (PE) file.

### Changed

- The `Error` and `GameType` enums are now non-exhaustive.
- Updated libc to v0.2.169.
- Updated regex to v1.11.1.
- Updated unicase to v2.8.1.

## [4.0.2] - 2024-10-08

### Changed

- Updated esplugin to v6.1.1.
- Updated libc to v0.2.159.
- Updated regex to v1.11.0.

## [4.0.1] - 2024-06-28

### Changed

- Updated crc32fast to v1.4.2.
- Updated esplugin to v6.0.0.
- Updated libc to v0.2.155.
- Updated regex to v1.10.5.

## [4.0.0] - 2024-05-02

### Added

- `Cargo.lock` is no longer ignored by Git.

### Changed

- `Error::ParsingIncomplete` now holds contextual data.
- Updated to Rust's 2021 edition.
- Updated esplugin to v5.0.1.

### Removed

- The `ffi-headers` build feature: if you want to generate C or C++ headers,
  install and run cbindgen separately.

## [3.1.0] - 2023-09-05

### Added

- `GameType::Starfield` and `LCI_GAME_STARFIELD` as the game code to use for
  Starfield.

### Fixed

- Only lowercase plugin file extensions were recognised as plugin file
  extensions.

## [3.0.0] - 2023-08-18

### Removed

- Support for the `"LOOT"` file path alias in conditions.
- The `State::new()` and `lci_state_create()` functions no longer take a
  `loot_path` argument.

## [2.4.0] - 2023-04-24

### Added

- Support for additional data paths that take precedence over the game's main
  data path. Additional data paths can be specified using
  `State::set_additional_data_paths()` or
  `lci_state_set_additional_data_paths()`. This is intended to support the
  Microsoft Store's Fallout 4 DLCs, which are installed outside of the base
  game install path, but may also be useful in other situations.

## [2.3.1] - 2022-09-15

### Changed

- Updated esplugin to v4.0.0.

## [2.3.0] - 2022-02-26

### Added

- Support for a `readable(path)` condition function that returns true if
  the given path is a readable file or directory and false otherwise.

## [2.2.2] - 2022-02-04

### Changed

- Updated nom to v7.0.0.
- Updated cbindgen to v0.20.

### Fixed

- Two versions that only differ by the presence and absence of pre-release
  identifiers were not correctly compared according to Semantic Versioning,
  which states that `1.0.0-alpha` is less than `1.0.0`.

## [2.2.1] - 2021-04-25

### Changed

- Version comparison now compares numeric against non-numeric release
  identifiers (and vice versa) by comparing the numeric value against the
  numeric value of leading digits in the non-numeric value, and treating the
  latter as greater if the two numeric values are equal. The numeric value as
  treated as less than the non-numeric value if the latter has no leading
  digits. Previously all non-numeric identifiers were always greater than any
  numeric identifier. For example, `78b` was previously considered to be greater
  than `86`, but is now considered to be less than `86`.

## [2.2.0] - 2021-04-17

### Added

- Support for inverting expressions using `not (<expression>)` syntax, e.g.
  `not ( file("example1") or file("example2") )`.

### Changed

- When evaluating a regular expression, installed ghosted plugin filenames have
  their `.ghost` file extension removed before they are matched against the
  regex. This makes functions that take regexes behave the same as those that
  take paths when handling ghosted plugins.
- Updated nom to v6.0.0.
- Updated cbindgen to v0.19.

### Fixed

- `.ghost` file extensions are no longer recursively trimmed when checking if a
  file has a plugin file extension, as only a single `.ghost` extension is
  valid.
- When looking for a plugin file matching a path, only add a `.ghost` extension
  to the path if one is not already present.

## [2.1.2] - 2020-10-23

### Fixed

- Version `0.1.1` of `pelite-macros` (a dependency of the `pelite` dependency)
  broke the ability to build `pelite` v0.8.x without pinning the version of
  `pelite-macros` used.

### Changed

- Updated pelite to v0.9.0.
- Updated cbindgen to v0.15.

## [2.1.1] - 2020-06-13

### Changed

- Checksum calculations are now much faster for larger files.
- Directory paths are now handled more gracefully in `checksum()`, `version()`
  and `product_version()` conditions.
- Resolved some `rustc` deprecation warnings by replacing usage of
  `std::error::Error`'s `description()` function with `to_string()`.
- Updated cbindgen to v0.14.
- Updated pelite to v0.8.0.

## [2.1.0] - 2019-10-05

### Added

- Support for an `is_master(file path)` condition function that returns true if
  the given file path is a master plugin, and false otherwise.

## [2.0.1] - 2019-07-23

### Fixed

- Regular expressions are now prefixed with `^` and suffixed with `$` to ensure
  that only exact matches to the given expression are found.

## [2.0.0] - 2019-06-30

### Added

- A cbindgen configuration file at `ffi/cbindgen.toml` so that cbindgen can now
  be run as `cbindgen ffi/ -o ffi/include/loot_condition_interpreter.h`.

### Changed

- The `ParsingError` enum has been renamed to `ParsingErrorKind`, and its
  `Unknown(u32)` variant has been replaced by a `GenericParserError(String)`
  variant.
- The `Error::GenericParsingError` and `Error::CustomParsingError` variants have
  been combined into a single `Error::ParsingError(String, ParsingErrorKind)`
  variant.
- `GameType` variants have been renamed to use fewer acronyms:
  - `Tes4` -> `Oblivion`
  - `Tes5` -> `Skyrim`
  - `Tes5se` -> `SkyrimSE`
  - `Tes5vr` -> `SkyrimVR`
  - `Fo3` -> `Fallout3`
  - `Fonv` -> `FalloutNV`
  - `Fo4` -> `Fallout4`
  - `Fo4vr` -> `Fallout4VR`
  - `Tes3` -> `Morrowind`
- The `LCI_GAME_*` constants have been renamed to match the new `GameType`
  names:
  - `LCI_GAME_TES4` -> `LCI_GAME_OBLIVION`
  - `LCI_GAME_TES5` -> `LCI_GAME_SKYRIM`
  - `LCI_GAME_TES5SE` -> `LCI_GAME_SKYRIM_SE`
  - `LCI_GAME_TES5VR` -> `LCI_GAME_SKYRIM_VR`
  - `LCI_GAME_FO3` -> `LCI_GAME_FALLOUT_3`
  - `LCI_GAME_FNV` -> `LCI_GAME_FALLOUT_NV`
  - `LCI_GAME_FO4` -> `LCI_GAME_FALLOUT_4`
  - `LCI_GAME_FO4VR` -> `LCI_GAME_FALLOUT_4_VR`
  - `LCI_GAME_TES3` -> `LCI_GAME_MORROWIND`
- The C header generated by cbindgen can now be used from C++.
- Updated nom to v5.
- Updated cbindgen to v0.9.
- Updated code to Rust 2018 syntax.

### Removed

- The C++ header `loot_condition_interpreter.hpp` is no longer generated by
  cbindgen. Include `loot_condition_interpreter.h` instead.

### Fixed

- Evaluating `version()` and `product_version()` conditions will no longer error
  if the given executable has no version fields. Instead, it will be evaluated
  as having no version.

## [1.3.0] - 2019-04-07

### Added

- Support for Morrowind using ``GameType::tes3``.

## [1.2.2] - 2019-01-26

### Fixed

- `file(<regex>)`, `active(<regex>)`, `many(<regex>)` and `many_active(<regex>)`
  did not parse the closing `)`, causing any remaining input to be skipped.

## Changed

- Parsing expressions will now fail if it does not consume all the given input.

## [1.2.1] - 2019-01-20

### Fixed

- Parsing error when reading the version fields of an executable that did not
  have any US English version info. Reading executables' version fields now uses
  the first version info structure instead of attempting to read the US English
  version info structure.

## [1.2.0] - 2018-12-22

### Added

- Support for parsing version strings that match the regular expression
  `\d+, \d+, \d+, \d+`.

### Changed

- An executable's product version is now read from the `ProductVersion` field in
  the executable's `VS_VERSIONINFO` structure, not from the product version
  fields in the `VS_FIXEDFILEINFO` structure. This is so that the version read
  matches the version displayed by Windows' File Explorer.

## [1.1.1] - 2018-11-14

### Fixed

- Parsing error caused by using `>=` or `<=` as the comparator in version
  functions.
- Parsing error when encountering backslashes in a version or checksum
  function's path argument.
- Parsing error when parentheses around expressions are padded with whitespace.

## [1.1.0] - 2018-10-23

### Added

- Support for `product_version()` condition functions, e.g.
  `product_version("file.exe", "1.0.0", ==)`. It will read the product version
  field of the executable at the given path, or error if the given path does not
  point to an executable.

## [1.0.0] - 2018-10-21

Initial release
