import yaml

from src.agents.repository_analysis_agent.models import RuleRecommendation
from src.core.models import RuleParameters


def test_rule_recommendation_schema_compliance():
    """
    Verify that RuleRecommendation can be instantiated with all fields
    and that it produces the expected YAML structure.
    """
    rule = RuleRecommendation(
        key="test_rule",
        name="Test Rule",
        description="This is a test rule",
        severity="error",  # Changed from "high" to match Literal["info", "warning", "error"]
        category="quality",
        reasoning="Because it is important",
        event_types=["pull_request"],
        parameters=RuleParameters(
            file_patterns=["src/**/*.py"],
            require_patterns=["def test_"],
            forbidden_patterns=["print("],
            how_to_fix="Remove print statements",
        ),
    )

    assert rule.key == "test_rule"
    assert rule.description == "This is a test rule"
    assert rule.parameters.file_patterns == ["src/**/*.py"]

    # Test YAML generation logic (mimicking src/api/recommendations.py)
    rules_output = {
        "rules": [
            rule.model_dump(
                include={"description", "enabled", "severity", "event_types", "parameters"}, exclude_none=True
            )
        ]
    }

    yaml_str = yaml.dump(rules_output, indent=2, sort_keys=False)

    """
rules:
  - description: This is a test rule
    enabled: true
    severity: error
    event_types:
      - pull_request
    parameters:
      file_patterns:
        - src/**/*.py
""".strip()

    # Simple check if key parts are present
    assert "description: This is a test rule" in yaml_str
    assert "severity: error" in yaml_str
    assert "event_types:" in yaml_str
    assert "- pull_request" in yaml_str
    assert "parameters:" in yaml_str
    assert "file_patterns:" in yaml_str

    # Ensure internal fields are NOT present
    assert "key: test_rule" not in yaml_str
    assert "name: Test Rule" not in yaml_str
    assert "reasoning: Because it is important" not in yaml_str


def test_rule_parameters_optional_fields():
    """Verify that RuleParameters handles optional fields correctly."""
    params = RuleParameters(message="Custom message")
    assert params.message == "Custom message"
    assert params.file_patterns is None

    dumped = params.model_dump(exclude_none=True)
    assert "message" in dumped
    assert "file_patterns" not in dumped
