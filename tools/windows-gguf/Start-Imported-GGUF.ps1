param([Parameter(Mandatory = $true)][string]$Alias)

$ErrorActionPreference = 'Stop'
$support = Split-Path -Parent $PSScriptRoot
$registryPath = Join-Path $support 'models\local-models.json'
$server = Join-Path $support 'llama.cpp\bin\llama-server.exe'
$model = @(Get-Content -Raw -LiteralPath $registryPath | ConvertFrom-Json) |
    Where-Object alias -eq $Alias | Select-Object -First 1
if (-not $model) { throw "Model is not registered: $Alias" }
if (-not (Test-Path -LiteralPath $model.path)) { throw "GGUF file does not exist: $($model.path)" }
if (-not (Test-Path -LiteralPath $server)) { throw "llama-server.exe does not exist: $server" }

foreach ($listener in @(Get-NetTCPConnection -LocalPort 8080 -State Listen -ErrorAction SilentlyContinue)) {
    $process = Get-Process -Id $listener.OwningProcess -ErrorAction SilentlyContinue
    if ($process -and $process.Path -eq $server) { Stop-Process -Id $process.Id -Force }
    elseif ($process) { throw "Port 8080 is used by $($process.ProcessName), PID $($process.Id)." }
}

$arguments = @('-m', [string]$model.path, '--alias', [string]$model.alias,
    '--host', '127.0.0.1', '--port', '8080', '-ngl', [string]$model.gpu_layers,
    '-c', [string]$model.context_size, '--jinja', '--reasoning', 'off')
$stdout = Join-Path $support "logs\$Alias.out.log"
$stderr = Join-Path $support "logs\$Alias.err.log"
Start-Process -FilePath $server -ArgumentList $arguments -WorkingDirectory (Split-Path $server) `
    -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr

for ($attempt = 0; $attempt -lt 180; $attempt++) {
    Start-Sleep -Seconds 1
    try {
        $response = Invoke-RestMethod -Uri 'http://127.0.0.1:8080/v1/models' -TimeoutSec 2
        if ($response.data.id -contains $Alias) { Write-Output "READY: $Alias"; exit 0 }
    } catch {}
}
Write-Error "The model server did not become ready. See: $stderr"
exit 1

