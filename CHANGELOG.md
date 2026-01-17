# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2026-01-XX

### Added
- **GRUB_DEFAULT Format Validation and Auto-Fix**: Automatic detection and fixing of old GRUB_DEFAULT format
  - Detects old title format (e.g., `Ubuntu, with Linux 6.5.0-rc2-snp-host-ec25de0e7141`)
  - Automatically converts to numeric path format (e.g., `0>2`) when loading configuration
  - Interactive warning and auto-fix prompt when viewing default boot entry with old format
  - Supports GRUB version detection for format compatibility

### Enhanced
- **GRUB Validation Module**: Extended `grub_validate` module with new functions
  - `get_grub_version()`: Detect GRUB version (major.minor)
  - `is_old_grub_default_format()`: Check if GRUB_DEFAULT uses deprecated title format
  - `fix_old_grub_default_format()`: Convert old title format to numeric path format
- **Configuration Loading**: Enhanced `GrubConfig::load()` to automatically detect and fix old formats
- **View Default Entry**: Added warning and auto-fix option when old format is detected

### Fixed
- Resolves GRUB warnings about using old title format for GRUB_DEFAULT
- Ensures compatibility with GRUB 2.00+ requirements

### Technical
- Added format validation logic in `grub_validate` module
- Enhanced `GrubConfig` with `validate_and_fix_grub_default()` method
- Improved error handling and user feedback for format issues

## [0.3.0] - 2026-01-15

### Added
- **Boot Time Statistics**: View current and historical boot times with kernel versions
  - Display boot time for current boot using `systemd-analyze time`
  - Show historical boot times from `journalctl --list-boots`
  - Display kernel version for each boot entry
- **State Stack Navigation**: Implemented navigation history stack for better UX
  - Return to correct parent menu instead of always going back to main menu
  - Maintain navigation context when navigating through sub-menus
- **All GRUB Parameters Configuration**: Extended configuration system to support all `/etc/default/grub` parameters
  - View and edit any GRUB parameter, not just predefined ones
  - Dynamic parameter loading and saving
  - Preserve comments and formatting in configuration file

### Changed
- **GRUB Settings Consolidation**: Merged "Configure Kernel Parameters" and "Configure GRUB Timeout" into single "Configure GRUB Settings" menu
  - Unified interface for all GRUB configuration options
  - Integrated GRUB validation directly into configuration menu
- **Enhanced Navigation**: Improved menu navigation with state stack
  - More intuitive back navigation
  - Better context preservation
- **Configuration System Refactor**: 
  - Refactored `GrubConfig` to use `HashMap` for storing all parameters
  - Backward compatible with existing code
  - More flexible parameter management

### Technical
- Added `boot_time` module for boot time statistics
- Enhanced `grub_config` module with dynamic parameter support
- Implemented state stack in main application state management
- Added `EditAllGrubParams` state for comprehensive parameter editing

## [0.2.0] - 2024-12-XX

### Added
- **Kernel Version Information Display**: View kernel version, release, architecture, and file path for each boot entry
- **Kernel Cleanup Tool**: Scan and delete unused kernel versions to free up disk space
- **Boot Entry Renaming**: Set custom names for boot entries for easier identification
- **Backup Manager**: View, restore, and delete GRUB configuration backups
- **GRUB Configuration Validation**: Validate GRUB configuration syntax using grub-mkconfig

### Changed
- Main menu now includes 5 new management tools
- Enhanced boot entry selection to support different actions (set default, view info, rename)

### Technical
- Added new modules: `kernel_info`, `kernel_cleanup`, `custom_names`, `backup_manager`, `grub_validate`
- Added dependencies: `serde`, `serde_json`, `chrono` for data serialization and time handling

## [0.1.5] - 2024-12-XX

### Added
- **Ratatui Framework**: Complete refactor to use ratatui (formerly tui-rs) for modern TUI experience
- **Mouse Support**: Click on menu items to select them
- **Right Key Support**: Use Right arrow key for continuous editing in parameter and timeout configuration menus
- **Circular Scrolling**: Up/Down navigation now wraps around in all menus
- **Left Key Navigation**: Use Left arrow key to return to previous menu in all configuration screens

### Changed
- **UI Framework**: Migrated from custom terminal rendering to ratatui framework with crossterm backend
- **Main Menu**: Now only displays configuration options, boot entries accessed through Set Default Boot Entry submenu
- **Exit Behavior**: Unified to use ESC key consistently across all menus
- **Search Integration**: Search functionality integrated into Set Default Boot Entry menu
- **Search Initiation**: Any letter or number key can start search mode

### Fixed
- Fixed navigation issues in SelectBootEntry when path is empty
- Fixed menu selection for ConfigureKernelParams and EditParameterList states
- Fixed various UI rendering issues with ratatui integration
- Fixed circular scrolling implementation for all menus

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

