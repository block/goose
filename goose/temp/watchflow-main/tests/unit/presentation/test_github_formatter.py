from src.core.models import Acknowledgment, Severity, Violation
from src.presentation.github_formatter import (
    format_acknowledgment_summary,
    format_check_run_output,
    format_violations_comment,
    format_violations_for_check_run,
)


def test_format_violations_comment_groups_by_severity():
    violations = [
        Violation(rule_description="Rule 1", severity=Severity.HIGH, message="Error 1", how_to_fix="Fix 1"),
        Violation(rule_description="Rule 2", severity=Severity.CRITICAL, message="Error 2", how_to_fix="Fix 2"),
        Violation(rule_description="Rule 3", severity=Severity.HIGH, message="Error 3"),
    ]

    comment = format_violations_comment(violations)

    assert "## 🚨 Watchflow Rule Violations Detected" in comment
    assert "### 🔴 Critical Severity" in comment
    assert "### 🟠 High Severity" in comment
    assert "**Rule 2**" in comment
    assert "**Rule 1**" in comment
    assert "**Rule 3**" in comment
    assert "Fix 1" in comment
    assert "Fix 2" in comment


def test_format_violations_comment_empty():
    comment = format_violations_comment([])
    assert "## 🚨 Watchflow Rule Violations Detected" in comment
    assert "---" in comment


def test_format_check_run_output_success():
    output = format_check_run_output([])
    assert output["title"] == "All rules passed"
    assert "✅ No rule violations detected" in output["summary"]
    assert "passed successfully" in output["text"]


def test_format_check_run_output_with_violations():
    violations = [Violation(rule_description="Missing Issue", severity=Severity.HIGH, message="No issue linked")]

    output = format_check_run_output(violations)

    assert "1 rule violations found" in output["title"]
    assert "🚨 1 violations found: 1 high" in output["summary"]
    assert "## 🟠 High Severity" in output["text"]
    assert "### Missing Issue" in output["text"]


def test_format_check_run_output_rules_not_configured():
    error = "Rules not configured"
    repo = "owner/repo"
    inst_id = 123

    output = format_check_run_output([], error=error, repo_full_name=repo, installation_id=inst_id)

    assert output["title"] == "Rules not configured"
    assert "Analyze your repository" in output["text"]
    assert f"repo={repo}" in output["text"]
    assert f"installation_id={inst_id}" in output["text"]


def test_format_acknowledgment_summary():
    violations = [Violation(rule_description="PR Title", severity=Severity.MEDIUM, message="Bad title")]
    acks = {"pr-title": Acknowledgment(rule_id="pr-title", reason="One-off", commenter="tom")}

    summary = format_acknowledgment_summary(violations, acks)
    assert "**PR Title**" in summary
    assert "Bad title" in summary


def test_format_violations_for_check_run():
    violations = [Violation(rule_description="Lint", severity=Severity.LOW, message="Trailing space")]

    result = format_violations_for_check_run(violations)
    assert "• **Lint** - Trailing space" in result


def test_format_violations_for_check_run_empty():
    assert format_violations_for_check_run([]) == "None"
