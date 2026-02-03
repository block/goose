# pylint: disable=duplicate-code, no-name-in-module
"""Utility functions for openlit.guard"""

import re
import json
import os
import logging
from typing import Optional, Tuple
from pydantic import BaseModel
from opentelemetry.metrics import get_meter
from opentelemetry.sdk.resources import TELEMETRY_SDK_NAME
from anthropic import Anthropic
from openai import OpenAI
from openlit.semcov import SemanticConvention

# Initialize logger for logging potential issues and operations
logger = logging.getLogger(__name__)


class JsonOutput(BaseModel):
    """
    A model representing the structure of JSON output for prompt injection detection.

    Attributes:
        score (float): The score of the harmful prompt likelihood.
        verdict (str): Verdict if detection is harmful or not.
        guard (str): The type of guardrail.
        classification (str): The classification of prompt detected.
        explanation (str): A detailed explanation of the detection.
    """

    score: float
    verdict: str
    guard: str
    classification: str
    explanation: str


def setup_provider(
    provider: Optional[str],
    api_key: Optional[str],
    model: Optional[str],
    base_url: Optional[str],
) -> Tuple[Optional[str], Optional[str], Optional[str]]:
    """Function to setup LLM provider"""
    provider_configs = {
        "openai": {"env_var": "OPENAI_API_KEY"},
        "anthropic": {"env_var": "ANTHROPIC_API_KEY"},
    }

    if provider is None:
        return None, None, None

    provider = provider.lower()
    if provider not in provider_configs:
        raise ValueError(f"Unsupported provider: {provider}")

    config = provider_configs[provider]
    env_var = config["env_var"]

    # Handle API key
    if api_key:
        os.environ[env_var] = api_key
    api_key = os.getenv(env_var)

    if not api_key:
        # pylint: disable=line-too-long
        raise ValueError(
            f"API key required via 'api_key' parameter or '{env_var}' environment variable"
        )

    return api_key, model, base_url


def format_prompt(system_prompt: str, text: str) -> str:
    """Function to format the prompt"""
    return system_prompt.replace("{{prompt}}", text)


def llm_response(provider: str, prompt: str, model: str, base_url: str) -> str:
    """Function to get LLM response based on provider"""
    # pylint: disable=no-else-return
    if provider.lower() == "openai":
        return llm_response_openai(prompt, model, base_url)
    elif provider.lower() == "anthropic":
        return llm_response_anthropic(prompt, model)
    else:
        raise ValueError(f"Unsupported provider: {provider}")


def llm_response_openai(prompt: str, model: str, base_url: str) -> str:
    """Function to make LLM call to OpenAI"""
    client = OpenAI(base_url=base_url)

    if model is None:
        model = "gpt-4o"

    if base_url is None:
        base_url = "https://api.openai.com/v1"

    response = client.beta.chat.completions.parse(
        model=model,
        messages=[
            {"role": "user", "content": prompt},
        ],
        temperature=0.0,
        response_format=JsonOutput,
    )
    return response.choices[0].message.content


def llm_response_anthropic(prompt: str, model: str) -> str:
    """Function to make LLM call to Anthropic"""
    client = Anthropic()

    if model is None:
        model = "claude-3-opus-20240229"

    tools = [
        {
            "name": "prompt_analysis",
            "description": "Prints the Prompt Injection score of a given prompt.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "verdict": {
                        "type": "string",
                        "description": "Verdict of guardrail",
                    },
                    "guard": {"type": "string", "description": "Type of guard"},
                    "score": {
                        "type": "number",
                        "description": "Prompt score from Guard.",
                    },
                    "classification": {
                        "type": "string",
                        "description": "Incorrect prompt type",
                    },
                    "explanation": {
                        "type": "string",
                        "description": "Reason for classification",
                    },
                },
                "required": [
                    "verdict",
                    "guard",
                    "score",
                    "classification",
                    "explanation",
                ],
            },
        }
    ]

    response = client.messages.create(
        model=model,
        messages=[{"role": "user", "content": prompt}],
        max_tokens=2000,
        temperature=0.0,
        tools=tools,
        stream=False,
    )

    for content in response.content:
        if content.type == "tool_use" and content.name == "prompt_analysis":
            response = content.input
            break

    return response


def parse_llm_response(response) -> JsonOutput:
    """
    Parses the LLM response into a JsonOutput object.

    Args:
        response: The response from the LLM, expected to be a JSON string or a dictionary.

    Returns:
        JsonOutput: The structured output representing the LLM's assessment.
    """
    try:
        if isinstance(response, str):
            data = json.loads(response)
        elif isinstance(response, dict):
            data = response
        else:
            raise TypeError("Response must be a JSON string or a dictionary.")

        return JsonOutput(**data)
    except (json.JSONDecodeError, TypeError) as e:
        logger.error("Error parsing LLM response: '%s'", e)
        return JsonOutput(
            score=0,
            classification="none",
            explanation="none",
            verdict="none",
            guard="none",
        )


def custom_rule_detection(text: str, custom_rules: list) -> JsonOutput:
    """
    Detects prompt injection using custom defined rules.

    Args:
        text (str): The text to analyze against custom rules.

    Returns:
        JsonOutput: The structured output based on custom rule matches.
    """
    for rule in custom_rules:
        if re.search(rule["pattern"], text):
            return JsonOutput(
                verdict=rule.get("verdict", "yes"),
                guard=rule.get("guard", "prompt_injection"),
                score=rule.get("score", 0.5),
                classification=rule.get("classification", "custom"),
                explanation=rule.get("explanation", "Matched custom rule pattern."),
            )
    return JsonOutput(
        score=0, classification="none", explanation="none", verdict="none", guard="none"
    )


def guard_metrics():
    """
    Initializes OpenTelemetry meter and counter.

    Returns:
        counter: The initialized telemetry counter.
    """
    meter = get_meter(
        __name__,
        "0.1.0",
        schema_url="https://opentelemetry.io/schemas/1.11.0",
    )

    guard_requests = meter.create_counter(
        name=SemanticConvention.GUARD_REQUESTS,
        description="Counter for Guard requests",
        unit="1",
    )

    return guard_requests


def guard_metric_attributes(verdict, score, validator, classification, explanation):
    """
    Initializes OpenTelemetry attributes for metrics.

    Args:
        score (float): The name of the attribute for Guard Score.
        validator (str): The name of the attribute for Guard.
        classification (str): The name of the attribute for Guard classification.
        explaination (str): The name of the attribute for Guard explanation.

    Returns:
        counter: The initialized telemetry counter.
    """
    return {
        TELEMETRY_SDK_NAME: "openlit",
        SemanticConvention.GUARD_VERDICT: verdict,
        SemanticConvention.GUARD_SCORE: score,
        SemanticConvention.GUARD_VALIDATOR: validator,
        SemanticConvention.GUARD_CLASSIFICATION: classification,
        SemanticConvention.GUARD_EXPLANATION: explanation,
    }
