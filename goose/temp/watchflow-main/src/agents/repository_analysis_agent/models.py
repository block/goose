# File: src/agents/repository_analysis_agent/models.py

from typing import Any

from giturlparse import parse  # type: ignore
from pydantic import BaseModel, Field, field_serializer, model_validator

from src.core.models import HygieneMetrics


def parse_github_repo_identifier(identifier: str) -> str:
    """
    Normalizes various GitHub identifiers into 'owner/repo' format using giturlparse.
    Used by tests to verify repository strings.
    """
    try:
        p = parse(identifier)
        if p.valid and p.owner and p.repo:
            return f"{p.owner}/{p.repo}"
        # Fallback for simple "owner/repo" strings that might fail strict URL parsing
        if "/" in identifier and not identifier.startswith(("http", "git@")):
            return identifier.strip().strip("/")
        return identifier
    except Exception:
        # If parsing fails completely, return original to let validator catch it later
        return identifier


class RepositoryAnalysisRequest(BaseModel):
    """
    Input request for analyzing a repository.
    Automatically normalizes the repository URL into a full name.
    """

    repository_full_name: str = Field(default="", description="GitHub repository in 'owner/repo' format")
    repository_url: str = Field(default="", description="Full GitHub repository URL (optional, will be normalized)")
    installation_id: int | None = Field(
        default=None, description="GitHub App installation ID for authenticated requests"
    )

    @model_validator(mode="after")
    def normalize_repo_name(self) -> "RepositoryAnalysisRequest":
        """Normalize repository URL to full name format."""
        if not self.repository_full_name and self.repository_url:
            self.repository_full_name = parse_github_repo_identifier(self.repository_url)
        return self


class RepositoryFeatures(BaseModel):
    """
    Extracted features from a repository used for analysis.
    """

    has_contributing: bool = Field(default=False, description="Has CONTRIBUTING.md file")
    has_codeowners: bool = Field(default=False, description="Has CODEOWNERS file")
    has_workflows: bool = Field(default=False, description="Has GitHub Actions workflows")
    contributor_count: int = Field(default=0, description="Number of contributors")
    detected_languages: list[str] = Field(default_factory=list, description="Programming languages detected")
    has_tests: bool = Field(default=False, description="Has test files or testing framework")


class RepoMetadata(BaseModel):
    """
    Structured repository metadata.
    """

    name: str
    owner: str
    default_branch: str
    language: str | None = None
    description: str | None = None
    stargazers_count: int = 0
    forks_count: int = 0
    open_issues_count: int = 0


class RepositoryAnalysisResponse(BaseModel):
    """
    Response from repository analysis containing recommendations and metadata.
    """

    success: bool = Field(..., description="Whether the analysis completed successfully")
    message: str = Field(..., description="Status message or error description")
    repository_full_name: str = Field(..., description="The analyzed repository")
    recommendations: list["RuleRecommendation"] = Field(default_factory=list, description="List of recommended rules")
    features: RepositoryFeatures | None = Field(default=None, description="Extracted repository features")
    metadata: RepoMetadata | None = Field(default=None, description="Additional analysis metadata")


class RuleRecommendation(BaseModel):
    """
    Represents a single rule suggested by the AI.
    Contains both internal fields (key, name, reasoning) and YAML-exportable fields.

    Internal fields:
    - key: Unique identifier for the rule (not exported to YAML)
    - name: Human-friendly name (not exported to YAML)
    - category: Rule category like 'quality', 'security', etc. (not exported to YAML)
    - reasoning: Justification for why this rule was recommended (not exported to YAML)

    YAML-exportable fields:
    - description: What the rule does
    - enabled: Whether the rule is enabled
    - severity: Severity level
    - event_types: Event types this rule applies to
    - parameters: Rule parameters (can be RuleParameters object or dict)
    """

    # Internal fields (not exported to YAML)
    key: str | None = Field(None, description="Unique identifier for the rule")
    name: str | None = Field(None, description="Human-friendly name")
    category: str | None = Field(None, description="Rule category (e.g., 'quality', 'security', 'hygiene')")
    reasoning: str | None = Field(None, description="Justification for why this rule was recommended")

    # YAML-exportable fields
    description: str = Field(..., description="What the rule does")
    enabled: bool = Field(True, description="Whether the rule is enabled")
    severity: str = Field("medium", description="low, medium, high, or critical")
    event_types: list[str] = Field(..., description="Event types this rule applies to (e.g., ['pull_request'])")
    parameters: Any = Field(default_factory=dict, description="Rule parameters for validators")

    model_config = {"arbitrary_types_allowed": True}

    @field_serializer("parameters", when_used="json-unless-none")
    def serialize_parameters(self, params: Any) -> dict[str, Any]:
        """Serialize parameters to dict, handling RuleParameters objects."""
        from src.core.models import RuleParameters

        if isinstance(params, RuleParameters):
            return params.model_dump(exclude_none=True)
        elif isinstance(params, dict):
            return params
        else:
            # For any other type, try to convert to dict
            return dict(params) if params else {}


class PRSignal(BaseModel):
    """
    Represents data from a single historical PR to detect AI spam or low-quality contributions.

    This model is a core component of the "AI Immune System" feature. It captures signals
    from merged PRs that indicate potential low-effort contributions, including:
    - Missing issue links (drive-by commits)
    - First-time contributors (higher risk profile)
    - AI-generated content markers (detected via heuristics)
    - Abnormal PR sizes (mass changes without context)

    These signals feed into HygieneMetrics to inform rule recommendations.
    """

    pr_number: int = Field(..., description="GitHub PR number for reference")
    has_linked_issue: bool = Field(..., description="Whether the PR references an issue (required for context)")
    author_association: str = Field(
        ..., description="GitHub author role: 'FIRST_TIME_CONTRIBUTOR', 'MEMBER', 'COLLABORATOR', etc."
    )
    is_ai_generated_hint: bool = Field(
        ..., description="Heuristic flag: True if PR description contains AI tool signatures (Claude, Cursor, etc.)"
    )
    lines_changed: int = Field(..., description="Total lines added + deleted (indicator of PR scope)")


class AnalysisState(BaseModel):
    """
    The Shared Memory (Blackboard) for the Analysis Agent.
    """

    # --- Inputs ---
    repo_full_name: str
    is_public: bool = False
    user_token: str | None = Field(
        None, description="Optional GitHub Personal Access Token for authenticated API requests"
    )

    # --- Collected Signals (Raw Data) ---
    file_tree: list[str] = Field(default_factory=list, description="List of file paths in the repo")
    readme_content: str | None = None
    contributing_content: str | None = None
    codeowners_content: str | None = Field(None, description="CODEOWNERS file content if available")
    detected_languages: list[str] = Field(default_factory=list)
    has_ci: bool = False
    has_codeowners: bool = False
    workflow_patterns: list[str] = Field(
        default_factory=list, description="Detected workflow patterns in .github/workflows/"
    )
    pr_signals: list[PRSignal] = Field(default_factory=list, description="Historical PR signals for hygiene analysis")
    hygiene_summary: HygieneMetrics | None = None

    # --- Outputs ---
    recommendations: list[RuleRecommendation] = Field(default_factory=list)
    rule_reasonings: dict[str, str] = Field(
        default_factory=dict, description="Map of rule description to reasoning/justification"
    )
    analysis_report: str | None = Field(default=None, description="Generated analysis report markdown")

    # --- Execution Metadata ---
    error: str | None = None
    warnings: list[str] = Field(default_factory=list, description="Warnings about incomplete data or rate limits")
    step_log: list[str] = Field(default_factory=list)
