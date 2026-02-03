"""
Acknowledgment parsing and rule ID management.

This module centralizes all acknowledgment-related logic previously scattered
across event processors. It provides:
- RuleID Enum replacing hardcoded magic strings
- Acknowledgment comment detection and parsing
- Violation text to rule ID mapping
"""

import logging
import re
from enum import StrEnum

from src.core.models import Acknowledgment

logger = logging.getLogger(__name__)


class RuleID(StrEnum):
    """
    Standardized rule identifiers.
    Replaces hardcoded string mappings for type safety and maintainability.
    """

    MIN_PR_APPROVALS = "min-pr-approvals"
    REQUIRED_LABELS = "required-labels"
    PR_TITLE_PATTERN = "pr-title-pattern"
    PR_DESCRIPTION_REQUIRED = "pr-description-required"
    FILE_SIZE_LIMIT = "file-size-limit"
    MAX_PR_LOC = "max-pr-loc"
    REQUIRE_LINKED_ISSUE = "require-linked-issue"
    NO_FORCE_PUSH = "no-force-push"
    PROTECTED_BRANCH_PUSH = "protected-branch-push"
    PATH_HAS_CODE_OWNER = "path-has-code-owner"
    REQUIRE_CODE_OWNER_REVIEWERS = "require-code-owner-reviewers"


# Mapping from violation text patterns to RuleID
VIOLATION_TEXT_TO_RULE_MAPPING: dict[str, RuleID] = {
    "Pull request does not have the minimum required": RuleID.MIN_PR_APPROVALS,
    "Pull request is missing required label": RuleID.REQUIRED_LABELS,
    "Pull request title does not match the required pattern": RuleID.PR_TITLE_PATTERN,
    "Pull request description is too short": RuleID.PR_DESCRIPTION_REQUIRED,
    "Individual files cannot exceed": RuleID.FILE_SIZE_LIMIT,
    "Pull request exceeds maximum lines changed": RuleID.MAX_PR_LOC,
    "does not reference a linked issue": RuleID.REQUIRE_LINKED_ISSUE,
    "Force pushes are not allowed": RuleID.NO_FORCE_PUSH,
    "Direct pushes to main/master branches": RuleID.PROTECTED_BRANCH_PUSH,
    "Paths without a code owner in CODEOWNERS": RuleID.PATH_HAS_CODE_OWNER,
    "Code owners for modified paths must be added as reviewers": RuleID.REQUIRE_CODE_OWNER_REVIEWERS,
}

# Mapping from RuleID to human-readable descriptions
RULE_ID_TO_DESCRIPTION: dict[RuleID, str] = {
    RuleID.MIN_PR_APPROVALS: "Pull requests require at least 2 approvals",
    RuleID.REQUIRED_LABELS: "Pull requests must have security and review labels",
    RuleID.PR_TITLE_PATTERN: "PR titles must follow conventional commit format",
    RuleID.PR_DESCRIPTION_REQUIRED: "Pull requests must have descriptions with at least 50 characters",
    RuleID.FILE_SIZE_LIMIT: "Files must not exceed 10MB",
    RuleID.MAX_PR_LOC: "Pull requests must not exceed the configured maximum lines changed (LOC).",
    RuleID.REQUIRE_LINKED_ISSUE: "PR must reference a linked issue (e.g. closes #123).",
    RuleID.NO_FORCE_PUSH: "Force pushes are not allowed",
    RuleID.PROTECTED_BRANCH_PUSH: "Direct pushes to main branch are not allowed",
    RuleID.PATH_HAS_CODE_OWNER: "Every changed path must have a code owner defined in CODEOWNERS.",
    RuleID.REQUIRE_CODE_OWNER_REVIEWERS: "When a PR modifies paths with CODEOWNERS, those owners must be added as reviewers.",
}

# Comment markers that indicate an acknowledgment comment
ACKNOWLEDGMENT_INDICATORS: tuple[str, ...] = (
    "✅ Violations Acknowledged",
    "🚨 Watchflow Rule Violations Detected",
    "This acknowledgment was validated",
)

# Regex patterns for extracting acknowledgment reasons
ACKNOWLEDGMENT_PATTERNS: tuple[str, ...] = (
    r'@watchflow\s+(acknowledge|ack)\s+"([^"]+)"',  # Double quotes
    r"@watchflow\s+(acknowledge|ack)\s+'([^']+)'",  # Single quotes
    r"@watchflow\s+(acknowledge|ack)\s+([^\n\r]+)",  # No quotes, until end of line
    r"@watchflow\s+override\s+(.+)",
    r"@watchflow\s+bypass\s+(.+)",
    r"/acknowledge\s+(.+)",
    r"/override\s+(.+)",
    r"/bypass\s+(.+)",
)


def is_acknowledgment_comment(comment_body: str) -> bool:
    """
    Check if a comment is an acknowledgment comment.

    Args:
        comment_body: The body text of the comment to check.

    Returns:
        True if the comment contains acknowledgment indicators.
    """
    return any(indicator in comment_body for indicator in ACKNOWLEDGMENT_INDICATORS)


def extract_acknowledgment_reason(comment_body: str) -> str:
    """
    Extract acknowledgment reason from a comment.

    Args:
        comment_body: The body text of the comment.

    Returns:
        The extracted reason string, or empty string if no match.
    """
    logger.info(f"🔍 Extracting acknowledgment reason from: '{comment_body}'")

    for i, pattern in enumerate(ACKNOWLEDGMENT_PATTERNS):
        match = re.search(pattern, comment_body, re.IGNORECASE | re.DOTALL)
        if match:
            # Patterns 0-2 have (acknowledge|ack) as group 1 and reason as group 2
            # Patterns 3-7 have reason as group 1
            reason = match.group(2).strip() if i < 3 else match.group(1).strip()

            logger.info(f"✅ Pattern {i + 1} matched! Reason: '{reason}'")
            if reason:
                return reason
        else:
            logger.debug(f"❌ Pattern {i + 1} did not match")

    logger.info("❌ No patterns matched for acknowledgment reason")
    return ""


def map_violation_text_to_rule_id(violation_text: str) -> RuleID | None:
    """
    Map violation text to a standardized RuleID.

    Args:
        violation_text: The human-readable violation message.

    Returns:
        The corresponding RuleID, or None if no match found.
    """
    for key, rule_id in VIOLATION_TEXT_TO_RULE_MAPPING.items():
        if key in violation_text:
            return rule_id
    return None


def map_violation_text_to_rule_description(violation_text: str) -> str:
    """
    Map violation text to a human-readable rule description.

    Args:
        violation_text: The human-readable violation message.

    Returns:
        The corresponding description, or "Unknown Rule" if no match.
    """
    rule_id = map_violation_text_to_rule_id(violation_text)
    if rule_id:
        return RULE_ID_TO_DESCRIPTION.get(rule_id, "Unknown Rule")
    return "Unknown Rule"


def parse_acknowledgment_comment(comment_body: str, commenter: str) -> list[Acknowledgment]:
    """
    Parse acknowledged violations from a comment.

    Args:
        comment_body: The body text of the acknowledgment comment.
        commenter: Username of the person who made the comment.

    Returns:
        List of Acknowledgment models for each parsed violation.
    """
    acknowledgments: list[Acknowledgment] = []

    # Extract acknowledgment reason
    reason_match = re.search(r"\*\*Reason:\*\* (.+)", comment_body)
    reason = reason_match.group(1) if reason_match else ""

    # Look for violation lines (bullet points)
    lines = comment_body.split("\n")
    in_violations_section = False

    for line in lines:
        line = line.strip()

        # Check if we're entering the violations section
        if "The following violations have been overridden:" in line:
            in_violations_section = True
            continue

        # Check if we're leaving the violations section
        if in_violations_section and (line.startswith("---") or line.startswith("⚠️") or line.startswith("*")):
            break

        # Parse violation lines
        if in_violations_section and line.startswith("•"):
            violation_text = line[1:].strip()

            # Map violation text to rule ID
            rule_id = map_violation_text_to_rule_id(violation_text)

            if rule_id:
                acknowledgments.append(
                    Acknowledgment(
                        rule_id=rule_id.value,
                        reason=reason,
                        commenter=commenter,
                    )
                )

    return acknowledgments
