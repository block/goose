from src.agents.repository_analysis_agent.models import (
    PRSignal,
    RepositoryAnalysisRequest,
    parse_github_repo_identifier,
)
from src.core.models import HygieneMetrics


def test_parse_github_repo_identifier_normalizes_url():
    assert parse_github_repo_identifier("https://github.com/owner/repo.git") == "owner/repo"
    assert parse_github_repo_identifier("owner/repo/") == "owner/repo"


def test_repository_analysis_request_normalizes_from_url():
    request = RepositoryAnalysisRequest(repository_url="https://github.com/owner/repo.git")
    assert request.repository_full_name == "owner/repo"


def test_hygiene_metrics_calculation():
    """Test that HygieneMetrics model correctly stores aggregated PR signals."""
    metrics = HygieneMetrics(
        unlinked_issue_rate=0.6,  # 60% of PRs have no issues
        average_pr_size=350,
        first_time_contributor_count=5,
    )
    assert metrics.unlinked_issue_rate == 0.6
    assert metrics.average_pr_size == 350
    assert metrics.first_time_contributor_count == 5


def test_pr_signal_model():
    """Test PRSignal model creation for AI spam detection."""
    signal = PRSignal(
        pr_number=123,
        has_linked_issue=False,
        author_association="FIRST_TIME_CONTRIBUTOR",
        is_ai_generated_hint=True,
        lines_changed=500,
    )
    assert signal.pr_number == 123
    assert signal.has_linked_issue is False
    assert signal.is_ai_generated_hint is True
    assert signal.lines_changed == 500
