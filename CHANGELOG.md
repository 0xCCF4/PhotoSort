# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.4](https://github.com/0xCCF4/PhotoSort/compare/v0.2.3...v0.2.4) - 2024-10-29

### Added

- added multithreading support

### Other

- Merge pull request [#59](https://github.com/0xCCF4/PhotoSort/pull/59) from 0xCCF4/dependabot/cargo/regex-1.11.1
- *(deps)* bump regex from 1.11.0 to 1.11.1

## [0.2.3](https://github.com/0xCCF4/PhotoSort/compare/v0.2.2...v0.2.3) - 2024-10-28

### Added

- added progress bar option
- added a --log and --quiet option

### Other

- ran cargo fmt
- changed log level for unprocessed files from debug to info, --debug now has TRACE log level
- *(deps)* bump anyhow from 1.0.89 to 1.0.90

## [0.2.2](https://github.com/0xCCF4/PhotoSort/compare/v0.2.1...v0.2.2) - 2024-10-20

### Added

- added option to specify file format for unknown files (which are not images/videos)
- a different format string for files with no derived date
- output warning if no date was derived for a file

## [0.2.1](https://github.com/0xCCF4/PhotoSort/compare/v0.2.0...v0.2.1) - 2024-10-18

### Added

- extension can be made upper or lower case when specifying a custom name format

### Other

- fixed clippy errors
- ran cargo fmt
- *(deps)* bump clap from 4.5.19 to 4.5.20
- *(deps)* bump clap from 4.5.18 to 4.5.19
- *(deps)* bump actions-rust-lang/setup-rust-toolchain
- Merge pull request [#41](https://github.com/0xCCF4/PhotoSort/pull/41) from 0xCCF4/dependabot/cargo/regex-1.11.0
- *(deps)* bump regex from 1.10.6 to 1.11.0
- *(deps)* bump clap from 4.5.17 to 4.5.18
- *(deps)* bump actions-rust-lang/setup-rust-toolchain
- *(deps)* bump anyhow from 1.0.87 to 1.0.89

## [0.2.0](https://github.com/0xCCF4/PhotoSort/compare/v0.1.6...v0.2.0) - 2024-09-13

### Added

- allow specifying a format string that allows subfolder creation [#33](https://github.com/0xCCF4/PhotoSort/pull/33)
- [**breaking**] overhauled file format interface

### Fixed

- *(doc)* fixed README.md examples

### Other

- *(doc)* cargo fmt
- updated readme to reflect the new cli options
- [**breaking**] moved parts of the source to own files, added more debug/error information
- Merge pull request [#32](https://github.com/0xCCF4/PhotoSort/pull/32) from 0xCCF4/dependabot/cargo/anyhow-1.0.87
- *(deps)* bump anyhow from 1.0.86 to 1.0.87

## [0.1.6](https://github.com/0xCCF4/PhotoSort/compare/v0.1.5...v0.1.6) - 2024-06-26

### Added
- updated ci, added release plz, devskim, automerge dependabot

### Fixed
- ci auto merge pr
- fixing ci video lib missing
- fixing ci video lib missing
- change ci token
- change ci token
- change compile against stable in ci

### Other
- *(deps)* bump actions-rust-lang/setup-rust-toolchain ([#12](https://github.com/0xCCF4/PhotoSort/pull/12))
- *(deps)* bump rust-build/rust-build.action from 1.4.4 to 1.4.5 ([#13](https://github.com/0xCCF4/PhotoSort/pull/13))
- renamed ci jobs
- Bump clap from 4.5.4 to 4.5.7 ([#10](https://github.com/0xCCF4/PhotoSort/pull/10))
- Bump regex from 1.10.4 to 1.10.5 ([#9](https://github.com/0xCCF4/PhotoSort/pull/9))
- Bump lazy_static from 1.4.0 to 1.5.0 ([#11](https://github.com/0xCCF4/PhotoSort/pull/11))
- cargo fmt and clippy
- update ci
- update ci
- Merge pull request [#6](https://github.com/0xCCF4/PhotoSort/pull/6) from 0xCCF4/dependabot/cargo/anyhow-1.0.86
- Bump ffmpeg-next from 7.0.0 to 7.0.2
