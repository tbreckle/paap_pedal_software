# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CI/CD pipeline with GitFlow releases: `release-start.yml` creates release/hotfix branches, `release-finish.yml` tags, builds and publishes GitHub Releases
- SemVer versioning via `scripts/version.sh`, injected into app and firmware at build time (unofficial builds are `0.0.0+<sha>`)
- macOS universal binary (Apple Silicon + Intel)
- `app/rust-toolchain.toml` pins the Rust toolchain (1.98.1 with clippy and rustfmt) for local builds and CI
- Pedal number (1-16), stored on the pedal and set in the app. The firmware adds it to the USB serial number (`…PAAPnn`), so the app lists pedals as `PAAP Pedal N` and the OS can tell them apart; new `PED` serial command
- Device list shows `[VID:PID]` at the end of each entry, lists pedals first and refreshes every 2 seconds while disconnected
- Key capture: click the keycap and press the key; a caption tells similar looking keys apart (lowercase L, uppercase I, digit 1)
- App icon, icons on all buttons (Lucide) and an About dialog with the Slint attribution

### Changed
- Configurator UI rewritten with Slint instead of egui; serial communication runs in the background, so the window stays responsive
- Rust edition 2024, dependencies updated to their latest versions (serialport 4.10, confy 2.0)
- App and firmware share one version and one `vX.Y.Z` release tag instead of separate `app-*`/`firmware-*` tags
- Release assets are archives named `paap-configurator-<version>-<platform>` and `paap-firmware-<version>.hex`, with `SHA256SUMS`

### Fixed
- Windows: no console window opens next to the app

## [1.0.0] - 2026-01-08

### Added
- Initial release of the PAAP firmware and PAAP Configurator

---

## Guide for Updating the Changelog

When making changes to the project:

1. Add your changes to the "Unreleased" section under the appropriate category
2. When a release starts, `release-start.yml` moves the "Unreleased" entries into a new
   `## [X.Y.Z] - date` section (`scripts/changelog.sh release`). That section becomes the
   GitHub Release notes (`scripts/changelog.sh notes`), empty categories are left out.

### Categories

- **Added**: New features
- **Changed**: Changes in existing functionality
- **Fixed**: Bug fixes
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Security**: Security fixes

### Examples

```markdown
### Added
- Support for key combinations (#42)
- Keyboard shortcut Ctrl+S for saving to the device

### Fixed
- Fixed serial connection timeout on Windows (#38)
- Corrected firmware version detection

### Changed
- Improved UI responsiveness during sync operations
```
