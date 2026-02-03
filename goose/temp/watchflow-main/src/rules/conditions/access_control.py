"""Access control conditions for rule validation.

This module contains conditions that validate security and access control
aspects like team membership, code ownership, and branch protection.
"""

from typing import Any, cast

import structlog

from src.core.constants import DEFAULT_TEAM_MEMBERSHIPS
from src.core.models import Severity, Violation
from src.rules.conditions.base import BaseCondition

logger = structlog.get_logger(__name__)


class AuthorTeamCondition(BaseCondition):
    """Validates if the event author is a member of a specific team."""

    name = "author_team_is"
    description = "Validates if the event author is a member of a specific team"
    parameter_patterns = ["team"]
    event_types = ["pull_request", "push", "deployment"]
    examples = [{"team": "devops"}, {"team": "codeowners"}]

    async def evaluate(self, context: Any) -> list[Violation]:
        """Evaluate team membership condition.

        Args:
            context: Dict with 'parameters' and 'event' keys.

        Returns:
            List of violations if author is not in the required team.
        """
        parameters = context.get("parameters", {})
        event = context.get("event", {})

        team_name = parameters.get("team")
        if not team_name:
            logger.warning("AuthorTeamCondition: No team specified in parameters")
            return [
                Violation(
                    rule_description=self.description,
                    severity=Severity.MEDIUM,
                    message="No team specified in rule parameters",
                    how_to_fix="Provide a 'team' parameter in the rule configuration.",
                )
            ]

        author_login = event.get("sender", {}).get("login", "")
        if not author_login:
            logger.warning("AuthorTeamCondition: No sender login found in event")
            return [
                Violation(
                    rule_description=self.description,
                    severity=Severity.INFO,
                    message="Unable to determine event author",
                )
            ]

        logger.debug("Checking team membership", author=author_login, team=team_name)

        # Use constants from src.core.constants
        team_memberships = DEFAULT_TEAM_MEMBERSHIPS
        is_member = author_login in team_memberships.get(team_name, [])

        if not is_member:
            return [
                Violation(
                    rule_description=self.description,
                    severity=Severity.HIGH,
                    message=f"Author '{author_login}' is not a member of team '{team_name}'",
                    details={"author": author_login, "required_team": team_name},
                    how_to_fix=f"Request a team member from '{team_name}' to perform this action.",
                )
            ]

        return []

    async def validate(self, parameters: dict[str, Any], event: dict[str, Any]) -> bool:
        """Legacy validation interface for backward compatibility."""
        team_name = parameters.get("team")
        if not team_name:
            logger.warning("AuthorTeamCondition: No team specified in parameters")
            return False

        author_login = event.get("sender", {}).get("login", "")
        if not author_login:
            logger.warning("AuthorTeamCondition: No sender login found in event")
            return False

        logger.debug("Checking team membership", author=author_login, team=team_name)

        team_memberships = DEFAULT_TEAM_MEMBERSHIPS
        return author_login in team_memberships.get(team_name, [])


class CodeOwnersCondition(BaseCondition):
    """Validates if changes to files require review from code owners."""

    name = "code_owners"
    description = "Validates if changes to files require review from code owners"
    parameter_patterns = ["critical_owners"]
    event_types = ["pull_request"]
    examples = [
        {"critical_owners": ["admin", "maintainers"]},
        {"critical_owners": ["security-team", "devops"]},
        {},  # No critical_owners means any file with owners is critical
    ]

    async def evaluate(self, context: Any) -> list[Violation]:
        """Evaluate code owners condition.

        Args:
            context: Dict with 'parameters' and 'event' keys.

        Returns:
            List of violations if code owner review is required but not present.
        """
        parameters = context.get("parameters", {})
        event = context.get("event", {})

        changed_files = self._get_changed_files(event)
        if not changed_files:
            logger.debug("CodeOwnersCondition: No files to check")
            return []

        from src.rules.utils.codeowners import is_critical_file

        critical_owners = parameters.get("critical_owners")

        critical_files = [
            file_path for file_path in changed_files if is_critical_file(file_path, critical_owners=critical_owners)
        ]

        if critical_files:
            return [
                Violation(
                    rule_description=self.description,
                    severity=Severity.HIGH,
                    message=f"Changes to critical files require code owner review: {', '.join(critical_files)}",
                    details={"critical_files": critical_files, "critical_owners": critical_owners},
                    how_to_fix="Request a review from a designated code owner before merging.",
                )
            ]

        return []

    async def validate(self, parameters: dict[str, Any], event: dict[str, Any]) -> bool:
        """Legacy validation interface for backward compatibility."""
        changed_files = self._get_changed_files(event)
        if not changed_files:
            logger.debug("CodeOwnersCondition: No files to check")
            return True

        from src.rules.utils.codeowners import is_critical_file

        critical_owners = parameters.get("critical_owners")

        requires_code_owner_review = any(
            is_critical_file(file_path, critical_owners=critical_owners) for file_path in changed_files
        )

        logger.debug(
            "CodeOwnersCondition: Files checked",
            files=changed_files,
            requires_review=requires_code_owner_review,
        )
        return not requires_code_owner_review

    def _get_changed_files(self, event: dict[str, Any]) -> list[str]:
        """Extract changed files from the event."""
        files = event.get("files", [])
        if files:
            return [file.get("filename", "") for file in files if file.get("filename")]

        pull_request = event.get("pull_request_details", {})
        if pull_request:
            from typing import cast

            return cast("list[str]", pull_request.get("changed_files", []))

        return []


def _get_changed_files_from_event(event: dict[str, Any]) -> list[str]:
    """Extract changed file paths from the event (shared by path-has-code-owner)."""
    files = event.get("files", [])
    if files:
        return [f.get("filename", "") for f in files if f.get("filename")]
    changed = event.get("changed_files", [])
    if changed:
        return [
            f.get("filename", f) if isinstance(f, dict) else f
            for f in changed
            if (f.get("filename") if isinstance(f, dict) else f)
        ]
    pull_request = event.get("pull_request_details", {})
    if pull_request:
        return cast("list[str]", pull_request.get("changed_files", []))
    return []


class PathHasCodeOwnerCondition(BaseCondition):
    """Validates that every changed path has a code owner defined in CODEOWNERS."""

    name = "require_path_has_code_owner"
    description = "Validates that every changed path has a code owner defined in the CODEOWNERS file"
    parameter_patterns = ["require_path_has_code_owner"]
    event_types = ["pull_request"]
    examples = [{"require_path_has_code_owner": True}]

    async def evaluate(self, context: Any) -> list[Violation]:
        """Evaluate path-has-code-owner condition.

        Args:
            context: Dict with 'parameters' and 'event' keys. Event may include
                codeowners_content (str) when enricher has fetched CODEOWNERS.

        Returns:
            List of violations if any changed path has no code owner defined.
        """
        event = context.get("event", {})
        changed_files = _get_changed_files_from_event(event)
        if not changed_files:
            logger.debug("PathHasCodeOwnerCondition: No files to check")
            return []

        codeowners_content = event.get("codeowners_content")
        if not codeowners_content:
            logger.debug("PathHasCodeOwnerCondition: No CODEOWNERS content in event, skipping")
            return []

        from src.rules.utils.codeowners import path_has_owner

        unowned = [p for p in changed_files if not path_has_owner(p, codeowners_content)]
        if not unowned:
            return []

        return [
            Violation(
                rule_description=self.description,
                severity=Severity.HIGH,
                message=f"Paths without a code owner in CODEOWNERS: {', '.join(unowned)}",
                details={"unowned_paths": unowned},
                how_to_fix="Add entries for these paths in the repository CODEOWNERS file.",
            )
        ]

    async def validate(self, parameters: dict[str, Any], event: dict[str, Any]) -> bool:
        """Legacy validation interface for backward compatibility."""
        changed_files = _get_changed_files_from_event(event)
        if not changed_files:
            return True

        codeowners_content = event.get("codeowners_content")
        if not codeowners_content:
            return True

        from src.rules.utils.codeowners import path_has_owner

        unowned = [p for p in changed_files if not path_has_owner(p, codeowners_content)]
        logger.debug(
            "PathHasCodeOwnerCondition: paths checked",
            changed=changed_files,
            unowned=unowned,
        )
        return len(unowned) == 0


def _required_code_owner_reviewers(event: dict[str, Any]) -> tuple[list[str], list[str]]:
    """
    Return (required_owners, missing_owners) for the code-owner-reviewers rule.

    required_owners: all code owner logins/teams that own at least one changed path.
    missing_owners: subset of required_owners that are not in the PR's requested reviewers/teams.
    """
    changed_files = _get_changed_files_from_event(event)
    codeowners_content = event.get("codeowners_content")
    if not changed_files or not codeowners_content:
        return ([], [])

    from src.rules.utils.codeowners import CodeOwnersParser

    parser = CodeOwnersParser(codeowners_content)
    required: set[str] = set()
    for path in changed_files:
        owners = parser.get_owners_for_file(path)
        required.update(owners)

    if not required:
        return ([], [])

    pr = event.get("pull_request_details", {})
    requested_users = pr.get("requested_reviewers") or []
    requested_teams = pr.get("requested_teams") or []
    requested_logins = {u.get("login") for u in requested_users if u.get("login")}
    requested_slugs = {t.get("slug") for t in requested_teams if t.get("slug")}

    # Owner can be a user (login) or a team (slug or org/slug). Match user by login, team by slug.
    requested_identifiers = requested_logins | requested_slugs

    missing: list[str] = []
    for owner in sorted(required):
        if "/" in owner:
            # Team: CODEOWNERS has "org/team-name", API has slug "team-name"
            slug = owner.split("/")[-1]
            if slug not in requested_slugs:
                missing.append(owner)
        else:
            # User or team slug (e.g. @docs-team); match if in requested reviewers or requested teams
            if owner not in requested_identifiers:
                missing.append(owner)

    return (sorted(required), missing)


class RequireCodeOwnerReviewersCondition(BaseCondition):
    """Validates that when a PR modifies paths with CODEOWNERS, those owners are requested as reviewers."""

    name = "require_code_owner_reviewers"
    description = "When a PR modifies paths that have owners defined in CODEOWNERS, the corresponding code owners must be added as reviewers"
    parameter_patterns = ["require_code_owner_reviewers"]
    event_types = ["pull_request"]
    examples = [{"require_code_owner_reviewers": True}]

    async def evaluate(self, context: Any) -> list[Violation]:
        """Evaluate require-code-owner-reviewers condition.

        Args:
            context: Dict with 'parameters' and 'event' keys. Event must include
                codeowners_content and pull_request_details.requested_reviewers / requested_teams.

        Returns:
            List of violations if required code owners are not requested as reviewers.
        """
        event = context.get("event", {})
        required, missing = _required_code_owner_reviewers(event)
        if not missing:
            return []

        return [
            Violation(
                rule_description=self.description,
                severity=Severity.HIGH,
                message=f"Code owners for modified paths must be added as reviewers: {', '.join(missing)}",
                details={"missing_reviewers": missing, "required_owners": required},
                how_to_fix="Add the listed code owners as requested reviewers on the PR.",
            )
        ]

    async def validate(self, parameters: dict[str, Any], event: dict[str, Any]) -> bool:
        """Legacy validation interface for backward compatibility."""
        _, missing = _required_code_owner_reviewers(event)
        logger.debug(
            "RequireCodeOwnerReviewersCondition: required vs requested",
            missing=missing,
        )
        return len(missing) == 0


class ProtectedBranchesCondition(BaseCondition):
    """Validates if the PR targets protected branches."""

    name = "protected_branches"
    description = "Validates if the PR targets protected branches"
    parameter_patterns = ["protected_branches"]
    event_types = ["pull_request"]
    examples = [{"protected_branches": ["main", "develop"]}, {"protected_branches": ["master"]}]

    async def evaluate(self, context: Any) -> list[Violation]:
        """Evaluate protected branches condition.

        Args:
            context: Dict with 'parameters' and 'event' keys.

        Returns:
            List of violations if PR targets a protected branch.
        """
        parameters = context.get("parameters", {})
        event = context.get("event", {})

        protected_branches = parameters.get("protected_branches", [])
        if not protected_branches:
            return []

        pull_request = event.get("pull_request_details", {})
        if not pull_request:
            return []

        base_branch = pull_request.get("base", {}).get("ref", "")
        is_protected = base_branch in protected_branches

        logger.debug(
            "ProtectedBranchesCondition: Checking branch",
            base_branch=base_branch,
            protected_branches=protected_branches,
            is_protected=is_protected,
        )

        if is_protected:
            return [
                Violation(
                    rule_description=self.description,
                    severity=Severity.HIGH,
                    message=f"PR targets protected branch '{base_branch}'",
                    details={"base_branch": base_branch, "protected_branches": protected_branches},
                    how_to_fix="Ensure additional review requirements are met for protected branches.",
                )
            ]

        return []

    async def validate(self, parameters: dict[str, Any], event: dict[str, Any]) -> bool:
        """Legacy validation interface for backward compatibility."""
        protected_branches = parameters.get("protected_branches", [])
        if not protected_branches:
            return True

        pull_request = event.get("pull_request_details", {})
        if not pull_request:
            return True

        base_branch = pull_request.get("base", {}).get("ref", "")
        is_protected = base_branch in protected_branches

        logger.debug(
            "ProtectedBranchesCondition: Checking branch",
            base_branch=base_branch,
            protected_branches=protected_branches,
            is_protected=is_protected,
        )

        return not is_protected


class NoForcePushCondition(BaseCondition):
    """Validates that no force pushes are performed."""

    name = "no_force_push"
    description = "Validates that no force pushes are performed"
    parameter_patterns = ["no_force_push"]
    event_types = ["push"]

    async def evaluate(self, context: Any) -> list[Violation]:
        """Evaluate no force push condition.

        Args:
             context: Dict with 'parameters' and 'event' keys.

        Returns:
            List of violations if force push is detected.
        """
        event = context.get("event", {})
        push_data = event.get("push", {})

        is_forced = push_data.get("forced", False)

        if is_forced:
            return [
                Violation(
                    rule_description=self.description,
                    severity=Severity.HIGH,
                    message="Force push detected on protected branch",
                    how_to_fix="Avoid force pushing to shared branches. Revert and push clean history.",
                )
            ]
        return []

    async def validate(self, parameters: dict[str, Any], event: dict[str, Any]) -> bool:
        """Legacy validation interface."""
        push_data = event.get("push", {})
        return not push_data.get("forced", False)
