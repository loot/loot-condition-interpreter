# Changelog

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
