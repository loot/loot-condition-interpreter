# Changelog

## [1.2.1] - 2018-01-20

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
