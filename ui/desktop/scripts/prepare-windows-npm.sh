#!/bin/bash
set -e

# Script to prepare Windows npm bundle
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM_WIN_DIR="$SCRIPT_DIR/../src/platform/windows"
WIN_BIN_DIR="$PLATFORM_WIN_DIR/bin"
DEST_BIN_DIR="$SCRIPT_DIR/../src/bin"

echo "Preparing Windows npm bundle..."
echo "SCRIPT_DIR: $SCRIPT_DIR"
echo "PLATFORM_WIN_DIR: $PLATFORM_WIN_DIR"
echo "WIN_BIN_DIR: $WIN_BIN_DIR"
echo "DEST_BIN_DIR: $DEST_BIN_DIR"

# Ensure directories exist
mkdir -p "$WIN_BIN_DIR"

# Node.js version and paths
NODE_VERSION="23.10.0"
NODE_MSI_URL="https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-x64.msi"

# Create Windows Node.js installer script
echo "Creating install-node.cmd..."
cat > "$WIN_BIN_DIR/install-node.cmd" << 'EOL'
@echo off
setlocal enabledelayedexpansion

REM Check if Node.js is installed in Program Files
if exist "C:\Program Files\nodejs\node.exe" (
    echo Node.js found in Program Files
    set "NODE_EXE=C:\Program Files\nodejs\node.exe"
    goto :found
)

REM Check if Node.js is installed in Program Files (x86)
if exist "C:\Program Files (x86)\nodejs\node.exe" (
    echo Node.js found in Program Files (x86)
    set "NODE_EXE=C:\Program Files (x86)\nodejs\node.exe"
    goto :found
)

echo Node.js not found in standard locations, installing...

REM Download Node.js MSI installer
powershell -Command "& {[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri '%1' -OutFile '%TEMP%\node-setup.msi'}"

REM Install Node.js silently
msiexec /i "%TEMP%\node-setup.msi" /qn

REM Wait a bit for installation to complete
timeout /t 5 /nobreak

REM Clean up
del "%TEMP%\node-setup.msi"

REM Set path to installed Node.js
set "NODE_EXE=C:\Program Files\nodejs\node.exe"

:found
echo Using Node.js: %NODE_EXE%
exit /b 0
EOL

# Create a modified npx.cmd that checks for system Node.js first
echo "Creating npx.cmd..."
cat > "$WIN_BIN_DIR/npx.cmd" << 'EOL'
@ECHO OFF
SETLOCAL EnableDelayedExpansion

SET "SCRIPT_DIR=%~dp0"

REM Try to find Node.js in standard locations first
if exist "C:\Program Files\nodejs\npx.cmd" (
    "C:\Program Files\nodejs\npx.cmd" %*
    exit /b %errorlevel%
)

if exist "C:\Program Files (x86)\nodejs\npx.cmd" (
    "C:\Program Files (x86)\nodejs\npx.cmd" %*
    exit /b %errorlevel%
)

REM If Node.js not found, run installer
call "%SCRIPT_DIR%install-node.cmd" "https://nodejs.org/dist/v23.10.0/node-v23.10.0-x64.msi"
if errorlevel 1 (
    echo Failed to install Node.js
    exit /b 1
)

REM Try using the newly installed Node.js
if exist "C:\Program Files\nodejs\npx.cmd" (
    "C:\Program Files\nodejs\npx.cmd" %*
    exit /b %errorlevel%
)

echo Failed to find npx after Node.js installation
exit /b 1
EOL

echo "Windows npm bundle prepared successfully"
