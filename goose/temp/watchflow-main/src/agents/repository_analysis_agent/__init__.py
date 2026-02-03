"""
Repository Analysis Agent for generating Watchflow rule recommendations.

This agent analyzes repository structure, contributing guidelines, and patterns
to automatically propose appropriate Watchflow rules with confidence scores.
"""

from src.agents.repository_analysis_agent.agent import RepositoryAnalysisAgent
from src.agents.repository_analysis_agent.models import (
    AnalysisState,
    HygieneMetrics,
    PRSignal,
    RepoMetadata,
    RepositoryAnalysisRequest,
    RepositoryAnalysisResponse,
    RepositoryFeatures,
    RuleRecommendation,
    parse_github_repo_identifier,
)

__all__ = [
    "RepositoryAnalysisAgent",
    "AnalysisState",
    "HygieneMetrics",
    "PRSignal",
    "RepoMetadata",
    "RepositoryAnalysisRequest",
    "RepositoryAnalysisResponse",
    "RepositoryFeatures",
    "RuleRecommendation",
    "parse_github_repo_identifier",
]
