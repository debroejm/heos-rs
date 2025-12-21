## [0.3.0] - 2025-12-21

### 🚀 Features

- Add browse music screen
- Highlight currently playing track in queue
- Add volume controls
- Add delete button to queue tracks

### 🐛 Bug Fixes

- Remove force-enabled console for Windows builds
- Revert screen to "Devices" if selected playable becomes invalid
- Update devices screen when NowPlaying changes
- Display leading zeros for seconds in MediaBar
- Disable unusable control buttons when a station is playing
- Re-enable console on Windows for debug builds
- Force dark mode for now, as light mode is not yet properly configured
## [heos-control-v0.2.0] - 2025-12-06

### 🚀 Features

- Add a queue screen
- Tidy up and add selection visuals for sidebar

### 🐛 Bug Fixes

- Wait for existing updates to finish before starting another

### 🚜 Refactor

- Standardize media item data structures
- Move Updater management to main.rs
- Add Actions structure for centralized async HEOS actions

### ⚙️ Miscellaneous Tasks

- Add changelog to heos-control
- Release
## [heos-control-v0.1.1] - 2025-11-30

### 🚀 Features

- Add group management support

### 🐛 Bug Fixes

- Use new CommandError format for media bar

### 💼 Other

- Initial commit of heos-control
- Add release configuration for heos-control

### ⚙️ Miscellaneous Tasks

- Release
