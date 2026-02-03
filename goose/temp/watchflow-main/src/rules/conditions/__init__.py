"""Conditions package for rule validation.

This package contains modular condition classes extracted from validators.py.
Each module focuses on a specific domain of validation.
"""

from src.rules.conditions.access_control import (
    AuthorTeamCondition,
    CodeOwnersCondition,
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

__all__ = [
    # Base
    "BaseCondition",
    # Filesystem
    "FilePatternCondition",
    "MaxFileSizeCondition",
    "MaxPrLocCondition",
    # Pull Request
    "TitlePatternCondition",
    "MinDescriptionLengthCondition",
    "RequireLinkedIssueCondition",
    "RequiredLabelsCondition",
    # Access Control
    "AuthorTeamCondition",
    "CodeOwnersCondition",
    "PathHasCodeOwnerCondition",
    "ProtectedBranchesCondition",
    "RequireCodeOwnerReviewersCondition",
    # Temporal
    "AllowedHoursCondition",
    "DaysCondition",
    "WeekendCondition",
    # Workflow
    "WorkflowDurationCondition",
]
