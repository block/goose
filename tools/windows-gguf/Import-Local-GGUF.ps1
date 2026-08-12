param([string]$ModelPath)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName Microsoft.VisualBasic

function Write-JsonUtf8NoBom {
    param([Parameter(Mandatory = $true)]$Value, [Parameter(Mandatory = $true)][string]$Path, [int]$Depth = 10)
    $json = $Value | ConvertTo-Json -Depth $Depth
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $json, $utf8NoBom)
}

$support = Split-Path -Parent $PSScriptRoot
$registryPath = Join-Path $support 'models\local-models.json'
$providerPath = Join-Path $env:APPDATA 'Block\goose\config\custom_providers\local_llamacpp.json'

if (-not $ModelPath) {
    $picker = New-Object System.Windows.Forms.OpenFileDialog
    $picker.Title = 'Select a GGUF model to import'
    $picker.Filter = 'GGUF model (*.gguf)|*.gguf|All files (*.*)|*.*'
    if ($picker.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) { exit 0 }
    $ModelPath = $picker.FileName
}

$file = Get-Item -LiteralPath $ModelPath
if ($file.Extension -ine '.gguf') { throw 'Select a .gguf model file.' }
$suggested = $file.BaseName.ToLowerInvariant() -replace '[^a-z0-9._-]+', '-'
$alias = [Microsoft.VisualBasic.Interaction]::InputBox(
    'Enter the model name shown in Goose. Use letters, digits, dots, underscores, and hyphens.',
    'Model name', $suggested
).Trim()
if (-not $alias) { exit 0 }
if ($alias -notmatch '^[A-Za-z0-9._-]+$') { throw 'The model name contains unsupported characters.' }

$defaultLayers = if ($file.Length -le 8GB) { 99 } else { 20 }
$layersText = [Microsoft.VisualBasic.Interaction]::InputBox(
    'Enter GPU layers. For 8 GB VRAM, use 99 for small models and start with 20 for large models.',
    'GPU layers', [string]$defaultLayers
).Trim()
$contextText = [Microsoft.VisualBasic.Interaction]::InputBox(
    'Enter the context size. Reduce it if VRAM is insufficient.',
    'Context size', '8192'
).Trim()
$layers = 0
$context = 0
if (-not [int]::TryParse($layersText, [ref]$layers) -or $layers -lt 0) { throw 'GPU layers must be a non-negative integer.' }
if (-not [int]::TryParse($contextText, [ref]$context) -or $context -lt 512) { throw 'Context size must be at least 512.' }

$entries = @()
if (Test-Path -LiteralPath $registryPath) {
    $loaded = Get-Content -Raw -LiteralPath $registryPath | ConvertFrom-Json
    if ($loaded) { $entries = @($loaded) }
}
$entries = @($entries | Where-Object alias -ne $alias)
$entries += [pscustomobject]@{
    alias = $alias; path = $file.FullName; gpu_layers = $layers
    context_size = $context; imported_at = (Get-Date).ToString('o')
}
Write-JsonUtf8NoBom -Value $entries -Path $registryPath -Depth 5

$providerDirectory = Split-Path -Parent $providerPath
New-Item -ItemType Directory -Force -Path $providerDirectory | Out-Null
if (Test-Path -LiteralPath $providerPath) {
    $provider = Get-Content -Raw -LiteralPath $providerPath | ConvertFrom-Json
    Copy-Item -LiteralPath $providerPath -Destination "$providerPath.$(Get-Date -Format 'yyyyMMdd-HHmmss').bak"
} else {
    $provider = [pscustomobject]@{
        name = 'local_llamacpp'; engine = 'openai'; display_name = 'Local GGUF (llama.cpp)'
        description = 'Local GGUF models served by llama.cpp'; api_key_env = ''
        base_url = 'http://127.0.0.1:8080/v1/chat/completions'; models = @()
        headers = $null; timeout_seconds = $null; supports_streaming = $true
        requires_auth = $false; catalog_provider_id = $null; base_path = $null
        env_vars = $null; dynamic_models = $null; skip_canonical_filtering = $false
        model_doc_link = $null; setup_steps = @(); fast_model = $null; preserves_thinking = $true
    }
}
if ($null -eq $provider.models) {
    $provider.models = @()
}
if (-not ($provider.models | Where-Object name -eq $alias)) {
    $provider.models += [pscustomobject]@{
        name = $alias; context_limit = $context; input_token_cost = $null
        output_token_cost = $null; currency = $null; supports_cache_control = $null
        reasoning = $false
    }
}
Write-JsonUtf8NoBom -Value $provider -Path $providerPath -Depth 10

& (Join-Path $PSScriptRoot 'Start-Imported-GGUF.ps1') -Alias $alias
if ($LASTEXITCODE -ne 0) { throw 'The model was registered, but llama.cpp failed to become ready.' }
[System.Windows.Forms.MessageBox]::Show(
    "Imported and started: $alias`r`n`r`nRestart Goose and select Local GGUF (llama.cpp).",
    'GGUF import complete'
) | Out-Null
