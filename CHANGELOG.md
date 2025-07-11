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

## [0.21.0]

### Changed
- Major changes to how passwords are used and how instruments are created

## [0.19.10]

### Added

- .abort command to abort all current jobs on the instrument

## [0.19.9]

### Changed

- Update branding from Keithley Instruments to Tektronix

## [0.19.8]

### Fixed

- Fixed issue with not exiting quickly when LAN gets disconnected

## [0.19.7]

### Fixed

- Fixed issue with missing progress indicators on TTI, 3706, and 2600 instruments


## [0.19.6]

### Added

- Progress indicators for very large scripts and firmware files

### Changed

- No longer need to call `slot.stop` and `slot.start` since that is done by firmware now

### Fixed

- Issues with firmware updates over USBTMC on some instruments


## [0.19.5]

### Changed

- Modify trebuchet firmware update procedure to use new command to check for validity

## [0.19.4]

### Fixed

- Fix issue with connecting to some instruments


## [0.19.3]

### Changed

- Remove logging of buffers for writes/reads

## [0.19.2]

### Fixed

- Fix incorrect upgrade commands for trebuchet
- Wait for trebuchet download to finish before continuing
- Stop and start module after loading firmware image

## [0.19.1]

### Fixed

- Don't call tsp commands in `get-info` because the `*LANG` of the instrument
  might be set to something besides `TSP` (specifically on TTI)

## [0.19.0]

### Added

- Added `is_supported` function that takes in a model number
- Added `model_is` method on `model::**::Instrument` structs

## [0.18.4]

### Fixed

- Fix issue with getting instrument information if prompts are turned on

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
[Unreleased]: https://github.com/tektronix/tsp-toolkit-kic-lib/compare/v0.21.0..HEAD
[0.21.0]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.21.0
[0.19.9]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.19.9
[0.19.8]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.19.8
[0.19.7]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.19.7
[0.19.6]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.19.6
[0.19.5]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.19.5
[0.19.4]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.19.4
[0.19.3]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.19.3
[0.19.2]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.19.2
[0.19.1]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.19.1
[0.19.0]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.19.0
[0.18.4]: https://github.com/tektronix/tsp-toolkit-kic-lib/releases/tag/v0.18.4
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
