"""
Data models for the Intelligent Acknowledgment Agent.
"""

from typing import Any

from pydantic import BaseModel, Field

from src.core.models import Violation


class AcknowledgedViolation(BaseModel):
    """Represents a violation that can be acknowledged."""

    rule_description: str = Field(description="Description of the rule that was violated")
    reason: str = Field(description="Reason why this violation can be acknowledged")
    risk_level: str = Field(description="Risk level: low, medium, high", default="medium")
    conditions: str | None = Field(description="Conditions for acknowledgment", default=None)


class RequiredFix(BaseModel):
    """Represents a violation that requires fixes."""

    rule_description: str = Field(description="Description of the rule that was violated")
    reason: str = Field(description="Reason why this violation requires fixes")
    priority: str = Field(description="Priority level: low, medium, high, critical", default="medium")


class AcknowledgmentEvaluation(BaseModel):
    """Result of acknowledgment evaluation."""

    is_valid: bool = Field(description="Whether the acknowledgment request is valid")
    reasoning: str = Field(description="Detailed reasoning for the decision")
    acknowledgable_violations: list[AcknowledgedViolation] = Field(
        description="Violations that can be acknowledged", default_factory=list
    )
    require_fixes: list[RequiredFix] = Field(description="Violations that require fixes", default_factory=list)
    confidence: float = Field(description="Confidence in the evaluation", ge=0.0, le=1.0, default=0.5)
    recommendations: list[str] = Field(description="Recommendations for improvement", default_factory=list)


class AcknowledgmentContext(BaseModel):
    """Context for acknowledgment evaluation workflow."""

    acknowledgment_reason: str = Field(description="The acknowledgment reason provided by the user")
    violations: list[Violation] = Field(description="List of violations to evaluate", default_factory=list)
    pr_data: dict[str, Any] = Field(description="Pull request data", default_factory=dict)
    commenter: str = Field(description="Username of the person making the acknowledgment")
    rules: list[dict[str, Any]] = Field(description="List of rules", default_factory=list)
