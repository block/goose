#!/usr/bin/env python3
"""Aggregate per-extension mcp-scanner output into summary.json + summary.md.

Reads <out-dir>/raw/<id>.json files (raw scanner JSON, or one of our
{"_scan_status": ...} markers) and produces:

  <out-dir>/summary.json  machine-readable verdict
  <out-dir>/summary.md    PR comment body

The raw scanner schema can vary across analyzers/versions, so severities are
discovered by walking the JSON for any "severity" string and any is_safe=false.
"""

import json
import os
import sys

SEVERITY_ORDER = ["NONE", "LOW", "MEDIUM", "HIGH", "CRITICAL"]


def normalize_block_severity(value):
    value = (value or "HIGH").upper()
    return value if value in SEVERITY_ORDER[1:] else "HIGH"


BLOCK_SEVERITY = normalize_block_severity(os.environ.get("BLOCK_SEVERITY"))


def severity_rank(sev):
    sev = (sev or "NONE").upper()
    return SEVERITY_ORDER.index(sev) if sev in SEVERITY_ORDER else 0


def walk_severities(node, found):
    """Collect severity strings and unsafe flags from an arbitrary JSON tree."""
    if isinstance(node, dict):
        for key, value in node.items():
            lk = key.lower()
            if lk == "severity" and isinstance(value, str):
                found["severities"].append(value.upper())
            if lk in ("is_safe", "safe") and value is False:
                found["unsafe"] = True
            if lk == "threat_names" and value:
                if isinstance(value, list):
                    found["threats"].extend(str(name) for name in value if name)
                else:
                    found["threats"].append(str(value))
            if lk == "threat_summary" and value:
                summary = str(value)
                if summary.lower() != "no threats detected":
                    found["threats"].append(summary)
            walk_severities(value, found)
    elif isinstance(node, list):
        for item in node:
            walk_severities(item, found)


def summarize_entry(entry_id, raw_path):
    try:
        with open(raw_path, "r") as fh:
            data = json.load(fh)
    except Exception as exc:  # noqa: BLE001
        return {"id": entry_id, "status": "error", "severity": "NONE",
                "detail": f"unreadable output: {exc}"}

    if isinstance(data, dict) and "_scan_status" in data:
        status = data["_scan_status"]
        detail = data.get("_reason") or (
            f"exit {data['_exit_code']}" if "_exit_code" in data else status
        )
        return {"id": entry_id, "status": status, "severity": "NONE", "detail": detail}

    found = {"severities": [], "unsafe": False, "threats": []}
    walk_severities(data, found)

    max_sev = "NONE"
    for sev in found["severities"]:
        if severity_rank(sev) > severity_rank(max_sev):
            max_sev = sev
    if found["unsafe"] and severity_rank(max_sev) == 0:
        max_sev = "MEDIUM"

    return {
        "id": entry_id,
        "status": "scanned",
        "severity": max_sev,
        "unsafe": found["unsafe"],
        "threats": sorted(set(found["threats"]))[:10],
    }


def main():
    out_dir = sys.argv[1]
    raw_dir = os.path.join(out_dir, "raw")
    results = []
    if os.path.isdir(raw_dir):
        for name in sorted(os.listdir(raw_dir)):
            if not name.endswith(".json"):
                continue
            entry_id = name[:-len(".json")]
            results.append(summarize_entry(entry_id, os.path.join(raw_dir, name)))

    block_rank = severity_rank(BLOCK_SEVERITY)
    blocked = [r for r in results if severity_rank(r["severity"]) >= block_rank
               and r["status"] == "scanned"]
    inconclusive = [r for r in results if r["status"] in ("error", "timeout")]

    if blocked or inconclusive:
        overall = "BLOCKED"
    elif results and all(r["status"] == "skipped" for r in results):
        overall = "SKIPPED"
    else:
        overall = "APPROVED"

    summary = {
        "overall_status": overall,
        "block_severity": BLOCK_SEVERITY,
        "scanned": len(results),
        "blocked": len(blocked),
        "inconclusive": len(inconclusive),
        "results": results,
    }
    with open(os.path.join(out_dir, "summary.json"), "w") as fh:
        json.dump(summary, fh, indent=2)

    write_markdown(out_dir, summary)
    print(f"Overall: {overall} | scanned={len(results)} blocked={len(blocked)} "
          f"inconclusive={len(inconclusive)}")


def write_markdown(out_dir, summary):
    status_label = {
        "APPROVED": "PASSED",
        "BLOCKED": "BLOCKED",
        "SKIPPED": "SKIPPED",
    }

    lines = ["## Extension Security Scan", ""]
    lines.append(f"**Status: {status_label.get(summary['overall_status'], 'UNKNOWN')}** "
                 f"(blocks at {summary['block_severity']}+)")
    lines.append("")
    lines.append(f"- Scanned: {summary['scanned']}")
    if summary["blocked"]:
        lines.append(f"- Blocked: {summary['blocked']}")
    if summary["inconclusive"]:
        lines.append(f"- Inconclusive (could not scan): {summary['inconclusive']}")
    lines.append("")

    if summary["results"]:
        lines.append("| Extension | Result | Max severity | Notes |")
        lines.append("| --- | --- | --- | --- |")
        for r in summary["results"]:
            if r["status"] == "scanned":
                result = r["severity"]
                notes = ", ".join(r.get("threats", [])) or "-"
            else:
                result = r["status"]
                notes = r.get("detail", "")
            lines.append(f"| `{r['id']}` | {result} | {r.get('severity', 'NONE')} | {notes} |")
        lines.append("")

    if summary["overall_status"] == "BLOCKED":
        if summary["blocked"] and summary["inconclusive"]:
            lines.append("> One or more extensions returned a "
                         f"{summary['block_severity']}+ finding, and one or more "
                         "extensions could not be scanned. These must be resolved "
                         "before merge. Maintainers may override after review.")
        elif summary["blocked"]:
            lines.append("> One or more extensions returned a "
                         f"{summary['block_severity']}+ finding. This must be resolved "
                         "before merge. Maintainers may override after review.")
        else:
            lines.append("> One or more extensions could not be scanned. This must be "
                         "resolved before merge. Maintainers may override after review.")
    lines.append("")
    lines.append("_Scanned with the "
                 "[Cisco AI Defense MCP Scanner](https://github.com/cisco-ai-defense/mcp-scanner). "
                 "Inconclusive results often mean the server needs real credentials to start "
                 "and now block the PR gate until reviewed._")

    with open(os.path.join(out_dir, "summary.md"), "w") as fh:
        fh.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
