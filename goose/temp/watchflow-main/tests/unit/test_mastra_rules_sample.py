"""Regression test for the Mastra sample rules."""

from pathlib import Path

import yaml

from src.rules.models import Rule

SAMPLE_RULES_PATH = Path(__file__).resolve().parents[2] / "docs" / "assets" / "mastra-watchflow-rules.yaml"


def test_mastra_sample_rules_validate_without_actions():
    """Ensure the Mastra sample rules stay compatible with the current rule schema."""
    assert SAMPLE_RULES_PATH.exists(), "Sample rules file is missing"

    data = yaml.safe_load(SAMPLE_RULES_PATH.read_text())
    assert isinstance(data, dict) and "rules" in data, "Sample file must include a top-level 'rules' list"

    for rule in data["rules"]:
        validated_rule = Rule.model_validate(rule)
        # Loader stores actions but invocation pipeline currently ignores them.
        # Keep the sample intentionally simple until action semantics are implemented.
        assert not validated_rule.actions, "Sample rules must omit 'actions' entries"
