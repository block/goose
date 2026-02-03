"""
Unit tests for src/rules/acknowledgment.py

Tests cover:
- RuleID Enum contract
- is_acknowledgment_comment() detection
- extract_acknowledgment_reason() parsing
- map_violation_text_to_rule_id() mappings
- parse_acknowledgment_comment() full parsing
"""

import pytest

from src.core.models import Acknowledgment
from src.rules.acknowledgment import (
    ACKNOWLEDGMENT_INDICATORS,
    RULE_ID_TO_DESCRIPTION,
    VIOLATION_TEXT_TO_RULE_MAPPING,
    RuleID,
    extract_acknowledgment_reason,
    is_acknowledgment_comment,
    map_violation_text_to_rule_description,
    map_violation_text_to_rule_id,
    parse_acknowledgment_comment,
)


class TestRuleIDEnum:
    """Tests for RuleID Enum contract."""

    def test_all_rule_ids_are_strings(self):
        """All RuleID values should be valid strings."""
        for rule_id in RuleID:
            assert isinstance(rule_id.value, str)
            assert len(rule_id.value) > 0

    def test_rule_id_count(self):
        """Verify we have exactly 11 standardized rule IDs."""
        assert len(RuleID) == 11

    def test_all_rule_ids_have_descriptions(self):
        """Every RuleID should have a corresponding description."""
        for rule_id in RuleID:
            assert rule_id in RULE_ID_TO_DESCRIPTION
            assert len(RULE_ID_TO_DESCRIPTION[rule_id]) > 0

    def test_all_rule_ids_have_violation_mappings(self):
        """Every RuleID should be reachable via violation text mapping."""
        mapped_rule_ids = set(VIOLATION_TEXT_TO_RULE_MAPPING.values())
        for rule_id in RuleID:
            assert rule_id in mapped_rule_ids, f"RuleID {rule_id} has no violation text mapping"


class TestIsAcknowledgmentComment:
    """Tests for is_acknowledgment_comment() function."""

    @pytest.mark.parametrize(
        "indicator",
        ACKNOWLEDGMENT_INDICATORS,
    )
    def test_detects_all_indicators(self, indicator: str):
        """Should detect all standard acknowledgment indicators."""
        comment = f"Some prefix text {indicator} some suffix text"
        assert is_acknowledgment_comment(comment) is True

    def test_returns_false_for_regular_comment(self):
        """Should return False for non-acknowledgment comments."""
        assert is_acknowledgment_comment("Just a regular comment") is False
        assert is_acknowledgment_comment("LGTM!") is False
        assert is_acknowledgment_comment("Please fix the tests") is False

    def test_returns_false_for_empty_comment(self):
        """Should handle empty strings gracefully."""
        assert is_acknowledgment_comment("") is False

    def test_case_sensitive_detection(self):
        """Indicators are case-sensitive."""
        # The actual indicator uses checkmark emoji, so case doesn't apply
        # But ensure we don't match lowercase version of non-emoji parts
        assert is_acknowledgment_comment("violations acknowledged") is False


class TestExtractAcknowledgmentReason:
    """Tests for extract_acknowledgment_reason() function."""

    def test_double_quoted_reason(self):
        """Should extract reason from double quotes."""
        comment = '@watchflow ack "Urgent production fix"'
        assert extract_acknowledgment_reason(comment) == "Urgent production fix"

    def test_single_quoted_reason(self):
        """Should extract reason from single quotes."""
        comment = "@watchflow acknowledge 'Critical security patch'"
        assert extract_acknowledgment_reason(comment) == "Critical security patch"

    def test_unquoted_reason(self):
        """Should extract reason without quotes until end of line."""
        comment = "@watchflow ack This is my reason"
        assert extract_acknowledgment_reason(comment) == "This is my reason"

    def test_override_pattern(self):
        """Should extract reason from @watchflow override."""
        comment = "@watchflow override Emergency deployment needed"
        assert extract_acknowledgment_reason(comment) == "Emergency deployment needed"

    def test_bypass_pattern(self):
        """Should extract reason from @watchflow bypass."""
        comment = "@watchflow bypass Weekend release approved by manager"
        assert extract_acknowledgment_reason(comment) == "Weekend release approved by manager"

    def test_slash_override_pattern(self):
        """Should extract reason from /override command."""
        comment = "/override Reason here"
        assert extract_acknowledgment_reason(comment) == "Reason here"

    def test_slash_acknowledge_pattern(self):
        """Should extract reason from /acknowledge command."""
        comment = "/acknowledge This is acceptable"
        assert extract_acknowledgment_reason(comment) == "This is acceptable"

    def test_slash_bypass_pattern(self):
        """Should extract reason from /bypass command."""
        comment = "/bypass Manager approved"
        assert extract_acknowledgment_reason(comment) == "Manager approved"

    def test_no_match_returns_empty(self):
        """Should return empty string when no pattern matches."""
        assert extract_acknowledgment_reason("No match here") == ""
        assert extract_acknowledgment_reason("Just a comment") == ""

    def test_case_insensitive_matching(self):
        """Patterns should match case-insensitively."""
        comment = '@WATCHFLOW ACK "uppercase test"'
        assert extract_acknowledgment_reason(comment) == "uppercase test"


class TestMapViolationTextToRuleId:
    """Tests for map_violation_text_to_rule_id() function."""

    @pytest.mark.parametrize(
        "text,expected_rule_id",
        [
            ("Pull request does not have the minimum required approvals", RuleID.MIN_PR_APPROVALS),
            ("Pull request is missing required label: security", RuleID.REQUIRED_LABELS),
            ("Pull request title does not match the required pattern", RuleID.PR_TITLE_PATTERN),
            ("Pull request description is too short (20 chars)", RuleID.PR_DESCRIPTION_REQUIRED),
            ("Individual files cannot exceed 10MB limit", RuleID.FILE_SIZE_LIMIT),
            ("Pull request exceeds maximum lines changed (1234 > 500)", RuleID.MAX_PR_LOC),
            (
                "PR does not reference a linked issue (e.g. #123 or closes #123 in body/title)",
                RuleID.REQUIRE_LINKED_ISSUE,
            ),
            ("Force pushes are not allowed on this branch", RuleID.NO_FORCE_PUSH),
            ("Direct pushes to main/master branches prohibited", RuleID.PROTECTED_BRANCH_PUSH),
            ("Paths without a code owner in CODEOWNERS: src/bar.py", RuleID.PATH_HAS_CODE_OWNER),
            (
                "Code owners for modified paths must be added as reviewers: alice",
                RuleID.REQUIRE_CODE_OWNER_REVIEWERS,
            ),
        ],
    )
    def test_maps_violation_text_correctly(self, text: str, expected_rule_id: RuleID):
        """Should map violation text to correct RuleID."""
        result = map_violation_text_to_rule_id(text)
        assert result == expected_rule_id

    def test_returns_none_for_unknown_text(self):
        """Should return None for unrecognized violation text."""
        assert map_violation_text_to_rule_id("Unknown violation type") is None
        assert map_violation_text_to_rule_id("") is None


class TestMapViolationTextToRuleDescription:
    """Tests for map_violation_text_to_rule_description() function."""

    def test_maps_to_description(self):
        """Should map violation text to human-readable description."""
        text = "Pull request does not have the minimum required approvals"
        description = map_violation_text_to_rule_description(text)
        assert description == "Pull requests require at least 2 approvals"

    def test_returns_unknown_for_unrecognized(self):
        """Should return 'Unknown Rule' for unrecognized text."""
        assert map_violation_text_to_rule_description("random text") == "Unknown Rule"


class TestParseAcknowledgmentComment:
    """Tests for parse_acknowledgment_comment() function."""

    def test_parses_single_violation(self):
        """Should parse a comment with one acknowledged violation."""
        comment = """✅ **Violations Acknowledged**
**Reason:** Emergency fix

The following violations have been overridden:
• Pull request does not have the minimum required approvals

---
*This acknowledgment was validated.*"""

        acknowledgments = parse_acknowledgment_comment(comment, "testuser")

        assert len(acknowledgments) == 1
        assert acknowledgments[0].rule_id == RuleID.MIN_PR_APPROVALS.value
        assert acknowledgments[0].reason == "Emergency fix"
        assert acknowledgments[0].commenter == "testuser"

    def test_parses_multiple_violations(self):
        """Should parse a comment with multiple acknowledged violations."""
        comment = """✅ **Violations Acknowledged**
**Reason:** Sprint deadline

The following violations have been overridden:
• Pull request does not have the minimum required approvals
• Pull request is missing required label: review

---"""

        acknowledgments = parse_acknowledgment_comment(comment, "dev")

        assert len(acknowledgments) == 2
        rule_ids = [ack.rule_id for ack in acknowledgments]
        assert RuleID.MIN_PR_APPROVALS.value in rule_ids
        assert RuleID.REQUIRED_LABELS.value in rule_ids

    def test_empty_comment_returns_empty_list(self):
        """Should return empty list for comments without violations."""
        assert parse_acknowledgment_comment("", "user") == []

    def test_returns_acknowledgment_models(self):
        """Should return proper Acknowledgment model instances."""
        comment = """The following violations have been overridden:
• Force pushes are not allowed"""

        acknowledgments = parse_acknowledgment_comment(comment, "admin")

        assert len(acknowledgments) == 1
        assert isinstance(acknowledgments[0], Acknowledgment)

    def test_stops_at_section_delimiter(self):
        """Should stop parsing when hitting section delimiters."""
        comment = """The following violations have been overridden:
• Pull request title does not match the required pattern
---
⚠️ Other content that should be ignored
• Some other bullet that is NOT a violation"""

        acknowledgments = parse_acknowledgment_comment(comment, "user")

        assert len(acknowledgments) == 1
        assert acknowledgments[0].rule_id == RuleID.PR_TITLE_PATTERN.value
