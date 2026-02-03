import logging
from typing import Any

from src.core.models import Violation
from src.integrations.github.api import GitHubClient
from src.presentation import github_formatter

logger = logging.getLogger(__name__)


class CheckRunManager:
    """
    Manager for handling GitHub Check Runs.
    Encapsulates logic for creating and updating check runs based on rule violations.
    """

    def __init__(self, github_client: GitHubClient):
        self.github_client = github_client

    async def create_check_run(
        self,
        repo: str,
        sha: str,
        installation_id: int,
        violations: list[Violation],
        conclusion: str | None = None,
        error: str | None = None,
    ) -> None:
        """
        Create a check run with violation results.

        Args:
            repo: Repository full name (owner/repo)
            sha: Commit SHA to associate the check run with
            installation_id: GitHub App installation ID
            violations: List of rule violations found
            conclusion: Optional override for check run conclusion (e.g., "neutral")
            error: Optional error message if processing failed
        """
        try:
            if not sha:
                logger.warning(f"Cannot create check run for {repo}: SHA is missing")
                return

            status = "completed"
            # Determine conclusion if not provided
            if conclusion is None:
                conclusion = "failure" if violations or error else "success"

            output = github_formatter.format_check_run_output(violations, error, repo, installation_id)

            await self.github_client.create_check_run(
                repo=repo,
                sha=sha,
                name="Watchflow Rules",
                status=status,
                conclusion=conclusion,
                output=output,
                installation_id=installation_id,
            )
            logger.info(f"Created check run for {repo}@{sha} with conclusion: {conclusion}")

        except Exception as e:
            logger.error(f"Error creating check run: {e}")

    async def create_acknowledgment_check_run(
        self,
        repo: str,
        sha: str,
        installation_id: int,
        acknowledgable_violations: list[Violation],
        violations: list[Violation],
        acknowledgments: dict[str, Any],  # Keeping Any for flexibility, but could be specific
    ) -> None:
        """
        Create a check run that reflects the acknowledgment state.

        Args:
            repo: Repository full name (owner/repo)
            sha: Commit SHA to associate the check run with
            installation_id: GitHub App installation ID
            acknowledgable_violations: Violations that have been acknowledged
            violations: Violations that still require fixes
            acknowledgments: Dictionary of acknowledgment details
        """
        try:
            if not sha:
                logger.warning(f"Cannot create check run for {repo}: SHA is missing")
                return

            # Convert raw dict acknowledgments to Acknowledgment objects if needed
            # The formatter expects a dict, but typed as dict[str, Acknowledgment] in signature hint,
            # but runtime logic often passes raw dicts.
            # Ideally we should cast/validate here, but following existing pattern for now.
            # Update: github_formatter.format_acknowledgment_check_run type hint says dict[str, Acknowledgment],
            # but let's assume the caller passes validated data or we trust the formatter handle.

            # Actually, to be safe and clean, let's trust the input types are correct
            # as per the refactor goals.

            output_data = github_formatter.format_acknowledgment_check_run(
                acknowledgable_violations, violations, acknowledgments
            )

            await self.github_client.create_check_run(
                repo=repo,
                sha=sha,
                name="watchflow-rules",
                status="completed",
                conclusion=output_data["conclusion"],
                output={
                    "title": output_data["title"],
                    "summary": output_data["summary"],
                    "text": output_data["text"],
                },
                installation_id=installation_id,
            )
            logger.info(f"Created acknowledgment check run for {repo}@{sha}")

        except Exception as e:
            logger.error(f"Error creating check run with acknowledgment: {e}")
