"""
Rule evaluation utilities for parsing and using CODEOWNERS files.

These utilities are used by rule validators to check code ownership
requirements and determine critical file patterns.
"""

import logging
import re
from pathlib import Path

logger = logging.getLogger(__name__)


class CodeOwnersParser:
    """Parser for CODEOWNERS files."""

    def __init__(self, codeowners_content: str):
        self.codeowners_content = codeowners_content
        self.owners_map = self._parse_codeowners()

    def _parse_codeowners(self) -> list[tuple[str, list[str]]]:
        """
        Parse CODEOWNERS content into a list of (pattern, owners) tuples.

        Returns:
            List of tuples where each tuple is (pattern, [owners])
        """
        owners_map = []

        for line_num, line in enumerate(self.codeowners_content.split("\n"), 1):
            line = line.strip()

            # Skip empty lines and comments
            if not line or line.startswith("#"):
                continue

            # Split on whitespace, first part is pattern, rest are owners
            parts = line.split()
            if len(parts) < 2:
                logger.warning(f"Invalid CODEOWNERS line {line_num}: {line}")
                continue

            pattern = parts[0]
            owners = parts[1:]

            # Clean up owner references (remove @ if present)
            owners = [owner.lstrip("@") for owner in owners]

            owners_map.append((pattern, owners))

        return owners_map

    def get_owners_for_file(self, file_path: str) -> list[str]:
        """
        Get the owners for a specific file based on CODEOWNERS rules.

        Args:
            file_path: Path to the file relative to repository root

        Returns:
            List of owner usernames/teams
        """
        owners = []

        for pattern, pattern_owners in self.owners_map:
            if self._matches_pattern(file_path, pattern):
                owners.extend(pattern_owners)

        # Remove duplicates while preserving order
        seen = set()
        unique_owners = []
        for owner in owners:
            if owner not in seen:
                seen.add(owner)
                unique_owners.append(owner)

        return unique_owners

    def _matches_pattern(self, file_path: str, pattern: str) -> bool:
        """
        Check if a file path matches a CODEOWNERS pattern.

        Args:
            file_path: Path to check
            pattern: CODEOWNERS pattern to match against

        Returns:
            True if the file matches the pattern
        """
        # Handle different pattern types
        if pattern == "*":
            return True

        # Convert pattern to regex
        regex_pattern = CodeOwnersParser._pattern_to_regex(pattern)

        try:
            return bool(re.match(regex_pattern, file_path))
        except re.error:
            logger.error(f"Invalid regex pattern: {regex_pattern}")
            return False

    @staticmethod
    def _pattern_to_regex(pattern: str) -> str:
        """
        Convert a CODEOWNERS pattern to a regex pattern.

        Args:
            pattern: CODEOWNERS pattern (e.g., "*.py", "/docs/", "src/")

        Returns:
            Regex pattern string
        """
        # Handle directory patterns
        if pattern.endswith("/"):
            # Directory pattern - match files in that directory
            pattern = pattern.rstrip("/")
            return f"^{re.escape(pattern)}/.*$"

        # Handle glob patterns
        if "*" in pattern:
            # Convert glob to regex
            regex = re.escape(pattern)
            regex = regex.replace("\\*", ".*")
            return f"^{regex}$"

        # Exact match
        return f"^{re.escape(pattern)}$"

    def get_critical_files(self, critical_owners: list[str] | None = None) -> list[str]:
        """
        Get a list of file patterns that are considered critical.

        Args:
            critical_owners: List of owner usernames/teams that indicate critical files
                           If None, returns all patterns with any owners

        Returns:
            List of critical file patterns
        """
        critical_patterns = []

        for pattern, owners in self.owners_map:
            # If no specific critical owners provided, consider all patterns with owners as critical
            if critical_owners is None or any(owner in critical_owners for owner in owners):
                critical_patterns.append(pattern)

        return critical_patterns

    def has_owners(self, file_path: str) -> bool:
        """
        Check if a file has any owners defined.

        Args:
            file_path: Path to the file relative to repository root

        Returns:
            True if the file has owners defined
        """
        return len(self.get_owners_for_file(file_path)) > 0


def path_has_owner(file_path: str, codeowners_content: str) -> bool:
    """
    Check if a path has any code owner defined using CODEOWNERS content (no disk read).

    Args:
        file_path: Path to the file relative to repository root
        codeowners_content: Raw content of the CODEOWNERS file

    Returns:
        True if the path matches at least one pattern and has owners
    """
    parser = CodeOwnersParser(codeowners_content)
    return parser.has_owners(file_path)


def load_codeowners(repo_path: str = ".") -> CodeOwnersParser | None:
    """
    Load and parse CODEOWNERS file from repository.

    Args:
        repo_path: Path to repository root

    Returns:
        CodeOwnersParser instance or None if file not found
    """
    codeowners_path = Path(repo_path) / "CODEOWNERS"

    if not codeowners_path.exists():
        logger.warning(f"CODEOWNERS file not found at {codeowners_path}")
        return None

    try:
        with open(codeowners_path, encoding="utf-8") as f:
            content = f.read()

        return CodeOwnersParser(content)
    except Exception as e:
        logger.error(f"Error loading CODEOWNERS file: {e}")
        return None


def get_file_owners(file_path: str, repo_path: str = ".") -> list[str]:
    """
    Get owners for a specific file.

    Args:
        file_path: Path to the file relative to repository root
        repo_path: Path to repository root

    Returns:
        List of owner usernames/teams
    """
    parser = load_codeowners(repo_path)
    if not parser:
        return []

    return parser.get_owners_for_file(file_path)


def is_critical_file(file_path: str, repo_path: str = ".", critical_owners: list[str] | None = None) -> bool:
    """
    Check if a file is considered critical based on CODEOWNERS.

    Args:
        file_path: Path to the file relative to repository root
        repo_path: Path to repository root
        critical_owners: List of owner usernames/teams that indicate critical files
                        If None, any file with owners is considered critical

    Returns:
        True if the file is critical
    """
    parser = load_codeowners(repo_path)
    if not parser:
        return False

    # If no critical owners specified, consider any file with owners as critical
    if critical_owners is None:
        return parser.has_owners(file_path)

    # Check if file has any of the critical owners
    owners = parser.get_owners_for_file(file_path)
    return any(owner in critical_owners for owner in owners)
