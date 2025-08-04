@echo off
setlocal enabledelayedexpansion

echo ========================================
echo Goose Desktop App Builder
echo ========================================
echo.

:: Prüfe ob wir im richtigen Verzeichnis sind
if not exist "Cargo.toml" (
    echo Fehler: Bitte führe dieses Script im goose-main Verzeichnis aus!
    pause
    exit /b 1
)

echo [1/8] Bereinige alte Build-Dateien...
if exist "ui\desktop\out" (
    rmdir /s /q "ui\desktop\out" 2>nul
)
if exist "ui\build-temp" (
    rmdir /s /q "ui\build-temp" 2>nul
)
if exist "ui\build-temp2" (
    rmdir /s /q "ui\build-temp2" 2>nul
)
if exist "Goose-Desktop-App" (
    rmdir /s /q "Goose-Desktop-App" 2>nul
)
if exist "Goose-Desktop-App-Fixed" (
    rmdir /s /q "Goose-Desktop-App-Fixed" 2>nul
)
if exist "Goose-Desktop-App*.zip" (
    del "Goose-Desktop-App*.zip" 2>nul
)

:: Stoppe laufende Prozesse
echo [2/8] Stoppe laufende Prozesse...
taskkill /f /im electron.exe 2>nul
taskkill /f /im goosed.exe 2>nul
taskkill /f /im Goose.exe 2>nul
timeout /t 2 /nobreak >nul

:: Installiere Protobuf-Compiler falls nicht vorhanden
echo [3/8] Prüfe Protobuf-Compiler...
if not exist "bin\protoc.exe" (
    echo Protobuf-Compiler wird heruntergeladen...
    if not exist "bin" mkdir bin
    powershell -Command "Invoke-WebRequest -Uri 'https://github.com/protocolbuffers/protobuf/releases/download/v25.3/protoc-25.3-win64.zip' -OutFile 'protoc.zip'"
    powershell -Command "Expand-Archive -Path 'protoc.zip' -DestinationPath 'temp_protoc' -Force"
    move "temp_protoc\bin\protoc.exe" "bin\"
    rmdir /s /q "temp_protoc"
    del "protoc.zip"
    echo Protobuf-Compiler installiert!
)

:: Setze Umgebungsvariablen
set "PATH=%CD%\bin;%PATH%"
set "PROTOC=%CD%\bin\protoc.exe"

:: Baue Backend
echo [4/8] Baue Backend (goosed)...
cargo build --bin goosed
if errorlevel 1 (
    echo Fehler beim Bauen des Backends!
    pause
    exit /b 1
)

:: Erstelle temporäres Build-Verzeichnis
echo [5/8] Erstelle Build-Umgebung...
mkdir "ui\build-temp"
xcopy "ui\desktop\*" "ui\build-temp\" /E /I /Y
cd "ui\build-temp"

:: Installiere Dependencies
echo [6/8] Installiere Dependencies...
call npm install
if errorlevel 1 (
    echo Fehler beim Installieren der Dependencies!
    cd ..\..
    pause
    exit /b 1
)

:: Generiere API
call npm run generate-api
if errorlevel 1 (
    echo Fehler beim Generieren der API!
    cd ..\..
    pause
    exit /b 1
)

:: Baue Main Process
node scripts/build-main.js
if errorlevel 1 (
    echo Fehler beim Bauen des Main Process!
    cd ..\..
    pause
    exit /b 1
)

:: Kopiere goosed.exe in Electron-Ressourcen
if not exist "node_modules\electron\dist\resources\bin" (
    mkdir "node_modules\electron\dist\resources\bin"
)
copy "..\..\target\debug\goosed.exe" "node_modules\electron\dist\resources\bin\"

:: Bereite Platform Binaries vor
set "ELECTRON_PLATFORM=win32"
node scripts/prepare-platform-binaries.js
if errorlevel 1 (
    echo Fehler beim Vorbereiten der Platform Binaries!
    cd ..\..
    pause
    exit /b 1
)

:: Baue Desktop App
echo [7/8] Baue Desktop App...
call npm run make -- --platform=win32 --arch=x64
if errorlevel 1 (
    echo Fehler beim Bauen der Desktop App!
    cd ..\..
    pause
    exit /b 1
)

:: Kopiere erstellte App
echo [8/8] Kopiere erstellte App...
cd ..\..
if exist "ui\build-temp\out\Goose-win32-x64" (
    xcopy "ui\build-temp\out\Goose-win32-x64" "Goose-Desktop-App\" /E /I /Y
    copy "ui\build-temp\out\make\zip\win32\x64\Goose-win32-x64-1.1.0.zip" "Goose-Desktop-App.zip"
    
    :: Stelle sicher, dass goosed.exe vorhanden ist
    if not exist "Goose-Desktop-App\resources\bin\goosed.exe" (
        copy "target\debug\goosed.exe" "Goose-Desktop-App\resources\bin\"
    )
    
    echo.
    echo ========================================
    echo Build erfolgreich abgeschlossen!
    echo ========================================
    echo.
    echo Erstellte Dateien:
    echo - Goose-Desktop-App\ (Verzeichnis)
    echo - Goose-Desktop-App.zip (ZIP-Datei)
    echo.
    echo Um die App zu starten:
    echo - Doppelklick auf Goose-Desktop-App\Goose.exe
    echo - Oder entpacke Goose-Desktop-App.zip
    echo.
) else (
    echo Fehler: Desktop App wurde nicht erstellt!
    pause
    exit /b 1
)

:: Räume auf
rmdir /s /q "ui\build-temp"

echo Build abgeschlossen! Drücke eine beliebige Taste zum Beenden...
pause >nul 