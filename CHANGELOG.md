# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.4] - 2024-12-XX

### Added
- **Interactive Boot Entry Selection**: Set Default Boot Entry now opens an interactive menu for selecting boot entries
- **Integrated Search in Set Default Menu**: Search functionality is now integrated into the Set Default Boot Entry menu
- **Type-to-Search**: Any letter or number key can now start search mode, not just `/`
- **Unified Exit Behavior**: All menus now use ESC key to exit to parent menu for consistent user experience

### Changed
- **Main Menu Redesign**: Main menu now only displays configuration options, boot entries are accessed through Set Default Boot Entry submenu
- **Search Mode Enhancement**: Special keys (n, y, q, d, /) can now be used in search queries
- **Improved Navigation**: More intuitive menu navigation with consistent ESC key behavior

### Fixed
- Fixed issue where special keys (n, y, q, d, /) couldn't be used in search queries
- Fixed search functionality to properly handle all printable characters

## [0.1.3] - Previous Release

### Added
- Kernel parameter configuration feature
- Interactive parameter editing interface
- Configuration backup functionality

## [0.1.2] - Previous Release

### Added
- View default boot entry feature
- Configure GRUB timeout feature

## [0.1.1] - Previous Release

### Added
- Set default boot entry (permanent) feature

## [0.1.0] - Initial Release

### Added
- Basic GRUB boot entry selection
- Interactive menu interface
- Color output support
- Terminal keyboard input handling

