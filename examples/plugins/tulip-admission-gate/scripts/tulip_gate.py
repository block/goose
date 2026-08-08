#!/usr/bin/env python3
"""Goose PreToolUse hook: gate tool calls through a Tulip-style admission
decision, backed by any tulip-compatible model.

Real contract, traced to Goose's own Rust source
(crates/goose/src/hooks/mod.rs), not assumed from docs:

  input (stdin JSON):  {event, session_id, matcher_context, tool_name,
                         tool_input, tool_output, message,
                         last_assistant_message, working_dir}
  deny:                exit code 2, reason on stderr
                        OR stdout {"decision": "block", "reason": "..."}
  allow:                anything else -- including a crashed/timed-out hook,
                        which Goose's own HookManager::emit_blocking treats
                        as Allow by design ("a misbehaving hook MUST NOT
                        block"). Stated here, not hidden: this script cannot
                        protect against its own subprocess dying before it
                        prints anything -- only against a live, working
                        model returning a real verdict.

Goose's hook decision space is BINARY (Allow / Deny) -- there is no
"ask"/pending-approval state the way Claude Code's PreToolUse or the tulip
verdict space has. `require_human` is mapped to `deny` here, with a reason
that says so explicitly, rather than silently downgrading to allow. Same
honest limitation already documented for the ADK plugin in this integration
work: it short-circuits the call rather than pausing for approval.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys

POLICY = (
    "Before executing any action that could destroy data, irreversibly alter git "
    "history, or expose credentials -- including deleting files, force-pushing, "
    "resetting a shared branch, or reading a private key or secrets file -- you "
    "must obtain explicit user confirmation. Read-only, build, test, lint, and "
    "search commands do not require confirmation."
)

SYSTEM_PROMPT = (
    "You are an admission gate for an AI agent. Given a written policy and a proposed "
    "action, decide what the policy requires:\n"
    "  allow          — the policy permits this action to proceed\n"
    "  require_human  — the policy requires a person to approve before it proceeds\n"
    "  deny           — the policy forbids this action\n"
    "Answer with exactly one of those three words and nothing else."
)

VERDICTS = {"allow", "require_human", "deny"}

GATE_SSH_HOST = os.environ.get("TULIP_GATE_SSH_HOST", "fedespark-lan")
GATE_URL = os.environ.get("TULIP_GATE_URL", "http://127.0.0.1:8010/v1/chat/completions")
GATE_MODEL = os.environ.get("TULIP_GATE_MODEL", "clusiana-admit-v4")


def classify(action_text: str) -> tuple[str, str]:
    payload = json.dumps(
        {
            "model": GATE_MODEL,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {
                    "role": "user",
                    "content": f"POLICY:\n{POLICY}\n\nPROPOSED ACTION:\n{action_text}\n\nVerdict?",
                },
            ],
            "max_tokens": 6,
            "temperature": 0,
            "chat_template_kwargs": {"enable_thinking": False},
        }
    )
    curl_cmd = f"curl -s -X POST {GATE_URL} -H 'Content-Type: application/json' --data-binary @- --max-time 15"
    try:
        result = subprocess.run(
            ["ssh", "-o", "ConnectTimeout=5", GATE_SSH_HOST, curl_cmd],
            input=payload,
            capture_output=True,
            text=True,
            timeout=18,
        )
        response = json.loads(result.stdout)
        text = str(response["choices"][0]["message"]["content"]).strip()
        predicted = text.split()[0].rstrip(".,:") if text.split() else text
        if predicted not in VERDICTS:
            return "require_human", f"off-schema response: {text!r}"
        return predicted, text
    except Exception as exc:  # noqa: BLE001 -- fail closed within this process
        return "require_human", f"gate unreachable ({exc}) -- failing closed"


def main() -> None:
    raw = sys.stdin.read()
    try:
        ctx = json.loads(raw)
    except json.JSONDecodeError:
        print(json.dumps({"decision": "block", "reason": "tulip gate: malformed hook input, failing closed"}))
        return

    tool_name = ctx.get("tool_name") or ""
    tool_input = ctx.get("tool_input") or {}
    action_text = f"{tool_name}({json.dumps(tool_input)})"

    verdict, raw_text = classify(action_text)

    if verdict == "allow":
        # No output, exit 0 -- Allow.
        return

    reason = f"tulip admission gate: verdict={verdict!r} raw={raw_text!r}"
    print(json.dumps({"decision": "block", "reason": reason}))


if __name__ == "__main__":
    main()
