param(
    [string]$SupportDirectory
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms

if (-not $SupportDirectory) {
    $picker = New-Object System.Windows.Forms.FolderBrowserDialog
    $picker.Description = 'Select the Goose support directory containing llama.cpp and models'
    $picker.ShowNewFolderButton = $true
    if ($picker.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) { exit 0 }
    $SupportDirectory = $picker.SelectedPath
}

$support = [System.IO.Path]::GetFullPath($SupportDirectory)
$scripts = Join-Path $support 'scripts'
$models = Join-Path $support 'models'
$logs = Join-Path $support 'logs'
New-Item -ItemType Directory -Force -Path $scripts, $models, $logs | Out-Null

Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Import-Local-GGUF.ps1') -Destination $scripts -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Start-Imported-GGUF.ps1') -Destination $scripts -Force

$launcher = @"
@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\Import-Local-GGUF.ps1" %*
if errorlevel 1 pause
"@
Set-Content -LiteralPath (Join-Path $support 'Import-Local-GGUF.cmd') -Value $launcher -Encoding ASCII

[System.Windows.Forms.MessageBox]::Show(
    "Installed GGUF tools in: $support",
    'Installation complete'
) | Out-Null

