# Windows-Specific Binaries

This directory contains Windows-specific binaries and scripts that are only included during Windows builds.

## Components

### Node.js Installation
- `install-node.cmd` - Script to check for and install Node.js if needed
- `npx.cmd` - Wrapper script that ensures Node.js is installed and uses system npx

### Windows Binaries
- `*.dll` files - Required Windows dynamic libraries
- `*.exe` files - Windows executables

## Build Process

These files are generated during the Windows build process by:
1. `prepare-windows-npm.sh` - Creates Node.js installation scripts
2. `copy-windows-dlls.js` - Copies all Windows-specific files to the output directory

None of these files should be committed to the repository - they are generated fresh during each Windows build.
