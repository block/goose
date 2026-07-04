---
title: Debug Desktop Startup Failures
sidebar_label: Debug Desktop Startup Failures
description: Find the desktop startup diagnostics log, understand the key fields, and share the right artifacts when gosling fails to start.
---

When gosling Desktop fails before the backend becomes ready, the normal server log may be empty or incomplete. In that case, the most useful artifact is the startup diagnostics JSON written by the desktop app.

## Find the Startup Diagnostics Log

gosling Desktop writes one startup diagnostics file per launch attempt.

Typical locations:

- macOS: `~/Library/Application Support/Gosling/logs/startup/`
- Windows: `%APPDATA%\Gosling\logs\startup\`
- Linux: `~/.config/Gosling/logs/startup/`

The files are named like:

```text
goslingd-startup-2026-04-21T01-24-03.149Z-23416.json
```

If several files exist, use the newest one.

## What To Share

When reporting a desktop startup failure, share:

- the newest `goslingd-startup-*.json`
- your gosling version
- your operating system and version

For Windows native crashes, also attach the Windows crash report for `goslingd.exe` if available.

Common places to find the Windows crash report:

- Event Viewer: `Windows Logs` → `Application`
- Reliability Monitor: `View technical details`
- WER files on disk:
  - `%LOCALAPPDATA%\Microsoft\Windows\WER\ReportArchive\`
  - `%LOCALAPPDATA%\Microsoft\Windows\WER\ReportQueue\`

Look for a `Report.wer` related to `goslingd.exe`.

If you are filing a GitHub issue or asking for support, this is usually enough:

- the newest `goslingd-startup-*.json`
- your gosling version
- your operating system and version
- on Windows, `Report.wer` for `goslingd.exe` if Windows created one

## What The Startup Log Contains

In most cases, sharing the newest startup log is enough.

If you want a quick high-level read, focus on these fields:

- `childExitCode` or `childExitSignal`
  Shows whether the backend process exited during startup.
- `certFingerprintSeen`
  Shows whether the backend reached the TLS startup stage.
- `healthCheckSucceeded`
  Shows whether the desktop app ever observed the backend as ready.
- `stderrTail`
  Shows the most recent startup output captured from the backend, including major startup stage markers when available.
- `events`
  Shows the order of major startup steps like process spawn, health check, and child exit.

## Related Diagnostics

For session or in-app issues after gosling has started, use the normal diagnostics bundle described in [Diagnostics and Reporting](/docs/troubleshooting/diagnostics-and-reporting).
