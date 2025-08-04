# Goose Desktop App Builder (PowerShell Version)
# Führe dieses Script als Administrator aus

param(
    [switch]$SkipCleanup,
    [switch]$Verbose
)

# Setze Error Action
$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Green
Write-Host "Goose Desktop App Builder" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""

# Prüfe ob wir im richtigen Verzeichnis sind
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "Fehler: Bitte führe dieses Script im goose-main Verzeichnis aus!" -ForegroundColor Red
    Read-Host "Drücke Enter zum Beenden"
    exit 1
}

try {
    # 1. Bereinige alte Build-Dateien
    Write-Host "[1/8] Bereinige alte Build-Dateien..." -ForegroundColor Yellow
    if (-not $SkipCleanup) {
        $directoriesToClean = @(
            "ui\desktop\out",
            "ui\build-temp", 
            "ui\build-temp2",
            "Goose-Desktop-App",
            "Goose-Desktop-App-Fixed"
        )
        
        foreach ($dir in $directoriesToClean) {
            if (Test-Path $dir) {
                Remove-Item -Path $dir -Recurse -Force -ErrorAction SilentlyContinue
                Write-Host "  Gelöscht: $dir" -ForegroundColor Gray
            }
        }
        
        # Lösche ZIP-Dateien
        Get-ChildItem -Path "." -Filter "Goose-Desktop-App*.zip" | Remove-Item -Force
    }

    # 2. Stoppe laufende Prozesse
    Write-Host "[2/8] Stoppe laufende Prozesse..." -ForegroundColor Yellow
    $processesToKill = @("electron", "goosed", "Goose")
    foreach ($proc in $processesToKill) {
        Get-Process -Name $proc -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Seconds 2

    # 3. Installiere Protobuf-Compiler falls nicht vorhanden
    Write-Host "[3/8] Prüfe Protobuf-Compiler..." -ForegroundColor Yellow
    if (-not (Test-Path "bin\protoc.exe")) {
        Write-Host "Protobuf-Compiler wird heruntergeladen..." -ForegroundColor Cyan
        if (-not (Test-Path "bin")) {
            New-Item -ItemType Directory -Path "bin" | Out-Null
        }
        
        $protocUrl = "https://github.com/protocolbuffers/protobuf/releases/download/v25.3/protoc-25.3-win64.zip"
        $protocZip = "protoc.zip"
        $tempDir = "temp_protoc"
        
        Invoke-WebRequest -Uri $protocUrl -OutFile $protocZip
        Expand-Archive -Path $protocZip -DestinationPath $tempDir -Force
        Move-Item -Path "$tempDir\bin\protoc.exe" -Destination "bin\"
        Remove-Item -Path $tempDir -Recurse -Force
        Remove-Item -Path $protocZip -Force
        
        Write-Host "Protobuf-Compiler installiert!" -ForegroundColor Green
    }

    # Setze Umgebungsvariablen
    $env:PATH = "$PWD\bin;$env:PATH"
    $env:PROTOC = "$PWD\bin\protoc.exe"

    # 4. Baue Backend
    Write-Host "[4/8] Baue Backend (goosed)..." -ForegroundColor Yellow
    cargo build --bin goosed
    if ($LASTEXITCODE -ne 0) {
        throw "Fehler beim Bauen des Backends!"
    }

    # 5. Erstelle temporäres Build-Verzeichnis
    Write-Host "[5/8] Erstelle Build-Umgebung..." -ForegroundColor Yellow
    New-Item -ItemType Directory -Path "ui\build-temp" -Force | Out-Null
    Copy-Item -Path "ui\desktop\*" -Destination "ui\build-temp\" -Recurse -Force
    Set-Location "ui\build-temp"

    # 6. Installiere Dependencies
    Write-Host "[6/8] Installiere Dependencies..." -ForegroundColor Yellow
    npm install
    if ($LASTEXITCODE -ne 0) {
        throw "Fehler beim Installieren der Dependencies!"
    }

    # 7. Generiere API
    npm run generate-api
    if ($LASTEXITCODE -ne 0) {
        throw "Fehler beim Generieren der API!"
    }

    # 8. Baue Main Process
    node scripts/build-main.js
    if ($LASTEXITCODE -ne 0) {
        throw "Fehler beim Bauen des Main Process!"
    }

    # 9. Kopiere goosed.exe in Electron-Ressourcen
    $electronBinPath = "node_modules\electron\dist\resources\bin"
    if (-not (Test-Path $electronBinPath)) {
        New-Item -ItemType Directory -Path $electronBinPath -Force | Out-Null
    }
    Copy-Item -Path "..\..\target\debug\goosed.exe" -Destination $electronBinPath -Force

    # 10. Bereite Platform Binaries vor
    $env:ELECTRON_PLATFORM = "win32"
    node scripts/prepare-platform-binaries.js
    if ($LASTEXITCODE -ne 0) {
        throw "Fehler beim Vorbereiten der Platform Binaries!"
    }

    # 11. Baue Desktop App
    Write-Host "[7/8] Baue Desktop App..." -ForegroundColor Yellow
    npm run make -- --platform=win32 --arch=x64
    if ($LASTEXITCODE -ne 0) {
        throw "Fehler beim Bauen der Desktop App!"
    }

    # 12. Kopiere erstellte App
    Write-Host "[8/8] Kopiere erstellte App..." -ForegroundColor Yellow
    Set-Location "..\.."
    
    if (Test-Path "ui\build-temp\out\Goose-win32-x64") {
        Copy-Item -Path "ui\build-temp\out\Goose-win32-x64" -Destination "Goose-Desktop-App" -Recurse -Force
        Copy-Item -Path "ui\build-temp\out\make\zip\win32\x64\Goose-win32-x64-1.1.0.zip" -Destination "Goose-Desktop-App.zip" -Force
        
        # Stelle sicher, dass goosed.exe vorhanden ist
        if (-not (Test-Path "Goose-Desktop-App\resources\bin\goosed.exe")) {
            Copy-Item -Path "target\debug\goosed.exe" -Destination "Goose-Desktop-App\resources\bin\" -Force
        }
        
        Write-Host ""
        Write-Host "========================================" -ForegroundColor Green
        Write-Host "Build erfolgreich abgeschlossen!" -ForegroundColor Green
        Write-Host "========================================" -ForegroundColor Green
        Write-Host ""
        Write-Host "Erstellte Dateien:" -ForegroundColor Cyan
        Write-Host "- Goose-Desktop-App\ (Verzeichnis)" -ForegroundColor White
        Write-Host "- Goose-Desktop-App.zip (ZIP-Datei)" -ForegroundColor White
        Write-Host ""
        Write-Host "Um die App zu starten:" -ForegroundColor Cyan
        Write-Host "- Doppelklick auf Goose-Desktop-App\Goose.exe" -ForegroundColor White
        Write-Host "- Oder entpacke Goose-Desktop-App.zip" -ForegroundColor White
        Write-Host ""
        
    } else {
        throw "Fehler: Desktop App wurde nicht erstellt!"
    }

    # 13. Räume auf
    if (Test-Path "ui\build-temp") {
        Remove-Item -Path "ui\build-temp" -Recurse -Force
    }

    Write-Host "Build abgeschlossen! Drücke Enter zum Beenden..." -ForegroundColor Green
    Read-Host

} catch {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Red
    Write-Host "Fehler beim Build!" -ForegroundColor Red
    Write-Host "========================================" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    Write-Host ""
    Write-Host "Drücke Enter zum Beenden..." -ForegroundColor Yellow
    Read-Host
    exit 1
} 