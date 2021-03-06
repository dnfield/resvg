# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

### Added
- (svg) Partial `clipPath` support.
- (svg) Partial `mask` support.
- (svg) Partial `pattern` support.
- (svg) Check that an external image is PNG or JPEG.
- (cli) Added `--query-all` and `--export-id` arguments to render SVG items by ID.
- (cli) Added `--perf` argument for a simple performance stats.

### Changed
- (lib) API is completely new.

### Fixed
- `font-size` attribute inheritance during `use` resolving.

[Unreleased]: https://github.com/RazrFalcon/resvg/compare/v0.1.0...HEAD
