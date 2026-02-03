"""
Agent factory for creating agent instances by name.

Provides a simple interface to get agents by their type name,
centralizing agent instantiation for consistency.
"""

import logging
from typing import Any

from src.agents.acknowledgment_agent import AcknowledgmentAgent
from src.agents.base import BaseAgent
from src.agents.engine_agent import RuleEngineAgent
from src.agents.feasibility_agent import RuleFeasibilityAgent
from src.agents.repository_analysis_agent import RepositoryAnalysisAgent

logger = logging.getLogger(__name__)


def get_agent(agent_type: str, **kwargs: Any) -> BaseAgent:
    """
    Get an agent instance by type name.

    Args:
        agent_type: Type of agent ("engine", "feasibility", "acknowledgment")
        **kwargs: Additional configuration for the agent

    Returns:
        Agent instance

    Raises:
        ValueError: If agent_type is not supported

    Examples:
        >>> engine_agent = get_agent("engine")
        >>> feasibility_agent = get_agent("feasibility")
        >>> acknowledgment_agent = get_agent("acknowledgment")
        >>> analysis_agent = get_agent("repository_analysis")
    """
    agent_type = agent_type.lower()

    if agent_type == "engine":
        return RuleEngineAgent(**kwargs)
    elif agent_type == "feasibility":
        return RuleFeasibilityAgent(**kwargs)
    elif agent_type == "acknowledgment":
        return AcknowledgmentAgent(**kwargs)
    elif agent_type == "repository_analysis":
        return RepositoryAnalysisAgent(**kwargs)
    else:
        supported = ", ".join(["engine", "feasibility", "acknowledgment", "repository_analysis"])
        raise ValueError(f"Unsupported agent type: {agent_type}. Supported: {supported}")
