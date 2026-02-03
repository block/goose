"""
Data models for the Rule Feasibility Agent.
"""

from pydantic import BaseModel, Field


class FeasibilityAnalysis(BaseModel):
    """Structured output model for rule feasibility analysis."""

    is_feasible: bool = Field(description="Whether the rule is feasible to implement with Watchflow")
    rule_type: str = Field(description="Type of rule (time_restriction, branch_pattern, title_pattern, etc.)")
    chosen_validators: list[str] = Field(
        description="Names of validators from the catalog that can implement this rule",
        default_factory=list,
    )
    confidence_score: float = Field(description="Confidence score from 0.0 to 1.0", ge=0.0, le=1.0)
    feedback: str = Field(description="Detailed feedback on implementation considerations")
    analysis_steps: list[str] = Field(description="Step-by-step analysis breakdown", default_factory=list)


class YamlGeneration(BaseModel):
    """Structured output model for YAML configuration generation."""

    yaml_content: str = Field(description="Generated Watchflow YAML rule configuration")


class FeasibilityState(BaseModel):
    """State for the feasibility evaluation workflow."""

    rule_description: str
    is_feasible: bool = False
    yaml_content: str = ""
    feedback: str = ""
    confidence_score: float = 0.0
    rule_type: str = ""
    chosen_validators: list[str] = Field(default_factory=list)
    analysis_steps: list[str] = Field(default_factory=list)
