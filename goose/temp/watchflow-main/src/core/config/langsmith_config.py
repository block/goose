"""
LangSmith configuration for agent debugging.
"""

from dataclasses import dataclass


@dataclass
class LangSmithConfig:
    """LangSmith configuration for agent debugging."""

    tracing_v2: bool = False
    endpoint: str = "https://api.smith.langchain.com"
    api_key: str = ""
    project: str = "watchflow-dev"
