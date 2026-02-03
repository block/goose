"""
Prompt templates for the Intelligent Acknowledgment Agent.
"""

from typing import Any


def get_system_prompt() -> str:
    """Get the system prompt for acknowledgment evaluation."""
    return """You are an intelligent acknowledgment evaluation agent for Watchflow, a GitHub security and compliance tool.

Your role is to evaluate acknowledgment requests for rule violations and determine:
1. Whether the acknowledgment request is valid and justified
2. Which violations can be safely acknowledged
3. Which violations require fixes regardless of the reason
4. Provide detailed reasoning for your decisions

You must be strict but fair. Only approve acknowledgments that have:
- Specific, legitimate business reasons
- Clear justification for the override
- Acceptable risk levels
- Proper context

Always prioritize security and compliance over convenience.

Respond with structured output as specified in the AcknowledgmentEvaluation model."""


def create_evaluation_prompt(
    acknowledgment_reason: str,
    violations: list[dict[str, Any]],
    pr_data: dict[str, Any],
    commenter: str,
    rules: list[dict[str, Any]],
) -> str:
    """Create a comprehensive evaluation prompt for acknowledgment analysis."""

    # Format violations for the prompt
    violations_text = ""
    for i, violation in enumerate(violations, 1):
        violations_text += f"""
Violation {i}:
- Rule: {violation.get("rule_description", "Unknown rule")}
- Severity: {violation.get("severity", "medium")}
- Message: {violation.get("message", "No message")}
- How to fix: {violation.get("how_to_fix", "No fix provided")}
"""

    # Format PR data
    pr_title = pr_data.get("title", "Unknown")
    pr_author = pr_data.get("user", {}).get("login", "Unknown")
    pr_labels = [label.get("name", "") for label in pr_data.get("labels", [])]

    # Format rules for context
    rules_text = ""
    for rule in rules:
        rules_text += f"- {rule.get('description', 'Unknown rule')} (severity: {rule.get('severity', 'medium')})\n"

    return f"""
Evaluate this acknowledgment request for rule violations.

**Acknowledgment Request:**
- Commenter: {commenter}
- Reason: {acknowledgment_reason}

**Pull Request Context:**
- Title: {pr_title}
- Author: {pr_author}
- Labels: {", ".join(pr_labels) if pr_labels else "None"}

**Current Violations:**
{violations_text}

**Available Rules:**
{rules_text}

**Evaluation Instructions:**
1. Analyze the acknowledgment reason for specificity, legitimacy, and business justification
2. Evaluate each violation against the acknowledgment reason
3. Consider the severity and nature of each violation
4. Determine which violations can be safely acknowledged vs. require fixes
5. Provide detailed reasoning for your decisions

**Important Guidelines:**
- Use the EXACT rule_description from the violations when referencing rules
- Be strict about security and compliance violations
- Only acknowledge violations with legitimate business justification
- Provide specific, actionable reasoning
- Consider the overall risk to the organization

Respond with structured output using the AcknowledgmentEvaluation model with the following fields:
- is_valid: boolean indicating if the acknowledgment request is valid
- reasoning: detailed explanation of your decision
- acknowledgable_violations: list of violations that can be acknowledged (use exact rule_description)
- require_fixes: list of violations that require fixes (use exact rule_description)
- confidence: float between 0.0 and 1.0 indicating confidence in the evaluation
- recommendations: list of recommendations for improvement
- details: additional context as a dictionary
"""
