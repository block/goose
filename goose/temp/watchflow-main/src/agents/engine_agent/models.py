"""
Data models for the Rule Engine Agent.
"""

from __future__ import annotations

from enum import Enum
from typing import Any

from pydantic import BaseModel, ConfigDict, Field

from src.core.models import Violation  # noqa: TCH001, TCH002, TC001
from src.rules.conditions.base import BaseCondition  # noqa: TCH001, TCH002, TC001
from src.rules.models import Rule  # noqa: TCH001, TCH002, TC001


class EngineRequest(BaseModel):
    """Request model for the Rule Engine Agent."""

    event_type: str = Field(description="The type of event (e.g., pull_request)")
    event_data: dict[str, Any] = Field(description="Normalized event payload")
    rules: list[Rule | dict[str, Any]] = Field(description="List of active rules")

    model_config = ConfigDict(arbitrary_types_allowed=True)


class ValidationStrategy(str, Enum):
    """Validation strategies for rule evaluation."""

    VALIDATOR = "validator"  # Use fast validator
    LLM_REASONING = "llm_reasoning"  # Use LLM for complex rules
    HYBRID = "hybrid"  # Try validator first, fallback to LLM


class ValidatorDescription(BaseModel):
    """Description of a validator for dynamic strategy selection."""

    name: str = Field(description="Name of the validator")
    description: str = Field(description="What this validator does")
    parameter_patterns: list[str] = Field(
        description="Parameter patterns this validator can handle", default_factory=list
    )
    event_types: list[str] = Field(description="Event types this validator supports", default_factory=list)
    examples: list[dict[str, Any]] = Field(description="Example rule configurations", default_factory=list)


class StrategySelectionResponse(BaseModel):
    """Structured response for validation strategy selection."""

    strategy: ValidationStrategy = Field(description="Selected validation strategy")
    validator_name: str | None = Field(description="Name of selected validator or null", default=None)
    reasoning: str = Field(description="Explanation of why this strategy was chosen")


class LLMEvaluationResponse(BaseModel):
    """Structured response for LLM rule evaluation."""

    is_violated: bool = Field(description="Whether the rule is violated")
    message: str = Field(description="Explanation of the violation or why the rule passed")
    details: dict[str, Any] = Field(
        description="Detailed reasoning and metadata",
        default_factory=dict,
    )
    how_to_fix: str | None = Field(description="Specific instructions on how to fix the violation", default=None)


class HowToFixResponse(BaseModel):
    """Structured response for generating 'how to fix' messages."""

    how_to_fix: str = Field(description="Specific, actionable instructions on how to fix the violation")
    steps: list[str] = Field(description="Step-by-step instructions", default_factory=list)
    examples: list[str] = Field(description="Example commands or actions", default_factory=list)
    context: str = Field(description="Additional context or explanation", default="")


class RuleViolation(Violation):
    """Represents a violation of a specific rule."""

    # Inherits: rule_description, rule_id, severity, message, details, how_to_fix from Violation

    docs_url: str | None = None
    validation_strategy: ValidationStrategy = ValidationStrategy.VALIDATOR
    execution_time_ms: float = 0.0


class RuleEvaluationResult(BaseModel):
    """The result of evaluating all rules against an event."""

    event_type: str
    repo_full_name: str
    violations: list[RuleViolation] = Field(default_factory=list)
    total_rules_evaluated: int = 0
    rules_triggered: int = 0
    total_rules: int = 0
    evaluation_time_ms: float | None = None
    validator_usage: dict[str, int] = Field(default_factory=dict)  # Track validator usage
    llm_usage: int = 0  # Track LLM usage


class RuleDescription(BaseModel):
    """Enhanced rule description with parameters and validation strategy."""

    description: str = Field(description="Human-readable description of the rule")
    rule_id: str | None = Field(default=None, description="Stable rule ID for acknowledgment lookup")
    parameters: dict[str, Any] = Field(default_factory=dict, description="Rule parameters")
    event_types: list[str] = Field(default_factory=list, description="Supported event types")
    severity: str = Field(default="medium", description="Rule severity level")
    validation_strategy: ValidationStrategy = Field(
        default=ValidationStrategy.HYBRID, description="Validation strategy"
    )
    validator_name: str | None = Field(default=None, description="Specific validator to use")
    fallback_to_llm: bool = Field(default=True, description="Whether to fallback to LLM if validator fails")
    conditions: list["BaseCondition"] = Field(default_factory=list, description="Attached executable conditions")  # noqa: UP037

    model_config = ConfigDict(arbitrary_types_allowed=True)


class EngineState(BaseModel):
    """State for the rule engine workflow."""

    event_type: str
    event_data: dict[str, Any]
    rules: list["Rule"]  # noqa: UP037
    rule_descriptions: list[RuleDescription] = Field(default_factory=list)
    available_validators: list[ValidatorDescription] = Field(default_factory=list)
    violations: list[dict[str, Any]] = Field(default_factory=list)
    evaluation_context: dict[str, Any] = Field(default_factory=dict)
    analysis_steps: list[str] = Field(default_factory=list)
    validator_usage: dict[str, int] = Field(default_factory=dict)
    llm_usage: int = 0

    model_config = ConfigDict(arbitrary_types_allowed=True)


# Update forward references
RuleDescription.model_rebuild()
EngineState.model_rebuild()
