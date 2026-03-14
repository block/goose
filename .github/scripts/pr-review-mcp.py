#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["mcp"]
# ///
"""MCP server for collecting PR review comments and conclusion."""

import json
from pathlib import Path

from mcp.server.fastmcp import FastMCP

server = FastMCP("pr-review")

output_dir = Path("/tmp")


@server.tool()
def add_comment(
    path: str, line: int, body: str, side: str = "RIGHT", start_line: int | None = None
) -> str:
    """Add a review comment on a specific line in the PR diff.

    Args:
        path: The relative file path in the repository (e.g. "src/main.rs").
        line: The line number in the file that the comment applies to.
              For added or modified lines, use the line number in the new version of the file (side=RIGHT).
              For deleted lines, use the line number in the old version of the file (side=LEFT).
        body: The review comment text (Markdown supported).
        side: Which version of the file the line number refers to.
              "RIGHT" for the new/modified version (default), "LEFT" for the old/deleted version.
        start_line: For multi-line comments, the first line of the range. When set, `line` is the last line.
    """
    comments_file = output_dir / "comments.json"
    if comments_file.exists():
        comments = json.loads(comments_file.read_text())
    else:
        comments = []

    comment = {"path": path, "line": line, "side": side, "body": body}
    if start_line is not None:
        comment["start_line"] = start_line
        comment["start_side"] = side

    comments.append(comment)
    comments_file.write_text(json.dumps(comments, indent=2))
    return f"Comment added on {path}:{line} ({len(comments)} total)."


@server.tool()
def finish_review(body: str) -> str:
    """Finish the review with an overall summary and verdict.

    Args:
        body: The top-level review body (Markdown supported).
    """
    conclusion = {"body": body, "event": "COMMENT"}
    conclusion_file = output_dir / "conclusion.json"
    conclusion_file.write_text(json.dumps(conclusion, indent=2))
    return "Review finished."


if __name__ == "__main__":
    server.run(transport="stdio")
