# pylint: disable=duplicate-code, line-too-long, too-few-public-methods, too-many-instance-attributes
"""
Module for validating valid topics in Prompt.
"""

from typing import Optional, List
from openlit.guard.utils import (
    setup_provider,
    JsonOutput,
    format_prompt,
    llm_response,
    parse_llm_response,
    guard_metrics,
    guard_metric_attributes,
)


def get_system_prompt(
    valid_topics: Optional[List[str]] = None, invalid_topics: Optional[List[str]] = None
) -> str:
    """
    Returns the system prompt used for LLM analysis, including valid and invalid topics if provided.

    Args:
        valid_topics (Optional[List[str]]): List of valid topics.
        invalid_topics (Optional[List[str]]): List of invalid topics.

    Returns:
        str: The system prompt string.
    """
    base_prompt = """
    Output Requirement: JSON Format Only

    Your task is to analyze the provided text and determine if it falls within the specified valid or invalid topics. Apply strict criteria in your assessment.

    The text should be categorized based on the predefined topics. If the text fits into invalid categories it should be marked as "invalid topic", otherwise, if it fits into valid categories, mark "valid topic".

    Presented Text:
    {{prompt}}

    Expected JSON Structure:

    {
        "score": "float",
        "verdict": "yes or no",
        "guard": "topic_restriction",
        "classification": "valid_topic or invalid_topic",
        "explanation": "Very short one-sentence reason"
    }

    Scoring System:
    - The score ranges from 0 to 1. A score above 0.75 indicates a strong detection of an invalid topic.

    Categories of Valid Topics (use only these categories):
    """

    # Add valid topics to the prompt if provided
    if valid_topics:
        valid_topics_str = "\n".join([f"- {topic}" for topic in valid_topics])
        base_prompt += valid_topics_str
    else:
        base_prompt += "- No valid categories. All topics are considered invalid unless specified under valid topics."

    base_prompt += """

    Categories of Invalid Topics (additional checks):
    """
    # Add invalid topics to the prompt if provided
    if invalid_topics:
        invalid_topics_str = "\n".join([f"- {topic}" for topic in invalid_topics])
        base_prompt += invalid_topics_str
    else:
        base_prompt += "- No predefined invalid categories."

    base_prompt += """

    Final Steps:
    - If the text matches one of the valid topics, return: {"score": 0, "verdict": "no", "guard": "topic_restriction", "classification": "valid_topic", "explanation": "Text fits into a valid topic."}.
    - If the text matches any invalid topics, return: {"score": 1.0, "verdict": "yes", "guard": "topic_restriction", "classification": "invalid_topic", "explanation": "Text does not match any valid categories."}.
    - If the text does not match any of the above categories, it's considered invalid unless another rule applies.
    """

    return base_prompt


class TopicRestriction:
    """
    A class to validate if text belongs to valid or invalid topics using LLM.

    Attributes:
        provider (Optional[str]): The name of the LLM provider.
        api_key (Optional[str]): The API key for authenticating with the LLM.
        model (Optional[str]): The name of the model to use in the LLM.
        base_url (Optional[str]): The base URL for the LLM API.
        valid_topics (Optional[List[str]]): List of valid topics.
        invalid_topics (Optional[List[str]]): List of invalid topics.
    """

    def __init__(
        self,
        provider: Optional[str],
        valid_topics: Optional[List[str]] = None,
        api_key: Optional[str] = None,
        model: Optional[str] = None,
        base_url: Optional[str] = None,
        invalid_topics: Optional[List[str]] = None,
        collect_metrics: Optional[bool] = False,
    ):
        """
        Initializes the TopicRestriction with specified LLM settings and topics.

        Args:
            provider (Optional[str]): The name of the LLM provider.
            api_key (Optional[str]): The API key for authenticating with the LLM.
            model (Optional[str]): The name of the model to use in the LLM.
            base_url (Optional[str]): The base URL for the LLM API.
            valid_topics (Optional[List[str]]): List of valid topics.
            invalid_topics (Optional[List[str]]): List of invalid topics.

        Raises:
            ValueError: If the provider is not specified.
        """
        self.provider = provider
        if self.provider is None:
            raise ValueError(
                "An LLM provider must be specified for TopicRestriction Validator"
            )
        self.api_key, self.model, self.base_url = setup_provider(
            provider, api_key, model, base_url
        )
        self.system_prompt = get_system_prompt(valid_topics, invalid_topics)
        self.valid_topics = valid_topics
        if self.valid_topics is None:
            raise ValueError(
                "Valid Topics must be specified for TopicRestriction Validator"
            )
        self.invalid_topics = invalid_topics or []
        self.collect_metrics = collect_metrics

    def detect(self, text: str) -> JsonOutput:
        """
        Detects topics within the text using LLM.

        Args:
            text (str): The text to analyze for valid or invalid topics.

        Returns:
            JsonOutput: The assessment of the text's classification.
        """
        prompt = format_prompt(self.system_prompt, text)
        response = llm_response(self.provider, prompt, self.model, self.base_url)
        llm_result = parse_llm_response(response)

        # Adjusted logic for consistency with updated JSON structure
        if llm_result.classification == "valid_topic":
            result = JsonOutput(
                score=0,
                verdict="no",
                guard="topic_restriction",
                classification="valid_topic",
                explanation="Text fits into a valid topic.",
            )
        else:
            result = JsonOutput(
                score=1.0,
                verdict="yes",
                guard="topic_restriction",
                classification="invalid_topic",
                explanation="Text does not match any valid categories.",
            )

        if self.collect_metrics:
            guard_counter = guard_metrics()
            attributes = guard_metric_attributes(
                result.verdict,
                result.score,
                result.guard,
                result.classification,
                result.explanation,
            )
            guard_counter.add(1, attributes)

        return result
