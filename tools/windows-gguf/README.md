# Windows GGUF importer for Goose

This optional helper registers an existing `.gguf` file with an
OpenAI-compatible `llama.cpp` server and Goose. It does not copy or redistribute
the model file.

## Requirements

- Windows PowerShell 5.1 or newer
- An existing Goose Desktop installation
- `llama-server.exe` from llama.cpp
- A GGUF model whose license permits your intended use

## Install

Run:

```powershell
powershell -ExecutionPolicy Bypass -File .\Install.ps1
```

Select the Goose support directory when prompted. The recommended layout is:

```text
<support directory>/
  llama.cpp/bin/llama-server.exe
  models/
  logs/
  scripts/
```

The installer creates `Import-Local-GGUF.cmd`. Double-click it to select and
register a model.

## What it changes

- Creates or updates `<support directory>/models/local-models.json`.
- Creates or updates the Goose custom provider `local_llamacpp` in the current
  user's Goose configuration.
- Backs up the provider JSON before changing an existing file.
- Starts one local llama.cpp model on `127.0.0.1:8080`.

The importer never uploads or copies the GGUF file. Only its local absolute path
is stored in the local registry. Do not commit the generated registry because it
contains machine-specific paths.

## Goose settings

After importing a model, restart Goose and select:

```text
Provider: Local GGUF (llama.cpp)
Model: the alias entered during import
```

Only one imported model is served at a time because all entries use port 8080.

