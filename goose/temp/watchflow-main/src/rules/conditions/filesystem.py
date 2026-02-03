"""Filesystem-related conditions for rule validation.

This module contains conditions that validate file-related aspects
of pull requests and push events.
"""

import logging
import re
from typing import Any

from src.core.models import Severity, Violation
from src.rules.conditions.base import BaseCondition

logger = logging.getLogger(__name__)


class FilePatternCondition(BaseCondition):
    """Validates if files in the event match or don't match a pattern."""

    name = "files_match_pattern"
    description = "Validates if files in the event match or don't match a pattern"
    parameter_patterns = ["pattern", "condition_type"]
    event_types = ["pull_request", "push"]
    examples = [
        {"pattern": "*.py", "condition_type": "files_match_pattern"},
        {"pattern": "*.md", "condition_type": "files_not_match_pattern"},
    ]

    async def evaluate(self, context: Any) -> list[Violation]:
        """Evaluate file pattern matching condition.

        Args:
            context: Dict with 'parameters' and 'event' keys.

        Returns:
            List of violations if pattern matching fails.
        """
        parameters = context.get("parameters", {})
        event = context.get("event", {})

        pattern = parameters.get("pattern")
        if not pattern:
            logger.warning("FilePatternCondition: No pattern specified in parameters")
            return [
                Violation(
                    rule_description=self.description,
                    severity=Severity.MEDIUM,
                    message="No pattern specified in parameters",
                    how_to_fix="Provide a 'pattern' parameter in the rule configuration.",
                )
            ]

        changed_files = self._get_changed_files(event)

        if not changed_files:
            logger.debug("No files to check against pattern")
            return [
                Violation(
                    rule_description=self.description,
                    severity=Severity.INFO,
                    message="No files available to check against pattern",
                )
            ]

        regex_pattern = self._glob_to_regex(pattern)
        matching_files = [file for file in changed_files if re.match(regex_pattern, file)]

        condition_type = parameters.get("condition_type", "files_match_pattern")

        if condition_type == "files_not_match_pattern":
            if len(matching_files) > 0:
                return [
                    Violation(
                        rule_description=self.description,
                        severity=Severity.MEDIUM,
                        message=f"Files match forbidden pattern '{pattern}': {matching_files}",
                        details={"matching_files": matching_files, "pattern": pattern},
                        how_to_fix=f"Remove or rename files matching pattern '{pattern}'.",
                    )
                ]
        else:
            if len(matching_files) == 0:
                return [
                    Violation(
                        rule_description=self.description,
                        severity=Severity.MEDIUM,
                        message=f"No files match required pattern '{pattern}'",
                        details={"pattern": pattern, "checked_files": changed_files},
                        how_to_fix=f"Ensure at least one file matches pattern '{pattern}'.",
                    )
                ]

        return []

    async def validate(self, parameters: dict[str, Any], event: dict[str, Any]) -> bool:
        """Legacy validation interface for backward compatibility."""
        pattern = parameters.get("pattern")
        if not pattern:
            logger.warning("FilePatternCondition: No pattern specified in parameters")
            return False

        changed_files = self._get_changed_files(event)

        if not changed_files:
            logger.debug("No files to check against pattern")
            return False

        regex_pattern = self._glob_to_regex(pattern)
        matching_files = [file for file in changed_files if re.match(regex_pattern, file)]

        condition_type = parameters.get("condition_type", "files_match_pattern")

        if condition_type == "files_not_match_pattern":
            return len(matching_files) == 0
        else:
            return len(matching_files) > 0

    def _get_changed_files(self, event: dict[str, Any]) -> list[str]:
        """Extract the list of changed files from the event."""
        event_type = event.get("event_type", "")
        if event_type == "pull_request":
            # TODO: Pull request—fetch changed files via GitHub API. Placeholder for now.
            return []
        elif event_type == "push":
            # Push event—files in commits, not implemented.
            return []
        else:
            return []

    @staticmethod
    def _glob_to_regex(glob_pattern: str) -> str:
        """Convert a glob pattern to a regex pattern."""
        regex = glob_pattern.replace(".", "\\.").replace("*", ".*").replace("?", ".")
        return f"^{regex}$"


class MaxFileSizeCondition(BaseCondition):
    """Validates if files don't exceed maximum size limits."""

    name = "max_file_size_mb"
    description = "Validates if files don't exceed maximum size limits"
    parameter_patterns = ["max_file_size_mb"]
    event_types = ["pull_request", "push"]
    examples = [{"max_file_size_mb": 10}, {"max_file_size_mb": 1}]

    async def evaluate(self, context: Any) -> list[Violation]:
        """Evaluate file size condition.

        Args:
            context: Dict with 'parameters' and 'event' keys.

        Returns:
            List of violations if any file exceeds size limit.
        """
        parameters = context.get("parameters", {})
        event = context.get("event", {})

        max_size_mb = parameters.get("max_file_size_mb", 100)
        files = event.get("files", [])

        if not files:
            logger.debug("MaxFileSizeCondition: No files data available, skipping validation")
            return []

        violations: list[Violation] = []
        oversized_files: list[str] = []

        for file in files:
            size_bytes = file.get("size", 0)
            size_mb = size_bytes / (1024 * 1024)
            if size_mb > max_size_mb:
                filename = file.get("filename", "unknown")
                oversized_files.append(f"{filename} ({size_mb:.2f}MB)")
                logger.debug(
                    f"MaxFileSizeCondition: File {filename} exceeds size limit: {size_mb:.2f}MB > {max_size_mb}MB"
                )

        if oversized_files:
            violations.append(
                Violation(
                    rule_description=self.description,
                    severity=Severity.HIGH,
                    message=f"Files exceed size limit of {max_size_mb}MB: {', '.join(oversized_files)}",
                    details={"oversized_files": oversized_files, "max_size_mb": max_size_mb},
                    how_to_fix=f"Reduce file sizes to under {max_size_mb}MB or use Git LFS for large files.",
                )
            )
        else:
            logger.debug(f"MaxFileSizeCondition: All {len(files)} files are within size limit of {max_size_mb}MB")

        return violations

    async def validate(self, parameters: dict[str, Any], event: dict[str, Any]) -> bool:
        """Legacy validation interface for backward compatibility."""
        max_size_mb = parameters.get("max_file_size_mb", 100)
        files = event.get("files", [])

        if not files:
            logger.debug("MaxFileSizeCondition: No files data available, skipping validation")
            return True

        oversized_files: list[str] = []
        for file in files:
            size_bytes = file.get("size", 0)
            size_mb = size_bytes / (1024 * 1024)
            if size_mb > max_size_mb:
                filename = file.get("filename", "unknown")
                oversized_files.append(f"{filename} ({size_mb:.2f}MB)")
                logger.debug(
                    f"MaxFileSizeCondition: File {filename} exceeds size limit: {size_mb:.2f}MB > {max_size_mb}MB"
                )

        is_valid = len(oversized_files) == 0

        if is_valid:
            logger.debug(f"MaxFileSizeCondition: All {len(files)} files are within size limit of {max_size_mb}MB")
        else:
            logger.debug(f"MaxFileSizeCondition: {len(oversized_files)} files exceed size limit: {oversized_files}")

        return is_valid


class MaxPrLocCondition(BaseCondition):
    """Validates that total lines changed (additions + deletions) in a PR do not exceed a maximum."""

    name = "max_pr_loc"
    description = "Validates that total lines changed (additions + deletions) in a PR do not exceed a maximum; enforces a maximum LOC per pull request."
    parameter_patterns = ["max_lines"]
    event_types = ["pull_request"]
    examples = [{"max_lines": 500}, {"max_lines": 1000}]

    async def evaluate(self, context: Any) -> list[Violation]:
        """Evaluate max PR LOC condition.

        Args:
            context: Dict with 'parameters' and 'event' keys.

        Returns:
            List of violations if total lines changed exceed the limit.
        """
        parameters = context.get("parameters", {})
        event = context.get("event", {})

        max_lines = parameters.get("max_lines", 0)
        if not max_lines:
            logger.debug("MaxPrLocCondition: No max_lines specified, skipping validation")
            return []

        changed_files = event.get("changed_files", []) or event.get("files", [])
        total = sum(int(f.get("additions", 0) or 0) + int(f.get("deletions", 0) or 0) for f in changed_files)

        if total > max_lines:
            message = f"Pull request exceeds maximum lines changed ({total} > {max_lines})"
            return [
                Violation(
                    rule_description=self.description,
                    severity=Severity.MEDIUM,
                    message=message,
                    details={"total_lines": total, "max_lines": max_lines},
                    how_to_fix=f"Reduce the size of this PR to at most {max_lines} lines changed (additions + deletions).",
                )
            ]

        logger.debug(f"MaxPrLocCondition: PR within limit ({total} <= {max_lines})")
        return []

    async def validate(self, parameters: dict[str, Any], event: dict[str, Any]) -> bool:
        """Legacy validation interface for backward compatibility."""
        max_lines = parameters.get("max_lines", 0)
        if not max_lines:
            return True

        changed_files = event.get("changed_files", []) or event.get("files", [])
        total = sum(int(f.get("additions", 0) or 0) + int(f.get("deletions", 0) or 0) for f in changed_files)
        return total <= max_lines
