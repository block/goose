@echo off
setlocal enabledelayedexpansion

rem Script to prepare Windows npm bundle
set "SCRIPT_DIR=%~dp0"
set "PLATFORM_WIN_DIR=%SCRIPT_DIR%\..\src\platform\windows"
set "WIN_BIN_DIR=%PLATFORM_WIN_DIR%\bin"
set "DEST_BIN_DIR=%SCRIPT_DIR%\..\src\bin"

echo Preparing Windows npm bundle...
echo SCRIPT_DIR: %SCRIPT_DIR%
echo PLATFORM_WIN_DIR: %PLATFORM_WIN_DIR%
echo WIN_BIN_DIR: %WIN_BIN_DIR%
echo DEST_BIN_DIR: %DEST_BIN_DIR%

rem Ensure directories exist
if not exist "%WIN_BIN_DIR%" mkdir "%WIN_BIN_DIR%"

rem Node.js version and paths
set "NODE_VERSION=23.10.0"
set "NODE_MSI_URL=https://nodejs.org/dist/v%NODE_VERSION%/node-v%NODE_VERSION%-x64.msi"

rem Create Windows Node.js installer script
echo Creating install-node.cmd...
(
echo @echo off
echo setlocal enabledelayedexpansion
echo.
echo REM Check if Node.js is installed in Program Files
echo if exist "C:\Program Files\nodejs\node.exe" ^(
echo     echo Node.js found in Program Files
echo     set "NODE_EXE=C:\Program Files\nodejs\node.exe"
echo     goto :found
echo ^)
echo.
echo REM Check if Node.js is installed in Program Files ^(x86^)
echo if exist "C:\Program Files ^(x86^)\nodejs\node.exe" ^(
echo     echo Node.js found in Program Files ^(x86^)
echo     set "NODE_EXE=C:\Program Files ^(x86^)\nodejs\node.exe"
echo     goto :found
echo ^)
echo.
echo echo Node.js not found in standard locations, installing...
echo.
echo REM Download Node.js MSI installer
echo powershell -Command "^& {[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri '%%1' -OutFile '%%TEMP%%\node-setup.msi'}"
echo.
echo REM Install Node.js silently
echo msiexec /i "%%TEMP%%\node-setup.msi" /qn
echo.
echo REM Wait a bit for installation to complete
echo timeout /t 5 /nobreak
echo.
echo REM Clean up
echo del "%%TEMP%%\node-setup.msi"
echo.
echo REM Set path to installed Node.js
echo set "NODE_EXE=C:\Program Files\nodejs\node.exe"
echo.
echo :found
echo echo Using Node.js: %%NODE_EXE%%
echo exit /b 0
) > "%WIN_BIN_DIR%\install-node.cmd"

rem Create a modified npx.cmd that checks for system Node.js first
echo Creating npx.cmd...
(
echo @ECHO OFF
echo SETLOCAL EnableDelayedExpansion
echo.
echo SET "SCRIPT_DIR=%%~dp0"
echo.
echo REM Try to find Node.js in standard locations first
echo if exist "C:\Program Files\nodejs\npx.cmd" ^(
echo     "C:\Program Files\nodejs\npx.cmd" %%*
echo     exit /b %%errorlevel%%
echo ^)
echo.
echo if exist "C:\Program Files ^(x86^)\nodejs\npx.cmd" ^(
echo     "C:\Program Files ^(x86^)\nodejs\npx.cmd" %%*
echo     exit /b %%errorlevel%%
echo ^)
echo.
echo REM If Node.js not found, run installer
echo call "%%SCRIPT_DIR%%install-node.cmd" "%NODE_MSI_URL%"
echo if errorlevel 1 ^(
echo     echo Failed to install Node.js
echo     exit /b 1
echo ^)
echo.
echo REM Try using the newly installed Node.js
echo if exist "C:\Program Files\nodejs\npx.cmd" ^(
echo     "C:\Program Files\nodejs\npx.cmd" %%*
echo     exit /b %%errorlevel%%
echo ^)
echo.
echo echo Failed to find npx after Node.js installation
echo exit /b 1
) > "%WIN_BIN_DIR%\npx.cmd"

echo Windows npm bundle prepared successfully