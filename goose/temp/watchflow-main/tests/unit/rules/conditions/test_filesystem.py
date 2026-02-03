"""Tests for filesystem conditions.

Tests for FilePatternCondition, MaxFileSizeCondition, and MaxPrLocCondition classes.
"""

from unittest.mock import patch

import pytest

from src.rules.conditions.filesystem import (
    FilePatternCondition,
    MaxFileSizeCondition,
    MaxPrLocCondition,
)


class TestFilePatternCondition:
    """Tests for FilePatternCondition class."""

    @pytest.mark.asyncio
    async def test_validate_match_pattern_success(self) -> None:
        """Test that validate returns True when files match pattern."""
        condition = FilePatternCondition()

        with patch.object(condition, "_get_changed_files", return_value=["src/foo.py"]):
            result = await condition.validate({"pattern": "*.py", "condition_type": "files_match_pattern"}, {})
            assert result is True

    @pytest.mark.asyncio
    async def test_validate_match_pattern_no_match(self) -> None:
        """Test that validate returns False when no files match pattern."""
        condition = FilePatternCondition()

        with patch.object(condition, "_get_changed_files", return_value=["src/foo.py"]):
            result = await condition.validate({"pattern": "*.js", "condition_type": "files_match_pattern"}, {})
            assert result is False

    @pytest.mark.asyncio
    async def test_validate_not_match_pattern_success(self) -> None:
        """Test files_not_match_pattern returns True when no files match."""
        condition = FilePatternCondition()

        with patch.object(condition, "_get_changed_files", return_value=["src/foo.py"]):
            result = await condition.validate({"pattern": "*.js", "condition_type": "files_not_match_pattern"}, {})
            assert result is True

    @pytest.mark.asyncio
    async def test_validate_not_match_pattern_fails_when_matching(self) -> None:
        """Test files_not_match_pattern returns False when files do match."""
        condition = FilePatternCondition()

        with patch.object(condition, "_get_changed_files", return_value=["src/foo.py"]):
            result = await condition.validate({"pattern": "*.py", "condition_type": "files_not_match_pattern"}, {})
            assert result is False

    @pytest.mark.asyncio
    async def test_validate_no_pattern_returns_false(self) -> None:
        """Test that validate returns False when no pattern is specified."""
        condition = FilePatternCondition()
        result = await condition.validate({}, {})
        assert result is False

    @pytest.mark.asyncio
    async def test_validate_no_files_returns_false(self) -> None:
        """Test that validate returns False when no files are available."""
        condition = FilePatternCondition()

        with patch.object(condition, "_get_changed_files", return_value=[]):
            result = await condition.validate({"pattern": "*.py"}, {})
            assert result is False

    @pytest.mark.asyncio
    async def test_evaluate_returns_violations_on_failure(self) -> None:
        """Test that evaluate returns violations when pattern matching fails."""
        condition = FilePatternCondition()

        with patch.object(condition, "_get_changed_files", return_value=["src/foo.py"]):
            context = {"parameters": {"pattern": "*.js"}, "event": {}}
            violations = await condition.evaluate(context)
            assert len(violations) == 1
            assert "No files match required pattern" in violations[0].message

    @pytest.mark.asyncio
    async def test_evaluate_returns_empty_on_success(self) -> None:
        """Test that evaluate returns empty list on success."""
        condition = FilePatternCondition()

        with patch.object(condition, "_get_changed_files", return_value=["src/foo.py"]):
            context = {"parameters": {"pattern": "*.py"}, "event": {}}
            violations = await condition.evaluate(context)
            assert len(violations) == 0

    def test_glob_to_regex_conversion(self) -> None:
        """Test glob pattern to regex conversion."""
        assert FilePatternCondition._glob_to_regex("*.py") == "^.*\\.py$"
        assert FilePatternCondition._glob_to_regex("src/*.js") == "^src/.*\\.js$"
        assert FilePatternCondition._glob_to_regex("file?.txt") == "^file.\\.txt$"


class TestMaxFileSizeCondition:
    """Tests for MaxFileSizeCondition class."""

    @pytest.mark.asyncio
    async def test_validate_all_files_under_limit(self) -> None:
        """Test that validate returns True when all files are under size limit."""
        condition = MaxFileSizeCondition()

        event = {
            "files": [
                {"filename": "small.py", "size": 1024},  # 1KB
                {"filename": "medium.py", "size": 1024 * 1024},  # 1MB
            ]
        }

        result = await condition.validate({"max_file_size_mb": 10}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_files_over_limit(self) -> None:
        """Test that validate returns False when files exceed size limit."""
        condition = MaxFileSizeCondition()

        event = {
            "files": [
                {"filename": "small.py", "size": 1024},  # 1KB
                {"filename": "large.bin", "size": 10 * 1024 * 1024 + 1},  # > 10MB
            ]
        }

        result = await condition.validate({"max_file_size_mb": 1}, event)
        assert result is False

    @pytest.mark.asyncio
    async def test_validate_large_limit_passes(self) -> None:
        """Test that large file limit allows oversized files."""
        condition = MaxFileSizeCondition()

        event = {
            "files": [
                {"filename": "large.bin", "size": 10 * 1024 * 1024 + 1},  # > 10MB
            ]
        }

        result = await condition.validate({"max_file_size_mb": 20}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_no_files_returns_true(self) -> None:
        """Test that validate returns True when no files are present."""
        condition = MaxFileSizeCondition()
        result = await condition.validate({"max_file_size_mb": 10}, {"files": []})
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_missing_files_key_returns_true(self) -> None:
        """Test that validate returns True when files key is missing."""
        condition = MaxFileSizeCondition()
        result = await condition.validate({"max_file_size_mb": 10}, {})
        assert result is True

    @pytest.mark.asyncio
    async def test_evaluate_returns_violations_for_oversized_files(self) -> None:
        """Test that evaluate returns violations for oversized files."""
        condition = MaxFileSizeCondition()

        event = {
            "files": [
                {"filename": "large.bin", "size": 20 * 1024 * 1024},  # 20MB
            ]
        }

        context = {"parameters": {"max_file_size_mb": 10}, "event": event}
        violations = await condition.evaluate(context)

        assert len(violations) == 1
        assert "exceed size limit" in violations[0].message
        assert "large.bin" in violations[0].message

    @pytest.mark.asyncio
    async def test_evaluate_returns_empty_for_valid_files(self) -> None:
        """Test that evaluate returns empty list for valid files."""
        condition = MaxFileSizeCondition()

        event = {
            "files": [
                {"filename": "small.py", "size": 1024},  # 1KB
            ]
        }

        context = {"parameters": {"max_file_size_mb": 10}, "event": event}
        violations = await condition.evaluate(context)

        assert len(violations) == 0


class TestMaxPrLocCondition:
    """Tests for MaxPrLocCondition class."""

    @pytest.mark.asyncio
    async def test_evaluate_under_limit(self) -> None:
        """Test that evaluate returns empty when total lines are under limit."""
        condition = MaxPrLocCondition()

        event = {
            "changed_files": [
                {"filename": "a.py", "additions": 100, "deletions": 50},
                {"filename": "b.py", "additions": 200, "deletions": 0},
            ]
        }
        context = {"parameters": {"max_lines": 500}, "event": event}
        violations = await condition.evaluate(context)

        assert len(violations) == 0

    @pytest.mark.asyncio
    async def test_evaluate_over_limit(self) -> None:
        """Test that evaluate returns violation when total lines exceed limit."""
        condition = MaxPrLocCondition()

        event = {
            "changed_files": [
                {"filename": "a.py", "additions": 400, "deletions": 150},
            ]
        }
        context = {"parameters": {"max_lines": 500}, "event": event}
        violations = await condition.evaluate(context)

        assert len(violations) == 1
        assert "Pull request exceeds maximum lines changed" in violations[0].message
        assert "550" in violations[0].message
        assert "500" in violations[0].message
        assert violations[0].details["total_lines"] == 550
        assert violations[0].details["max_lines"] == 500

    @pytest.mark.asyncio
    async def test_evaluate_missing_max_lines_returns_empty(self) -> None:
        """Test that evaluate returns empty when max_lines is 0 or missing."""
        condition = MaxPrLocCondition()

        event = {"changed_files": [{"filename": "a.py", "additions": 1000, "deletions": 500}]}
        context = {"parameters": {}, "event": event}
        violations = await condition.evaluate(context)

        assert len(violations) == 0

        context_no_param = {"parameters": {"max_lines": 0}, "event": event}
        violations2 = await condition.evaluate(context_no_param)
        assert len(violations2) == 0

    @pytest.mark.asyncio
    async def test_evaluate_empty_changed_files_returns_empty(self) -> None:
        """Test that evaluate returns empty when no changed files."""
        condition = MaxPrLocCondition()

        context = {"parameters": {"max_lines": 500}, "event": {"changed_files": []}}
        violations = await condition.evaluate(context)

        assert len(violations) == 0

    @pytest.mark.asyncio
    async def test_evaluate_uses_fallback_files_when_no_changed_files(self) -> None:
        """Test that evaluate uses event.files when changed_files is missing."""
        condition = MaxPrLocCondition()

        event = {
            "files": [
                {"filename": "a.py", "additions": 300, "deletions": 100},
            ]
        }
        context = {"parameters": {"max_lines": 300}, "event": event}
        violations = await condition.evaluate(context)

        assert len(violations) == 1
        assert violations[0].details["total_lines"] == 400

    @pytest.mark.asyncio
    async def test_validate_under_limit(self) -> None:
        """Test that validate returns True when under limit."""
        condition = MaxPrLocCondition()

        event = {"changed_files": [{"filename": "a.py", "additions": 100, "deletions": 50}]}
        result = await condition.validate({"max_lines": 500}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_over_limit(self) -> None:
        """Test that validate returns False when over limit."""
        condition = MaxPrLocCondition()

        event = {"changed_files": [{"filename": "a.py", "additions": 600, "deletions": 0}]}
        result = await condition.validate({"max_lines": 500}, event)
        assert result is False
