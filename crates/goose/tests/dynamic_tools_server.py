#!/usr/bin/env python3
"""Minimal stdlib-only MCP stdio server that reproduces the dropped
`notifications/tools/list_changed` bug in goose.

Behavior
--------
- Advertises `capabilities.tools.listChanged = true` at initialize.
- `tools/list` returns a single tool, `alpha`, until it has been "unlocked".
- The first time `alpha` is called, the server (a) sends an unsolicited
  `notifications/tools/list_changed` to the client and (b) starts returning a
  second tool, `beta`, from `tools/list`.

Expected client behavior (per MCP spec): on receiving
`notifications/tools/list_changed`, the client re-fetches `tools/list` and
`beta` becomes available on the next turn — without restarting.

Buggy behavior (goose before the fix): the notification is dropped, the tool
list stays cached at init, and `beta` never appears until the app is restarted.

Usage (as a goose Stdio extension):
    command: python3
    args: ["/abs/path/to/dynamic_tools_server.py"]

No third-party dependencies; Python 3.8+.
"""
from __future__ import annotations

import json
import sys
import threading

_unlocked = threading.Event()
_out_lock = threading.Lock()


def _send(obj: dict) -> None:
    with _out_lock:
        sys.stdout.write(json.dumps(obj) + "\n")
        sys.stdout.flush()


def _tools() -> list:
    alpha = {
        "name": "alpha",
        "description": "Always present. Call me once to unlock beta.",
        "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False},
    }
    beta = {
        "name": "beta",
        "description": "Only appears after alpha is called + tools/list is re-fetched.",
        "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False},
    }
    return [alpha, beta] if _unlocked.is_set() else [alpha]


def _handle(msg: dict) -> None:
    method = msg.get("method")
    msg_id = msg.get("id")

    if method == "initialize":
        _send({
            "jsonrpc": "2.0",
            "id": msg_id,
            "result": {
                "protocolVersion": msg.get("params", {}).get("protocolVersion", "2025-03-26"),
                "capabilities": {"tools": {"listChanged": True}},
                "serverInfo": {"name": "dynamic-tools", "version": "0.1.0"},
            },
        })
        return

    if method == "notifications/initialized" or (method or "").startswith("notifications/"):
        return

    if method == "tools/list":
        _send({"jsonrpc": "2.0", "id": msg_id, "result": {"tools": _tools()}})
        return

    if method in ("resources/list", "prompts/list"):
        key = method.split("/")[0]
        _send({"jsonrpc": "2.0", "id": msg_id, "result": {key: []}})
        return

    if method == "tools/call":
        name = (msg.get("params") or {}).get("name")
        if name == "alpha" and not _unlocked.is_set():
            _unlocked.set()
            # Tell the client the tool set changed. A spec-compliant client
            # re-fetches tools/list and picks up `beta`.
            _send({"jsonrpc": "2.0", "method": "notifications/tools/list_changed"})
        text = (
            "alpha called; beta is now published (a spec-compliant client will "
            "re-fetch tools/list)."
            if name == "alpha"
            else f"{name} called."
        )
        _send({
            "jsonrpc": "2.0",
            "id": msg_id,
            "result": {"content": [{"type": "text", "text": text}], "isError": False},
        })
        return

    if msg_id is not None:
        _send({"jsonrpc": "2.0", "id": msg_id, "error": {"code": -32601, "message": f"unknown method {method}"}})


def main() -> int:
    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except Exception:
            continue
        if isinstance(msg, dict):
            _handle(msg)
    return 0


if __name__ == "__main__":
    sys.exit(main())
