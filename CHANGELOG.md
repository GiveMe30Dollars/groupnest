# Changelog

All notable changes to `groupnest` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

The completion of support for the `serde` feature flag.

### Added

- **`serde` feature flag:** `de::DeserializeSeed` implementation for `RefDocBuilder`.
- `Document::to_representation`, which converts any document type to an equivalent representation form.

### Changed

- **`serde` feature flag:** Various changes to `Serialize` and `Deserialize` implementations to erase and reconstruct derived internal fields of certain types essential to the correctness of the layout algorithm.

## [0.1.1] - 2026-08-16

### Added

- [`CHANGELOG.md`] file.
- `repository` package metadata for [`Cargo.toml`](/Cargo.toml).

### Changed

- [`README.md`] now links to the corresponding [`Doc.rs`](https://docs.rs/groupnest/latest/groupnest/index.html) website, rather than into the source code.

## [0.1.0] - 2026-08-16

### Added

- Initial public release.

[Unreleased]: https://github.com/GiveMe30Dollars/groupnest/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/GiveMe30Dollars/groupnest/releases/tag/v0.1.1
[0.1.0]: https://github.com/GiveMe30Dollars/groupnest/releases/tag/v0.1.0

[`README.md`]: /README.md
[`CHANGELOG.md`]: /CHANGELOG.md