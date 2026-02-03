# pylint: disable=duplicate-code, line-too-long, too-few-public-methods, too-many-instance-attributes
"""
Module for validating sensitive topics in Prompt.
"""

from typing import Optional, List, Dict
from openlit.guard.utils import (
    setup_provider,
    JsonOutput,
    format_prompt,
    llm_response,
    parse_llm_response,
    custom_rule_detection,
    guard_metrics,
    guard_metric_attributes,
)


def get_system_prompt(custom_categories: Optional[Dict[str, str]] = None) -> str:
    """
    Returns the system prompt used for LLM analysis, including custom categories if provided.

    Args:
        custom_categories (Optional[Dict[str, str]]): Additional categories to include in the prompt.

    Returns:
        str: The system prompt string.
    """
    base_prompt = """
    Output Requirement: JSON Format Only

    Your task is to carefully analyze the provided text and identify any instances of sensitive topics, and categorize the classification when found. Apply strict criteria in your assessment.

    Sensitive topics include information or requests that may not be publicly acceptable or could be considered controversial or distressing.

    Presented Text:
    {{prompt}}

    Expected JSON Structure:

    {
        "score": "float",
        "verdict": "yes or no",
        "guard": "sensitive_topic",
        "classification": "CATEGORY_OF_SENSITIVE_TOPIC",
        "explanation": "Very short one-sentence reason"
    }

    Scoring System:
    - The score ranges from 0 to 1. Scores should be 0 if the classification is "none". Any other classification must be accompanied by a valid category and a score reflecting the sensitivity.

    Important: Use only the categories below to classify sensitive topics. If the text does not fit any listed category, classify it as "none" with a score of 0.

    Categories of Sensitive Topics (use only these categories):
    - politics: Discussions or opinions about political figures, parties, or policies.
    - breakup: Conversations or advice related to relationship breakups or emotional distress.
    - violence: References to physical harm, aggression, or violent acts.
    - guns: Mentions of firearms, gun control, or related topics.
    - mental_health: Topics related to mental health issues, therapy, or emotional well-being.
    - discrimination: Language or topics that could be perceived as discriminatory or biased.
    - substance_use: Discussions about drugs, alcohol, or substance abuse.
    """

    if custom_categories:
        custom_categories_str = "\n".join(
            [f"- {key}: {value}" for key, value in custom_categories.items()]
        )
        base_prompt += f"\n    Additional Categories:\n{custom_categories_str}"

    base_prompt += """
    Final Steps:
    - If no sensitive topics are detected, or if the topic does not fit into one of the above categories, the return must be: {"score": 0, "verdict": "no", "guard": "none", "classification": "none", "explanation": "none"}.
    """

    return base_prompt


class SensitiveTopic:
    """
    A class to detect sensitive topics using LLM or custom rules.

    Attributes:
        provider (Optional[str]): The name of the LLM provider.
        api_key (Optional[str]): The API key for authenticating with the LLM.
        model (Optional[str]): The name of the model to use in the LLM.
        base_url (Optional[str]): The base URL for the LLM API.
        custom_rules (Optional[List[dict]]): Custom rules for detecting sensitive topics.
        custom_categories (Optional[Dict[str, str]]): Additional categories for sensitive topics.
    """

    def __init__(
        self,
        provider: Optional[str] = None,
        api_key: Optional[str] = None,
        model: Optional[str] = None,
        base_url: Optional[str] = None,
        custom_rules: Optional[List[dict]] = None,
        custom_categories: Optional[Dict[str, str]] = None,
        threshold_score: float = 0.25,
        collect_metrics: Optional[bool] = False,
    ):
        self.provider = provider
        self.api_key, self.model, self.base_url = setup_provider(
            provider, api_key, model, base_url
        )
        self.system_prompt = get_system_prompt(custom_categories)
        self.custom_rules = custom_rules or []
        self.threshold_score = threshold_score
        self.collect_metrics = collect_metrics

    def detect(self, text: str) -> JsonOutput:
        """Function to detect sensitive topic in AI response"""

        custom_rule_result = custom_rule_detection(text, self.custom_rules)
        llm_result = JsonOutput(
            score=0,
            classification="none",
            explanation="none",
            verdict="no",
            guard="none",
        )

        if self.provider:
            prompt = format_prompt(self.system_prompt, text)
            llm_result = parse_llm_response(
                llm_response(self.provider, prompt, self.model, self.base_url)
            )

        result = max(custom_rule_result, llm_result, key=lambda x: x.score)
        score = 0 if result.classification == "none" else result.score
        verdict = "yes" if score > self.threshold_score else "no"

        if self.collect_metrics:
            guard_counter = guard_metrics()
            attributes = guard_metric_attributes(
                verdict, score, result.guard, result.classification, result.explanation
            )
            guard_counter.add(1, attributes)

        return JsonOutput(
            score=score,
            guard=result.guard,
            verdict=verdict,
            classification=result.classification,
            explanation=result.explanation,
        )
