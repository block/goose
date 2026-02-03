from datetime import UTC, datetime
from enum import Enum, StrEnum
from typing import Any, Literal

from pydantic import BaseModel, Field, SecretStr, field_validator


class Severity(StrEnum):
    """
    Severity levels for rule violations.
    """

    INFO = "info"
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    CRITICAL = "critical"


class Violation(BaseModel):
    """
    Represents a single rule violation.
    Standardized model to replace ad-hoc dictionary usage.
    """

    rule_description: str = Field(description="Human-readable description of the rule")
    rule_id: str | None = Field(default=None, description="Stable rule ID for acknowledgment lookup")
    severity: Severity = Field(default=Severity.MEDIUM, description="Severity level of the violation")
    message: str = Field(description="Explanation of why the rule failed")
    details: dict[str, Any] = Field(default_factory=dict, description="Additional context or metadata")
    how_to_fix: str | None = Field(default=None, description="Actionable advice for the user")


class Acknowledgment(BaseModel):
    """
    Represents a user acknowledgment of a violation.
    """

    rule_id: str = Field(description="Unique identifier of the rule being acknowledged")
    reason: str = Field(description="Justification provided by the user")
    commenter: str = Field(description="Username of the person acknowledging")
    timestamp: datetime = Field(default_factory=lambda: datetime.now(UTC), description="Time of acknowledgment")


class User(BaseModel):
    """
    Represents an authenticated user in the system.
    Used for dependency injection in API endpoints.
    """

    id: int
    username: str
    email: str | None = None
    avatar_url: str | None = None
    # storing the token allows the service layer to make requests on behalf of the user
    github_token: SecretStr | None = Field(None, description="OAuth token for GitHub API access", exclude=True)


class RuleParameters(BaseModel):
    """
    Parameters for rule configuration.
    Used to define specific requirements and behaviors for governance rules.
    All fields are optional to support flexible rule definitions.
    """

    message: str | None = Field(None, description="Custom message to display when rule is violated")
    file_patterns: list[str] | None = Field(None, description="File patterns to match (glob format)")
    require_patterns: list[str] | None = Field(None, description="Patterns that must be present")
    forbidden_patterns: list[str] | None = Field(None, description="Patterns that must not be present")
    how_to_fix: str | None = Field(None, description="Instructions on how to fix violations")
    max_size: int | None = Field(None, description="Maximum size threshold (e.g., for PR size limits)")
    min_coverage: float | None = Field(None, description="Minimum coverage threshold (0.0-1.0)")


class RuleConfig(BaseModel):
    """
    Configuration for a rule as exported to YAML.
    This model excludes internal fields like 'key', 'name', and 'reasoning'
    that are used during analysis but not written to the rules.yaml file.
    """

    description: str = Field(..., description="Description of what the rule does")
    enabled: bool = Field(True, description="Whether the rule is enabled")
    severity: Literal["info", "warning", "error", "low", "medium", "high", "critical"] = Field(
        "medium", description="Severity level of the rule"
    )
    event_types: list[str] = Field(..., description="Event types this rule applies to (e.g., ['pull_request'])")
    parameters: dict[str, Any] = Field(default_factory=dict, description="Rule parameters for validators")


class EventType(str, Enum):
    """
    Supported GitHub Event Types.
    Reference: project_detail_med.md [cite: 32]
    """

    PUSH = "push"
    ISSUE_COMMENT = "issue_comment"
    PULL_REQUEST = "pull_request"
    CHECK_RUN = "check_run"
    DEPLOYMENT = "deployment"
    DEPLOYMENT_STATUS = "deployment_status"
    DEPLOYMENT_REVIEW = "deployment_review"
    DEPLOYMENT_PROTECTION_RULE = "deployment_protection_rule"
    WORKFLOW_RUN = "workflow_run"


class WebhookResponse(BaseModel):
    """Standardized response from webhook handlers."""

    status: Literal["ok", "error", "ignored"] = "ok"
    detail: str | None = None
    event_type: EventType | None = None


class WebhookEvent:
    """
    Encapsulates a GitHub webhook event.
    Currently a plain Python class for efficiency, but validates against EventType.
    Reference: project_detail_med.md [cite: 33]
    """

    def __init__(
        self,
        event_type: EventType,
        payload: dict[str, Any],
        delivery_id: str | None = None,
    ):
        self.event_type = event_type
        self.payload = payload
        self.delivery_id = delivery_id  # X-GitHub-Delivery header; used for dedup so each delivery is processed
        self.repository = payload.get("repository", {})
        self.sender = payload.get("sender", {})
        self.installation_id = payload.get("installation", {}).get("id")

    @property
    def repo_full_name(self) -> str:
        """Helper to safely get 'owner/repo' string."""
        return str(self.repository.get("full_name", ""))

    @property
    def sender_login(self) -> str:
        """Helper to safely get the username of the event sender."""
        return str(self.sender.get("login", ""))


class HygieneMetrics(BaseModel):
    """
    Aggregated repository signals for hygiene analysis.
    This is the canonical model used across agents and rules.
    """

    unlinked_issue_rate: float = Field(
        default=0.0,
        description="Percentage (0.0-1.0) of PRs without linked issues. High values indicate poor governance.",
    )
    average_pr_size: int = Field(
        default=0, description="Mean lines changed per PR. Unusually high values suggest untargeted contributions."
    )
    first_time_contributor_count: int = Field(
        default=0, description="Count of unique first-time contributors in recent PRs (risk indicator)."
    )
    ci_skip_rate: float = Field(
        default=0.0,
        description="Percentage (0.0-1.0) of PRs that skip CI checks via commit message.",
    )
    codeowner_bypass_rate: float = Field(
        default=0.0,
        description="Percentage (0.0-1.0) of PRs merged without required CODEOWNER approval.",
    )
    new_code_test_coverage: float = Field(
        default=0.0,
        description="Average ratio of test line additions relative to source code changes.",
    )
    issue_diff_mismatch_rate: float = Field(
        default=0.0,
        description="Percentage (0.0-1.0) of PRs where the linked issue doesn't semantically match the code diff.",
    )
    ghost_contributor_rate: float = Field(
        default=0.0,
        description="Percentage (0.0-1.0) of PRs where the author never responded to review comments.",
    )
    ai_generated_rate: float | None = Field(
        default=0.0,
        description="Percentage (0.0-1.0) of PRs flagged as AI-generated based on heuristic signatures.",
    )

    @field_validator(
        "unlinked_issue_rate",
        "ci_skip_rate",
        "codeowner_bypass_rate",
        "new_code_test_coverage",
        "issue_diff_mismatch_rate",
        "ghost_contributor_rate",
        "ai_generated_rate",
        mode="before",
    )
    @classmethod
    def validate_rate(cls, v: float | None) -> float | None:
        if v is None:
            return None
        if not 0.0 <= v <= 1.0:
            raise ValueError(f"Rate must be between 0.0 and 1.0, got {v}")
        return v
