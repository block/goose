@echo off
set ELECTRON_PLATFORM=win32
node scripts/build-main.js
node scripts/prepare-platform-binaries.js
npm run make -- --platform=win32 --arch=x64 