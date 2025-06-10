# GUI Update Feature Implementation

This branch adds an update feature to the Goose desktop application, similar to the CLI's update command.

## Changes Made

### 1. New Update Section Component (`UpdateSection.tsx`)
- Check for updates by fetching the latest release from GitHub
- Compare current version with latest available version
- Download and execute the update script
- Show progress during download and installation
- Prompt user to restart the app after successful update

### 2. Main Process Updates (`main.ts`)
- Added IPC handler for `execute-update` to run the update script
- Added IPC handler for `restart-app` to relaunch the application
- Update script runs with `CONFIGURE=false` to skip configuration during update

### 3. Preload Script Updates (`preload.ts`)
- Added `getVersion()` method to retrieve current app version
- Added `executeUpdate()` method to execute update scripts
- Added `restartApp()` method to restart the application

### 4. UI Integration
- Integrated UpdateSection into AppSettingsSection
- Added visual separation with a border between app settings and update section

## How It Works

1. User clicks "Check for Updates" button
2. App fetches latest release info from GitHub API
3. Compares versions to determine if update is available
4. If update available, shows "Download & Install" button
5. Downloads the official update script from GitHub releases
6. Executes the script through Electron IPC
7. Shows progress during download/installation
8. Prompts user to restart after successful update

## Testing

To test this feature:
1. Run the desktop app: `npm run start-gui` (from ui/desktop directory)
2. Navigate to Settings > App Settings
3. Scroll down to see the Updates section
4. Click "Check for Updates"

## Future Enhancements

- Add automatic update checks on app startup
- Support for beta/canary release channels
- Background update downloads
- Update notifications in the system tray