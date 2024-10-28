# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!--
Check [Keep a Changelog](http://keepachangelog.com/) for recommendations on how to structure this file.

    Added -- for new features.
    Changed -- for changes in existing functionality.
    Deprecated -- for soon-to-be removed features.
    Removed -- for now removed features.
    Fixed -- for any bug fixes.
    Security -- in case of vulnerabilities.
-->

## [0.18.3]

### Fixed

- Fix issue with getting instrument information if there is data on the output queue

## [0.18.2]

### Added

- Add support for macOS (LAN only)

### Removed

- Remove support for USBTMC without VISA

## [0.18.1]

### Fixed

- Fix issue where versatest instrument fw flash would be aborted by drop

## [0.18.0]

### Added

- VISA support
- Implement Default trait for InstumentInfo struct (TSP-785)
- Reset trait implemented

## [0.17.0]

### Changed

- Properly close connections when an instrument model is `drop`'d

## [0.16.1]

### Fixed

- Fix Support for FW flash on the 3706B and 70xB *Open Source Contribution: c3charvat*


## [0.15.1]

### Changed

- Implemented Drop for AsyncStream (TSP-584)

### Security

- Update `h2` crate (GHSA-q6cp-qfwq-4gcv), which isn't anticipated to be
  exploitable for this crate


## [0.15.0]

### Changed

- Use `*TST?` on TTI instruments instead of `print("unlocked")`
- Add short delay before dropping TTI Instrument to ensure `logout` is sent.


## [0.14.1]

### Fixed

- Update Dependencies (TSP-576)


## [0.13.2]

### Fixed

- Updated project manifests to have update version info


## [0.13.0]

### Changed

- Using `read_password` instead of `prompt_password` of rpassword crate (TSP-517)

<!--Version Comparison Links-->
[Unreleased]: https://github.com/tektronix/tsp-toolkit-kic-lib/compare/v0.18.3..HEAD
[0.18.3]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.18.3
[0.18.2]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.18.2
[0.18.1]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.18.1
[0.18.0]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.18.0
[0.17.0]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.17.0
[0.16.1]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.16.1
[0.15.1]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.15.1
[0.15.0]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.15.0
[0.14.1]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.14.1
[0.13.2]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.13.2
[0.13.0]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.13.0
