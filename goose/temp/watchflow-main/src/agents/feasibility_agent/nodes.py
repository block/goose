"""
LangGraph nodes for the Rule Feasibility Agent with enhanced error handling.
"""

import logging

from src.agents.feasibility_agent.models import FeasibilityAnalysis, FeasibilityState, YamlGeneration
from src.agents.feasibility_agent.prompts import RULE_FEASIBILITY_PROMPT, YAML_GENERATION_PROMPT
from src.integrations.providers import get_chat_model
from src.rules.registry import AVAILABLE_CONDITIONS

logger = logging.getLogger(__name__)


async def analyze_rule_feasibility(state: FeasibilityState) -> FeasibilityState:
    """
    Analyze whether a rule description is feasible to implement using structured output.
    Enhanced with retry logic and better error handling.
    """
    try:
        # Create LLM client with structured output
        llm = get_chat_model(agent="feasibility_agent")

        # Use structured output instead of manual JSON parsing
        structured_llm = llm.with_structured_output(FeasibilityAnalysis)

        # Build validator catalog text for the prompt (description + examples when present)
        validator_catalog = []
        for condition_cls in AVAILABLE_CONDITIONS:
            entry = (
                f"- name: {condition_cls.name}\n"
                f"  event_types: {condition_cls.event_types}\n"
                f"  parameter_patterns: {condition_cls.parameter_patterns}\n"
                f"  description: {condition_cls.description}"
            )
            if getattr(condition_cls, "examples", None):
                entry += f"\n  examples: {condition_cls.examples}"
            validator_catalog.append(entry)
        validators_text = "\n".join(validator_catalog)

        # Analyze rule feasibility with awareness of available validators
        prompt = RULE_FEASIBILITY_PROMPT.format(
            rule_description=state.rule_description,
            validator_catalog=validators_text,
        )

        # Get structured response with retry logic
        result = await structured_llm.ainvoke(prompt)

        # Update state with analysis results - now type-safe!
        state.is_feasible = result.is_feasible
        state.rule_type = result.rule_type
        state.chosen_validators = result.chosen_validators
        state.confidence_score = result.confidence_score
        state.feedback = result.feedback
        state.analysis_steps = result.analysis_steps

        logger.info(f"🔍 Rule feasibility analysis completed: {state.is_feasible}")
        logger.info(f"🔍 Rule type identified: {state.rule_type}")
        logger.info(f"🔍 Confidence score: {state.confidence_score}")
        logger.info(f"🔍 Analysis steps: {len(state.analysis_steps)} steps")

    except Exception as e:
        logger.error(f"❌ Error in rule feasibility analysis: {e}")
        state.is_feasible = False
        state.feedback = f"Analysis failed: {str(e)}"
        state.confidence_score = 0.0
        state.analysis_steps = [f"Error occurred: {str(e)}"]

    return state


async def generate_yaml_config(state: FeasibilityState) -> FeasibilityState:
    """
    Generate YAML configuration for feasible rules using structured output.
    This node only runs if the rule is feasible.
    """
    if not state.is_feasible or not state.rule_type:
        logger.info("🔧 Skipping YAML generation - rule not feasible or no rule type")
        return state

    try:
        # Build parameter keys and examples for chosen validators (engine infers validator from these keys)
        validator_parameters_lines = []
        for name in state.chosen_validators:
            for condition_cls in AVAILABLE_CONDITIONS:
                if condition_cls.name == name:
                    keys = getattr(condition_cls, "parameter_patterns", []) or []
                    examples = getattr(condition_cls, "examples", None) or []
                    line = f"- {name}: use only parameter keys {keys}"
                    if examples:
                        line += f"; example(s): {examples[0]}"
                    validator_parameters_lines.append(line)
                    break
        validator_parameters = (
            "\n".join(validator_parameters_lines)
            if validator_parameters_lines
            else "Use only parameter keys from the chosen validators' parameter_patterns."
        )

        # Create LLM client with structured output
        llm = get_chat_model(agent="feasibility_agent")

        # Use structured output for YAML generation
        structured_llm = llm.with_structured_output(YamlGeneration)

        prompt = YAML_GENERATION_PROMPT.format(
            rule_type=state.rule_type,
            rule_description=state.rule_description,
            chosen_validators=", ".join(state.chosen_validators),
            validator_parameters=validator_parameters,
        )

        # Get structured response with retry logic
        result = await structured_llm.ainvoke(prompt)

        # Update state with generated YAML
        state.yaml_content = result.yaml_content.strip()

        # Basic validation of generated YAML
        if not state.yaml_content or len(state.yaml_content) < 10:
            logger.warning("⚠️ Generated YAML seems too short, may be invalid")
            state.feedback += "\nWarning: Generated YAML may be incomplete"

        logger.info(f"🔧 YAML configuration generated for rule type: {state.rule_type}")
        logger.info(f"🔧 Generated YAML length: {len(state.yaml_content)} characters")

    except Exception as e:
        logger.error(f"❌ Error generating YAML configuration: {e}")
        state.feedback += f"\nYAML generation failed: {str(e)}"
        state.yaml_content = ""

    return state
