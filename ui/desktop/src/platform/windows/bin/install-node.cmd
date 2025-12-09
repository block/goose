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
    echo "Node.js found in Program Files (x86)"
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
