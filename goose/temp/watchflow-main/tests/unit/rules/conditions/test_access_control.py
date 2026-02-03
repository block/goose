"""Tests for access control conditions.

Tests for AuthorTeamCondition, CodeOwnersCondition, PathHasCodeOwnerCondition,
RequireCodeOwnerReviewersCondition, and ProtectedBranchesCondition classes.
"""

from unittest.mock import patch

import pytest

from src.rules.conditions.access_control import (
    AuthorTeamCondition,
    CodeOwnersCondition,
    NoForcePushCondition,
    PathHasCodeOwnerCondition,
    ProtectedBranchesCondition,
    RequireCodeOwnerReviewersCondition,
)


class TestNoForcePushCondition:
    """Tests for NoForcePushCondition class."""

    @pytest.mark.asyncio
    async def test_validate_force_push(self) -> None:
        """Test that validate returns False when force push is detected."""
        condition = NoForcePushCondition()

        event = {"push": {"forced": True}}

        result = await condition.validate({}, event)
        assert result is False

    @pytest.mark.asyncio
    async def test_validate_normal_push(self) -> None:
        """Test that validate returns True when normal push is performed."""
        condition = NoForcePushCondition()

        event = {"push": {"forced": False}}

        result = await condition.validate({}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_evaluate_returns_violations_for_force_push(self) -> None:
        """Test that evaluate returns violations for force push."""
        condition = NoForcePushCondition()

        event = {"push": {"forced": True}}
        context = {"parameters": {}, "event": event}

        violations = await condition.evaluate(context)
        assert len(violations) == 1
        assert "Force push detected" in violations[0].message

    @pytest.mark.asyncio
    async def test_evaluate_returns_empty_for_normal_push(self) -> None:
        """Test that evaluate returns empty list for normal push."""
        condition = NoForcePushCondition()

        event = {"push": {"forced": False}}
        context = {"parameters": {}, "event": event}

        violations = await condition.evaluate(context)
        assert len(violations) == 0


class TestAuthorTeamCondition:
    """Tests for AuthorTeamCondition class."""

    @pytest.mark.asyncio
    async def test_validate_member_of_team(self) -> None:
        """Test that validate returns True when author is team member."""
        condition = AuthorTeamCondition()

        event = {"sender": {"login": "devops-user"}}

        result = await condition.validate({"team": "devops"}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_not_member_of_team(self) -> None:
        """Test that validate returns False when author is not team member."""
        condition = AuthorTeamCondition()

        event = {"sender": {"login": "random-user"}}

        result = await condition.validate({"team": "devops"}, event)
        assert result is False

    @pytest.mark.asyncio
    async def test_validate_no_team_specified(self) -> None:
        """Test that validate returns False when no team is specified."""
        condition = AuthorTeamCondition()

        event = {"sender": {"login": "devops-user"}}

        result = await condition.validate({}, event)
        assert result is False

    @pytest.mark.asyncio
    async def test_validate_no_sender_in_event(self) -> None:
        """Test that validate returns False when sender is missing."""
        condition = AuthorTeamCondition()

        result = await condition.validate({"team": "devops"}, {})
        assert result is False

    @pytest.mark.asyncio
    async def test_evaluate_returns_violations_for_non_member(self) -> None:
        """Test that evaluate returns violations when author is not team member."""
        with patch("src.rules.conditions.access_control.DEFAULT_TEAM_MEMBERSHIPS", {"devops": ["devops-user"]}):
            condition = AuthorTeamCondition()

            event = {"sender": {"login": "random-user"}}
            context = {"parameters": {"team": "devops"}, "event": event}

            violations = await condition.evaluate(context)
            assert len(violations) == 1
            assert "not a member of team" in violations[0].message

    @pytest.mark.asyncio
    async def test_evaluate_returns_empty_for_member(self) -> None:
        """Test that evaluate returns empty list when author is team member."""
        with patch("src.rules.conditions.access_control.DEFAULT_TEAM_MEMBERSHIPS", {"devops": ["devops-user"]}):
            condition = AuthorTeamCondition()

            event = {"sender": {"login": "devops-user"}}
            context = {"parameters": {"team": "devops"}, "event": event}

            violations = await condition.evaluate(context)
            assert len(violations) == 0


class TestCodeOwnersCondition:
    """Tests for CodeOwnersCondition class."""

    @pytest.mark.asyncio
    async def test_validate_no_files(self) -> None:
        """Test that validate returns True when no files are present."""
        condition = CodeOwnersCondition()
        result = await condition.validate({}, {"files": []})
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_with_critical_files(self) -> None:
        """Test that validate returns False when critical files are modified."""
        condition = CodeOwnersCondition()

        event = {"files": [{"filename": "src/critical.py"}]}

        with patch("src.rules.utils.codeowners.is_critical_file", return_value=True):
            result = await condition.validate({"critical_owners": ["admin"]}, event)
            assert result is False

    @pytest.mark.asyncio
    async def test_validate_without_critical_files(self) -> None:
        """Test that validate returns True when no critical files are modified."""
        condition = CodeOwnersCondition()

        event = {"files": [{"filename": "src/normal.py"}]}

        with patch("src.rules.utils.codeowners.is_critical_file", return_value=False):
            result = await condition.validate({"critical_owners": ["admin"]}, event)
            assert result is True

    @pytest.mark.asyncio
    async def test_evaluate_returns_violations_for_critical_files(self) -> None:
        """Test that evaluate returns violations for critical files."""
        condition = CodeOwnersCondition()

        event = {"files": [{"filename": "src/critical.py"}]}
        context = {"parameters": {"critical_owners": ["admin"]}, "event": event}

        with patch("src.rules.utils.codeowners.is_critical_file", return_value=True):
            violations = await condition.evaluate(context)
            assert len(violations) == 1
            assert "code owner review" in violations[0].message


class TestPathHasCodeOwnerCondition:
    """Tests for PathHasCodeOwnerCondition class."""

    CODEOWNERS_WITH_PY = "# Owners\n*.py @py-owners\nsrc/ @src-team\n"
    CODEOWNERS_ROOT_ONLY = "/ @root-team\n"

    @pytest.mark.asyncio
    async def test_validate_no_files(self) -> None:
        """When no files are present, validate returns True."""
        condition = PathHasCodeOwnerCondition()
        result = await condition.validate({"require_path_has_code_owner": True}, {"files": []})
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_no_codeowners_content_skips(self) -> None:
        """When event has no codeowners_content, condition passes (rule not applicable)."""
        condition = PathHasCodeOwnerCondition()
        event = {"files": [{"filename": "foo/bar.py"}]}
        result = await condition.validate({"require_path_has_code_owner": True}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_all_paths_have_owner(self) -> None:
        """When all changed paths match CODEOWNERS, validate returns True."""
        condition = PathHasCodeOwnerCondition()
        event = {
            "codeowners_content": self.CODEOWNERS_WITH_PY,
            "files": [{"filename": "src/main.py"}, {"filename": "README.py"}],
        }
        result = await condition.validate({"require_path_has_code_owner": True}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_some_paths_without_owner(self) -> None:
        """When some changed paths have no owner in CODEOWNERS, validate returns False."""
        condition = PathHasCodeOwnerCondition()
        event = {
            "codeowners_content": "/docs/ @docs-team\n",
            "files": [{"filename": "docs/readme.md"}, {"filename": "src/foo.py"}],
        }
        result = await condition.validate({"require_path_has_code_owner": True}, event)
        assert result is False

    @pytest.mark.asyncio
    async def test_evaluate_returns_violation_for_unowned_paths(self) -> None:
        """evaluate returns a violation listing paths without code owner."""
        condition = PathHasCodeOwnerCondition()
        event = {
            "codeowners_content": "/docs/ @docs\n",
            "files": [{"filename": "docs/a.md"}, {"filename": "src/bar.py"}],
        }
        context = {"parameters": {"require_path_has_code_owner": True}, "event": event}
        violations = await condition.evaluate(context)
        assert len(violations) == 1
        assert "Paths without a code owner" in violations[0].message
        assert "src/bar.py" in violations[0].message


class TestRequireCodeOwnerReviewersCondition:
    """Tests for RequireCodeOwnerReviewersCondition class."""

    # Use "docs/" (no leading slash) so path "docs/a.md" matches
    CODEOWNERS_DOCS = "docs/ @docs-team\n"
    CODEOWNERS_DOCS_AND_ALICE = "docs/ @docs-team @alice\n*.py @alice\n"

    @pytest.mark.asyncio
    async def test_validate_no_codeowners_skips(self) -> None:
        """When event has no codeowners_content, condition passes (rule not applicable)."""
        condition = RequireCodeOwnerReviewersCondition()
        event = {"files": [{"filename": "docs/readme.md"}]}
        result = await condition.validate({"require_code_owner_reviewers": True}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_no_changed_files_passes(self) -> None:
        """When no changed files, validate returns True."""
        condition = RequireCodeOwnerReviewersCondition()
        event = {"codeowners_content": self.CODEOWNERS_DOCS, "files": []}
        result = await condition.validate({"require_code_owner_reviewers": True}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_required_owners_already_requested_passes(self) -> None:
        """When all required code owners are in requested_reviewers/requested_teams, validate returns True."""
        condition = RequireCodeOwnerReviewersCondition()
        event = {
            "codeowners_content": self.CODEOWNERS_DOCS_AND_ALICE,
            "files": [{"filename": "docs/a.md"}, {"filename": "src/foo.py"}],
            "pull_request_details": {
                "requested_reviewers": [{"login": "alice"}],
                "requested_teams": [{"slug": "docs-team"}],
            },
        }
        result = await condition.validate({"require_code_owner_reviewers": True}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_missing_reviewer_fails(self) -> None:
        """When a required code owner is not requested, validate returns False."""
        condition = RequireCodeOwnerReviewersCondition()
        event = {
            "codeowners_content": self.CODEOWNERS_DOCS_AND_ALICE,
            "files": [{"filename": "docs/a.md"}],
            "pull_request_details": {"requested_reviewers": [], "requested_teams": []},
        }
        result = await condition.validate({"require_code_owner_reviewers": True}, event)
        assert result is False

    @pytest.mark.asyncio
    async def test_evaluate_returns_violation_with_missing_reviewers(self) -> None:
        """evaluate returns a violation listing code owners that must be added as reviewers."""
        condition = RequireCodeOwnerReviewersCondition()
        event = {
            "codeowners_content": self.CODEOWNERS_DOCS_AND_ALICE,
            "files": [{"filename": "src/bar.py"}],
            "pull_request_details": {"requested_reviewers": [], "requested_teams": []},
        }
        context = {"parameters": {"require_code_owner_reviewers": True}, "event": event}
        violations = await condition.evaluate(context)
        assert len(violations) == 1
        assert "Code owners for modified paths must be added as reviewers" in violations[0].message
        assert "alice" in violations[0].message

    @pytest.mark.asyncio
    async def test_evaluate_no_pull_request_details_treats_no_reviewers_requested(self) -> None:
        """When pull_request_details is missing, required owners are considered missing (violation)."""
        condition = RequireCodeOwnerReviewersCondition()
        event = {
            "codeowners_content": self.CODEOWNERS_DOCS_AND_ALICE,
            "files": [{"filename": "src/bar.py"}],
        }
        context = {"parameters": {"require_code_owner_reviewers": True}, "event": event}
        violations = await condition.evaluate(context)
        assert len(violations) == 1
        assert "alice" in violations[0].message


class TestProtectedBranchesCondition:
    """Tests for ProtectedBranchesCondition class."""

    @pytest.mark.asyncio
    async def test_validate_non_protected_branch(self) -> None:
        """Test that validate returns True for non-protected branches."""
        condition = ProtectedBranchesCondition()

        event = {"pull_request_details": {"base": {"ref": "feature-branch"}}}

        result = await condition.validate({"protected_branches": ["main", "develop"]}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_protected_branch(self) -> None:
        """Test that validate returns False for protected branches."""
        condition = ProtectedBranchesCondition()

        event = {"pull_request_details": {"base": {"ref": "main"}}}

        result = await condition.validate({"protected_branches": ["main", "develop"]}, event)
        assert result is False

    @pytest.mark.asyncio
    async def test_validate_no_protected_branches_specified(self) -> None:
        """Test that validate returns True when no protected branches specified."""
        condition = ProtectedBranchesCondition()

        event = {"pull_request_details": {"base": {"ref": "main"}}}

        result = await condition.validate({"protected_branches": []}, event)
        assert result is True

    @pytest.mark.asyncio
    async def test_validate_no_pr_details(self) -> None:
        """Test that validate returns True when PR details are missing."""
        condition = ProtectedBranchesCondition()
        result = await condition.validate({"protected_branches": ["main"]}, {})
        assert result is True

    @pytest.mark.asyncio
    async def test_evaluate_returns_violations_for_protected(self) -> None:
        """Test that evaluate returns violations for protected branches."""
        condition = ProtectedBranchesCondition()

        event = {"pull_request_details": {"base": {"ref": "main"}}}
        context = {"parameters": {"protected_branches": ["main"]}, "event": event}

        violations = await condition.evaluate(context)
        assert len(violations) == 1
        assert "protected branch" in violations[0].message

    @pytest.mark.asyncio
    async def test_evaluate_returns_empty_for_non_protected(self) -> None:
        """Test that evaluate returns empty list for non-protected branches."""
        condition = ProtectedBranchesCondition()

        event = {"pull_request_details": {"base": {"ref": "feature"}}}
        context = {"parameters": {"protected_branches": ["main"]}, "event": event}

        violations = await condition.evaluate(context)
        assert len(violations) == 0
