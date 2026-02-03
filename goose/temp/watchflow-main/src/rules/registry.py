"""
Registry for rule conditions.

This module maps RuleIDs and parameters to their corresponding Condition classes,
enabling dynamic loading and execution of rules.
"""

import logging

from src.rules.acknowledgment import RuleID
from src.rules.conditions.access_control import (
    AuthorTeamCondition,
    CodeOwnersCondition,
    NoForcePushCondition,
    PathHasCodeOwnerCondition,
    ProtectedBranchesCondition,
    RequireCodeOwnerReviewersCondition,
)
from src.rules.conditions.base import BaseCondition
from src.rules.conditions.filesystem import (
    FilePatternCondition,
    MaxFileSizeCondition,
    MaxPrLocCondition,
)
from src.rules.conditions.pull_request import (
    MinApprovalsCondition,
    MinDescriptionLengthCondition,
    RequiredLabelsCondition,
    RequireLinkedIssueCondition,
    TitlePatternCondition,
)
from src.rules.conditions.temporal import (
    AllowedHoursCondition,
    DaysCondition,
    WeekendCondition,
)
from src.rules.conditions.workflow import WorkflowDurationCondition

logger = logging.getLogger(__name__)

# Map RuleID to Condition classes
RULE_ID_TO_CONDITION: dict[RuleID, type[BaseCondition]] = {
    RuleID.REQUIRED_LABELS: RequiredLabelsCondition,
    RuleID.PR_TITLE_PATTERN: TitlePatternCondition,
    RuleID.PR_DESCRIPTION_REQUIRED: MinDescriptionLengthCondition,
    RuleID.FILE_SIZE_LIMIT: MaxFileSizeCondition,
    RuleID.MAX_PR_LOC: MaxPrLocCondition,
    RuleID.REQUIRE_LINKED_ISSUE: RequireLinkedIssueCondition,
    RuleID.PROTECTED_BRANCH_PUSH: ProtectedBranchesCondition,
    RuleID.NO_FORCE_PUSH: NoForcePushCondition,
    RuleID.MIN_PR_APPROVALS: MinApprovalsCondition,
    RuleID.PATH_HAS_CODE_OWNER: PathHasCodeOwnerCondition,
    RuleID.REQUIRE_CODE_OWNER_REVIEWERS: RequireCodeOwnerReviewersCondition,
}

# Reverse map: condition class -> RuleID (for populating rule_id on violations)
CONDITION_CLASS_TO_RULE_ID: dict[type[BaseCondition], RuleID] = {cls: rid for rid, cls in RULE_ID_TO_CONDITION.items()}

# List of all available condition classes
AVAILABLE_CONDITIONS: list[type[BaseCondition]] = [
    RequiredLabelsCondition,
    TitlePatternCondition,
    MinDescriptionLengthCondition,
    RequireLinkedIssueCondition,
    MaxFileSizeCondition,
    MaxPrLocCondition,
    MinApprovalsCondition,
    ProtectedBranchesCondition,
    AuthorTeamCondition,
    CodeOwnersCondition,
    PathHasCodeOwnerCondition,
    RequireCodeOwnerReviewersCondition,
    FilePatternCondition,
    AllowedHoursCondition,
    DaysCondition,
    WeekendCondition,
    WorkflowDurationCondition,
]


class ConditionRegistry:
    """Registry for looking up and instantiating rule conditions."""

    @staticmethod
    def get_condition_class_by_id(rule_id: RuleID) -> type[BaseCondition] | None:
        """Get condition class by RuleID."""
        return RULE_ID_TO_CONDITION.get(rule_id)

    @staticmethod
    def get_conditions_for_parameters(parameters: dict) -> list[BaseCondition]:
        """
        Identify and instantiate conditions based on available parameters.

        Args:
            parameters: Dictionary of parameters from the rule definition.

        Returns:
            List of instantiated BaseCondition objects that match the parameters.
        """
        matched_conditions = []

        for condition_cls in AVAILABLE_CONDITIONS:
            # Check if condition's parameter patterns exist in the rule parameters
            # If a condition has no patterns, it can't be inferred solely from parameters
            if not condition_cls.parameter_patterns:
                continue

            # Check if ANY of the condition's parameter patterns match keys in parameters
            # This is a heuristic; might need refinement for strict matching
            if any(key in parameters for key in condition_cls.parameter_patterns):
                try:
                    condition = condition_cls()
                    matched_conditions.append(condition)
                    logger.debug(f"Matches condition: {condition_cls.name}")
                except Exception as e:
                    logger.error(f"Failed to instantiate condition {condition_cls.name}: {e}")

        return matched_conditions
