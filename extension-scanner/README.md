# Extension security scanner

Scans goose catalog extensions (`documentation/static/servers.json`) with the
[Cisco AI Defense MCP Scanner](https://github.com/cisco-ai-defense/mcp-scanner)
for tool poisoning, prompt injection, and other MCP threats.

Two workflows use the shared script in this directory:

- **`extension-security-scan.yml`** - submission gate. Scans only the entries
  added/changed in a PR and blocks merge on a `HIGH`+ finding.
- **`extension-catalog-audit.yml`** - scheduled audit. Re-scans the entire
  catalog and opens a tracking issue when something regresses.

## Files

- `scan-extensions.sh` - orchestrates per-entry scans and aggregation.
- `summarize.py` - aggregates raw scanner output into `summary.json` (verdict)
  and `summary.md` (PR/issue body).

## Local usage

```sh
uv tool install --python 3.13 cisco-ai-mcp-scanner

# Scan the whole catalog
bash extension-scanner/scan-extensions.sh documentation/static/servers.json /tmp/scan

# Scan specific ids
printf 'agentql-mcp\nbeads\n' > /tmp/ids.txt
bash extension-scanner/scan-extensions.sh documentation/static/servers.json /tmp/scan /tmp/ids.txt

cat /tmp/scan/summary.md
```

## Configuration

| Variable | Default | Notes |
| --- | --- | --- |
| `ANALYZERS` | `yara` | Comma-separated mcp-scanner analyzers. YARA needs no API keys. |
| `BLOCK_SEVERITY` | `HIGH` | Minimum severity that marks the run `BLOCKED`. |
| `SCAN_TIMEOUT` | `180` | Per-entry timeout (seconds). |
| `STDIO_TIMEOUT` | `120` | mcp-scanner stdio startup timeout (seconds). |

## Security note

Scanning a stdio server launches third-party code (`npx`/`uvx`). The submission
gate therefore runs in the untrusted `pull_request` context with **no secrets**,
so the default analyzer is the key-less YARA engine. Only enable LLM/API
analyzers in a context where no untrusted code runs with the key in scope (e.g.
the periodic audit over already-merged entries).
