import logging
from typing import Any

from src.core.models import Acknowledgment
from src.rules.acknowledgment import (
    is_acknowledgment_comment,
    parse_acknowledgment_comment,
)

logger = logging.getLogger(__name__)


class PullRequestEnricher:
    """
    Handles data fetching and enrichment for pull request processing.
    Delegates GitHub API calls and returns structured data.
    """

    def __init__(self, github_client: Any):
        self.github_client = github_client

    async def fetch_api_data(self, repo_full_name: str, pr_number: int, installation_id: int) -> dict[str, Any]:
        """Fetch supplementary data not available in the webhook payload."""
        api_data = {}
        try:
            # Fetch reviews
            reviews = await self.github_client.get_pull_request_reviews(repo_full_name, pr_number, installation_id)
            api_data["reviews"] = reviews or []

            # Fetch files
            files = await self.github_client.get_pull_request_files(repo_full_name, pr_number, installation_id)
            api_data["files"] = files or []

        except Exception as e:
            logger.error(f"Error fetching API data for PR #{pr_number}: {e}")

        return api_data

    async def enrich_event_data(self, task: Any, github_token: str) -> dict[str, Any]:
        """Prepare enriched event data for rule evaluation agents."""
        if not task or not hasattr(task, "payload") or not task.payload:
            return {}

        pr_data = task.payload.get("pull_request", {}) or {}
        pr_number = pr_data.get("number")
        repo_full_name = getattr(task, "repo_full_name", "")
        installation_id = getattr(task, "installation_id", 0)

        # Base event data
        event_data = {
            "pull_request_details": pr_data,
            "triggering_user": {"login": (pr_data.get("user") or {}).get("login")},
            "repository": task.payload.get("repository", {}),
            "organization": task.payload.get("organization", {}),
            "event_id": task.payload.get("event_id"),
            "timestamp": task.payload.get("timestamp"),
            "installation": {"id": installation_id},
            "github_client": self.github_client,
        }

        # Enrich with API data if PR number is available
        if pr_number:
            api_data = await self.fetch_api_data(repo_full_name, pr_number, installation_id)
            event_data.update(api_data)

            if "files" in api_data:
                files = api_data["files"]
                event_data["changed_files"] = [
                    {
                        "filename": f.get("filename"),
                        "status": f.get("status"),
                        "additions": f.get("additions"),
                        "deletions": f.get("deletions"),
                    }
                    for f in files
                ]
                event_data["diff_summary"] = self.summarize_files(files)

            # Fetch CODEOWNERS so path-has-code-owner rule can evaluate without a local repo
            codeowners_paths = [".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"]
            for path in codeowners_paths:
                try:
                    content = await self.github_client.get_file_content(repo_full_name, path, installation_id)
                    if content:
                        event_data["codeowners_content"] = content
                        break
                except Exception:
                    continue

        return event_data

    async def fetch_acknowledgments(self, repo: str, pr_number: int, installation_id: int) -> dict[str, Acknowledgment]:
        """Fetch and parse previous acknowledgments from PR comments."""
        try:
            comments = await self.github_client.get_issue_comments(repo, pr_number, installation_id)
            if not comments:
                return {}

            acknowledgments = {}
            for comment in comments:
                comment_body = comment.get("body", "")
                commenter = comment.get("user", {}).get("login", "")

                if is_acknowledgment_comment(comment_body):
                    acknowledged_violations = parse_acknowledgment_comment(comment_body, commenter)
                    for ack in acknowledged_violations:
                        if ack.rule_id:
                            acknowledgments[ack.rule_id] = ack

            return acknowledgments
        except Exception as e:
            logger.error(f"Error fetching acknowledgments: {e}")
            return {}

    def prepare_webhook_data(self, task: Any) -> dict[str, Any]:
        """Extract data available in webhook payload."""
        if not task or not hasattr(task, "payload") or not task.payload:
            return {}

        pr_data = task.payload.get("pull_request", {}) or {}

        return {
            "event_type": "pull_request",
            "repo_full_name": getattr(task, "repo_full_name", ""),
            "action": task.payload.get("action"),
            "pull_request": {
                "number": pr_data.get("number"),
                "title": pr_data.get("title"),
                "body": pr_data.get("body"),
                "state": pr_data.get("state"),
                "created_at": pr_data.get("created_at"),
                "updated_at": pr_data.get("updated_at"),
                "merged_at": pr_data.get("merged_at"),
                "user": (pr_data.get("user") or {}).get("login"),
                "head": {
                    "ref": (pr_data.get("head") or {}).get("ref"),
                    "sha": (pr_data.get("head") or {}).get("sha"),
                },
                "base": {
                    "ref": (pr_data.get("base") or {}).get("ref"),
                    "sha": (pr_data.get("base") or {}).get("sha"),
                },
                "labels": pr_data.get("labels", []),
                "files": pr_data.get("files", []),
            },
        }

    @staticmethod
    def summarize_files(files: list[dict[str, Any]], max_files: int = 5, max_patch_lines: int = 8) -> str:
        """Build a compact diff summary suitable for LLM prompts."""
        if not files:
            return ""

        summary_lines: list[str] = []
        for file in files[:max_files]:
            filename = file.get("filename", "unknown")
            status = file.get("status", "modified")
            additions = file.get("additions", 0)
            deletions = file.get("deletions", 0)
            summary_lines.append(f"- {filename} ({status}, +{additions}/-{deletions})")

            patch = file.get("patch")
            if patch:
                lines = patch.splitlines()
                truncated = lines[:max_patch_lines]
                indented_patch = "\n".join(f"    {line}" for line in truncated)
                summary_lines.append(indented_patch)
                if len(lines) > max_patch_lines:
                    summary_lines.append("    ... (diff truncated)")

        return "\n".join(summary_lines)
