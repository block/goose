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
def add_comment(path: str, position: int, body: str) -> str:
    """Add a review comment on a specific line in the PR diff.

    Args:
        path: The relative file path in the repository (e.g. "src/main.rs").
        position: The line position in the diff. Position 1 is the first line
                  after the @@ hunk header, position 2 is the next line, and so on.
                  The count continues through whitespace and additional hunks
                  until a new file begins.
        body: The review comment text (Markdown supported).
    """
    comments_file = output_dir / "comments.json"
    if comments_file.exists():
        comments = json.loads(comments_file.read_text())
    else:
        comments = []

    comments.append({"path": path, "position": position, "body": body})
    comments_file.write_text(json.dumps(comments, indent=2))
    return f"Comment added on {path} at position {position} ({len(comments)} total)."


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
