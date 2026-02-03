"""
Prompts for the Rule Engine Agent with hybrid validation strategy.
"""

from src.agents.engine_agent.models import RuleDescription, ValidatorDescription


def create_rule_analysis_prompt(rule_descriptions: list[RuleDescription], event_type: str) -> str:
    """Create a prompt for analyzing rule descriptions and parameters."""

    rules_text = ""
    for i, rule in enumerate(rule_descriptions, 1):
        rules_text += f"""
Rule {i}: {rule.description}
- Parameters: {rule.parameters}
- Event Types: {rule.event_types}
- Severity: {rule.severity}
"""

    return f"""
You are analyzing rule descriptions and parameters for a GitHub event.

Event Type: {event_type}

Rules to analyze:
{rules_text}

Please analyze each rule and determine:
1. Whether the rule is applicable to this event type
2. What validation strategy would be most appropriate (validator vs LLM reasoning)
3. Any potential issues or ambiguities in the rule description or parameters

Focus on the rule descriptions and parameters to understand the intent and requirements.
"""


def create_validation_strategy_prompt(
    rule_desc: RuleDescription, available_validators: list[ValidatorDescription]
) -> str:
    """Create a prompt for selecting validation strategy based on rule description and parameters."""

    # Format available validators
    validators_text = ""
    for validator in available_validators:
        validators_text += f"""
- {validator.name}: {validator.description}
  Parameter patterns: {validator.parameter_patterns}
  Event types: {validator.event_types}
  Examples: {validator.examples}
"""

    return f"""
You are selecting the best validation strategy for a rule based on its description and parameters.

Rule Information:
- Description: {rule_desc.description}
- Parameters: {rule_desc.parameters}
- Event Types: {rule_desc.event_types}
- Severity: {rule_desc.severity}

Available Validators:
{validators_text}

Validation Strategies:
1. VALIDATOR: Use a fast, deterministic validator for common rule patterns
2. LLM_REASONING: Use LLM for complex, custom, or ambiguous rules
3. HYBRID: Try validator first, fallback to LLM if needed

Consider:
- Rule complexity and specificity
- Whether parameters match available validator patterns
- Whether the rule requires contextual understanding
- Performance implications (validators are faster and cheaper)

Select the best strategy based on the rule description and parameters.
"""


def create_llm_evaluation_prompt(rule_desc: RuleDescription, event_data: dict, event_type: str) -> str:
    """Create a prompt for LLM evaluation of a rule based on its description and parameters."""

    # Extract relevant event data for context
    event_context = _extract_event_context(event_data, event_type)

    return f"""
You are evaluating whether a GitHub event violates a rule based on the rule's description and parameters.

Rule Information:
- Description: {rule_desc.description}
- Parameters: {rule_desc.parameters}
- Event Types: {rule_desc.event_types}
- Severity: {rule_desc.severity}

Event Information:
- Type: {event_type}
- Context: {event_context}

Evaluation Task:
Analyze whether this event violates the rule based on the rule's description and parameters.

Consider:
1. The rule's intent as described in its description
2. The specific parameters and their requirements
3. The event context and whether it meets the rule's criteria
4. Any edge cases or ambiguities in the rule description

Focus on the rule description and parameters to make your determination.
"""


def create_how_to_fix_prompt(rule_desc: RuleDescription, event_data: dict, validator_name: str) -> str:
    """Create a prompt for generating dynamic 'how to fix' messages."""

    # Extract relevant event data for context
    event_context = _extract_event_context(event_data, "pull_request")  # Most common case

    return f"""
You are generating specific, actionable instructions for fixing a GitHub rule violation.

Rule Information:
- Description: {rule_desc.description}
- Parameters: {rule_desc.parameters}
- Severity: {rule_desc.severity}
- Validator Used: {validator_name}

Event Context:
{event_context}

Task:
Generate specific, actionable instructions for fixing this violation. Consider:

1. **Specificity**: Provide exact steps or commands when possible
2. **Context**: Use the actual event data to provide relevant guidance
3. **Actionability**: Give concrete steps that can be followed immediately
4. **Clarity**: Make instructions clear and easy to understand
5. **GitHub-specific**: Use GitHub terminology and workflows

Examples of good instructions:
- "Add the 'security' and 'review' labels to this pull request"
- "Update the PR title to match the pattern: '^feat|^fix|^docs'"
- "Add more details to the PR description (minimum 50 characters)"
- "Change the target branch from 'feature' to 'main' or 'develop'"
- "Wait until a weekday to merge this pull request"

Focus on providing the most helpful and specific guidance based on the rule description, parameters, and current event context.
"""


def get_llm_evaluation_system_prompt() -> str:
    """Get the system prompt for LLM rule evaluation."""

    return """
You are an expert at evaluating GitHub events against governance rules.

Your task is to determine whether a GitHub event violates a rule based on the rule's description and parameters.

Key Principles:
1. Focus on the rule's description and parameters to understand the intent
2. Consider the specific context of the GitHub event
3. Be precise and objective in your evaluation
4. Provide clear reasoning for your decision
5. Give actionable feedback when violations are found

Evaluation Guidelines:
- If the rule description is clear and the event clearly violates it: mark as violated
- If the rule parameters are met and the event complies: mark as passed
- If there's ambiguity in the rule description: use reasonable interpretation
- If the event type doesn't match the rule's event types: mark as passed (not applicable)

Always respond with valid structured output as specified in the prompt.
"""


def _extract_event_context(event_data: dict, event_type: str) -> str:
    """Extract relevant context from event data for evaluation."""

    context_parts = []

    if event_type == "pull_request":
        pr = event_data.get("pull_request_details") or event_data.get("pull_request") or {}
        context_parts.extend(
            [
                f"Title: {pr.get('title', 'N/A')}",
                f"Description: {pr.get('body', 'N/A')[:200]}...",
                f"Labels: {[label.get('name') for label in pr.get('labels', [])]}",
                f"Review Status: {pr.get('state', 'N/A')}",
                f"Review Count: {pr.get('requested_reviewers', [])}",
            ]
        )

        files = event_data.get("files", [])
        if files:
            top_files = [file.get("filename") for file in files[:5] if file.get("filename")]
            context_parts.append(
                f"Changed Files ({len(files)} total): {top_files if top_files else '[filenames unavailable]'}"
            )

        diff_summary = event_data.get("diff_summary")
        if diff_summary:
            context_parts.append(f"Diff Summary:\n{diff_summary}")

    elif event_type == "push":
        context_parts.extend(
            [
                f"Branch: {event_data.get('ref', 'N/A')}",
                f"Commits: {len(event_data.get('commits', []))}",
            ]
        )

    elif event_type == "deployment":
        deployment = event_data.get("deployment", {})
        context_parts.extend(
            [
                f"Environment: {deployment.get('environment', 'N/A')}",
                f"Ref: {deployment.get('ref', 'N/A')}",
            ]
        )

    # Add common fields
    repo = event_data.get("repository", {})
    if repo:
        context_parts.append(f"Repository: {repo.get('full_name', 'N/A')}")

    return "; ".join(context_parts) if context_parts else "Limited context available"
