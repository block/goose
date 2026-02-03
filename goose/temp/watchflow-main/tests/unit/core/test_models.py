from datetime import datetime

import pytest
from pydantic import ValidationError

from src.core.models import Acknowledgment, Severity, Violation


class TestSeverity:
    def test_severity_values(self) -> None:
        """Test that Severity enum has expected values."""
        assert str(Severity.INFO) == "info"
        assert str(Severity.LOW) == "low"
        assert str(Severity.MEDIUM) == "medium"
        assert str(Severity.HIGH) == "high"
        assert str(Severity.CRITICAL) == "critical"

    def test_severity_str_behavior(self) -> None:
        """Test that Severity behaves like a string."""
        assert str(Severity.HIGH) == "high"
        assert str(Severity.CRITICAL) == "critical"
        assert f"Severity is {Severity.LOW}" == "Severity is low"


class TestViolation:
    def test_valid_violation(self) -> None:
        """Test creating a valid violation."""
        v = Violation(rule_description="Test Rule", severity=Severity.HIGH, message="Something went wrong")
        assert v.rule_description == "Test Rule"
        assert v.severity == Severity.HIGH
        assert v.message == "Something went wrong"
        assert v.details == {}
        assert v.how_to_fix is None

    def test_violation_with_defaults(self) -> None:
        """Test violation default values."""
        v = Violation(rule_description="Test Rule", message="Message")
        assert v.severity == Severity.MEDIUM
        assert v.details == {}

    def test_invalid_severity(self) -> None:
        """Test validation error for invalid severity."""
        with pytest.raises(ValidationError):
            Violation(rule_description="Test", message="Msg", severity="unknown_level")  # type: ignore


class TestAcknowledgment:
    def test_valid_acknowledgment(self) -> None:
        """Test creating a valid acknowledgment."""
        ack = Acknowledgment(
            rule_id="rule-1", reason="False positive", commenter="user1", violations=[], pull_request_id=1
        )
        assert ack.rule_id == "rule-1"
        assert ack.reason == "False positive"
        assert ack.commenter == "user1"
        assert isinstance(ack.timestamp, datetime)

    def test_required_fields(self) -> None:
        """Test missing required fields raises error."""
        with pytest.raises(ValidationError):
            Acknowledgment(rule_id="rule-1")  # type: ignore
