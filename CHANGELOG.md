# Changelog

## [1.1.0] - 2018-10-23

### Added

- Support for `product_version()` condition functions, e.g.
  `product_version("file.exe", "1.0.0", ==)`. It will read the product version
  field of the executable at the given path, or error if the given path does not
  point to an executable.

## [1.0.0] - 2018-10-21

Initial release
