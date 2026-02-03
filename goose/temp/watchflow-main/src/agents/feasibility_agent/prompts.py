"""
Prompt templates for the Rule Feasibility Agent.
"""

RULE_FEASIBILITY_PROMPT = """
You are an expert in Watchflow rules and GitHub automation. Analyze whether a natural language rule description is feasible to implement using Watchflow’s existing validator catalog. Do NOT invent custom logic; choose from the provided validators. If none fit, mark as not feasible.

Rule Description:
{rule_description}

Available validators (name, event_types, parameter_patterns, description):
{validator_catalog}

Decide:
1) is_feasible (true/false)
2) rule_type (short label you infer)
3) chosen_validators (list of validator names from the catalog that can implement this rule; empty if not feasible)
4) feedback (practical, under 120 words)
5) analysis_steps (succinct bullets)
"""

YAML_GENERATION_PROMPT = """
Generate a complete Watchflow rules.yaml for the rule below using ONLY the selected validators.

Rule Type: {rule_type}
Description: {rule_description}
Chosen Validators: {chosen_validators}

Parameter keys to use (use only these keys under parameters; the engine infers which validator runs from them):
{validator_parameters}

Rules YAML format:
```yaml
rules:
  - description: "<concise description>"
    enabled: true
    severity: "medium"
    event_types: ["pull_request"]
    parameters:
      <only the parameter keys listed above, with appropriate values>
```

Guidelines:
- Under parameters use ONLY the parameter keys listed above. Do not add a "validator" key; end users do not specify validators—the engine selects them from the parameter names.
- Keep severity appropriate (low/medium/high/critical).
- event_types must align with the validators chosen.
- For regex, use single quotes.
- If no validators fit, return an empty yaml_content.
Return only the YAML content.
"""
