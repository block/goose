"""Tests for temporal conditions.

Tests for WeekendCondition, AllowedHoursCondition, and DaysCondition classes.
"""

from datetime import datetime
from unittest.mock import patch

import pytest

from src.rules.conditions.temporal import (
    AllowedHoursCondition,
    DaysCondition,
    WeekendCondition,
)


class TestWeekendCondition:
    """Tests for WeekendCondition class."""

    @pytest.mark.asyncio
    async def test_validate_weekday_returns_true(self) -> None:
        """Test that validate returns True on weekdays."""
        condition = WeekendCondition()

        # Mock datetime to return a Wednesday (weekday 2)
        mock_dt = datetime(2026, 1, 28, 10, 0, 0)  # Wednesday
        with patch("src.rules.conditions.temporal.datetime") as mock_datetime:
            mock_datetime.now.return_value = mock_dt
            result = await condition.validate({}, {})
            assert result is True

    @pytest.mark.asyncio
    async def test_validate_weekend_returns_false(self) -> None:
        """Test that validate returns False on weekends."""
        condition = WeekendCondition()

        # Mock datetime to return a Saturday (weekday 5)
        mock_dt = datetime(2026, 1, 31, 10, 0, 0)  # Saturday
        with patch("src.rules.conditions.temporal.datetime") as mock_datetime:
            mock_datetime.now.return_value = mock_dt
            result = await condition.validate({}, {})
            assert result is False

    @pytest.mark.asyncio
    async def test_evaluate_returns_violations_on_weekend(self) -> None:
        """Test that evaluate returns violations on weekends."""
        condition = WeekendCondition()

        # Mock datetime to return a Sunday (weekday 6)
        mock_dt = datetime(2026, 2, 1, 10, 0, 0)  # Sunday
        with patch("src.rules.conditions.temporal.datetime") as mock_datetime:
            mock_datetime.now.return_value = mock_dt
            mock_datetime.strftime = datetime.strftime.__get__(mock_dt)

            violations = await condition.evaluate({"parameters": {}, "event": {}})
            assert len(violations) == 1
            assert "weekend" in violations[0].message.lower()

    @pytest.mark.asyncio
    async def test_evaluate_returns_empty_on_weekday(self) -> None:
        """Test that evaluate returns empty list on weekdays."""
        condition = WeekendCondition()

        mock_dt = datetime(2026, 1, 28, 10, 0, 0)  # Wednesday
        with patch("src.rules.conditions.temporal.datetime") as mock_datetime:
            mock_datetime.now.return_value = mock_dt

            violations = await condition.evaluate({"parameters": {}, "event": {}})
            assert len(violations) == 0


class TestAllowedHoursCondition:
    """Tests for AllowedHoursCondition class."""

    @pytest.mark.asyncio
    async def test_validate_within_allowed_hours(self) -> None:
        """Test that validate returns True within allowed hours."""
        condition = AllowedHoursCondition()

        mock_dt = datetime(2026, 1, 28, 10, 0, 0)  # 10:00
        with patch.object(condition, "_get_current_time", return_value=mock_dt):
            result = await condition.validate({"allowed_hours": [9, 10, 11]}, {})
            assert result is True

    @pytest.mark.asyncio
    async def test_validate_outside_allowed_hours(self) -> None:
        """Test that validate returns False outside allowed hours."""
        condition = AllowedHoursCondition()

        mock_dt = datetime(2026, 1, 28, 20, 0, 0)  # 20:00
        with patch.object(condition, "_get_current_time", return_value=mock_dt):
            result = await condition.validate({"allowed_hours": [9, 10, 11]}, {})
            assert result is False

    @pytest.mark.asyncio
    async def test_validate_no_hours_specified_returns_true(self) -> None:
        """Test that validate returns True when no hours are specified."""
        condition = AllowedHoursCondition()
        result = await condition.validate({}, {})
        assert result is True

    @pytest.mark.asyncio
    async def test_evaluate_returns_violations_outside_hours(self) -> None:
        """Test that evaluate returns violations outside allowed hours."""
        condition = AllowedHoursCondition()

        mock_dt = datetime(2026, 1, 28, 23, 0, 0)  # 23:00
        with patch.object(condition, "_get_current_time", return_value=mock_dt):
            context = {"parameters": {"allowed_hours": [9, 10, 11]}, "event": {}}
            violations = await condition.evaluate(context)
            assert len(violations) == 1
            assert "outside allowed hours" in violations[0].message

    @pytest.mark.asyncio
    async def test_evaluate_returns_empty_within_hours(self) -> None:
        """Test that evaluate returns empty list within allowed hours."""
        condition = AllowedHoursCondition()

        mock_dt = datetime(2026, 1, 28, 10, 0, 0)  # 10:00
        with patch.object(condition, "_get_current_time", return_value=mock_dt):
            context = {"parameters": {"allowed_hours": [9, 10, 11]}, "event": {}}
            violations = await condition.evaluate(context)
            assert len(violations) == 0


class TestDaysCondition:
    """Tests for DaysCondition class."""

    @pytest.mark.asyncio
    async def test_validate_merge_on_unrestricted_day(self) -> None:
        """Test that validate returns True when merged on unrestricted day."""
        condition = DaysCondition()

        event = {"pull_request_details": {"merged_at": "2026-01-28T10:00:00Z"}}  # Wednesday

        result = await condition.validate({"days": ["Friday", "Saturday"]}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_merge_on_restricted_day(self) -> None:
        """Test that validate returns False when merged on restricted day."""
        condition = DaysCondition()

        event = {"pull_request_details": {"merged_at": "2026-01-30T10:00:00Z"}}  # Friday

        result = await condition.validate({"days": ["Friday", "Saturday"]}, event)
        assert result is False

    @pytest.mark.asyncio
    async def test_validate_no_merged_at_returns_true(self) -> None:
        """Test that validate returns True when PR is not merged."""
        condition = DaysCondition()

        event = {"pull_request_details": {"merged_at": None}}

        result = await condition.validate({"days": ["Friday"]}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_no_days_specified_returns_true(self) -> None:
        """Test that validate returns True when no days are specified."""
        condition = DaysCondition()

        event = {"pull_request_details": {"merged_at": "2026-01-30T10:00:00Z"}}  # Friday

        result = await condition.validate({"days": []}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_no_pr_details_returns_true(self) -> None:
        """Test that validate returns True when PR details are missing."""
        condition = DaysCondition()
        result = await condition.validate({"days": ["Friday"]}, {})
        assert result is True

    @pytest.mark.asyncio
    async def test_evaluate_returns_violations_on_restricted_day(self) -> None:
        """Test that evaluate returns violations when merged on restricted day."""
        condition = DaysCondition()

        event = {"pull_request_details": {"merged_at": "2026-01-30T10:00:00Z"}}  # Friday
        context = {"parameters": {"days": ["Friday"]}, "event": event}

        violations = await condition.evaluate(context)
        assert len(violations) == 1
        assert "restricted day" in violations[0].message.lower()

    @pytest.mark.asyncio
    async def test_evaluate_returns_empty_on_unrestricted_day(self) -> None:
        """Test that evaluate returns empty list on unrestricted day."""
        condition = DaysCondition()

        event = {"pull_request_details": {"merged_at": "2026-01-28T10:00:00Z"}}  # Wednesday
        context = {"parameters": {"days": ["Friday"]}, "event": event}

        violations = await condition.evaluate(context)
        assert len(violations) == 0
