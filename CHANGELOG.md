# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.4.0 - 2025-01-23

### Changed

- Update esp-hal to `0.21.1`
- `Display::new` is now fallible and returns a `Result`

### Fixed

- Fixed multiple integer overflows. 