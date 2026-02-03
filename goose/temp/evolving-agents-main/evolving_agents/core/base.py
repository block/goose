# evolving_agents/core/base.py

"""Base definitions to prevent circular imports."""

from typing import Any, Protocol

class IAgent(Protocol):
    """Interface for an agent that can run prompts."""
    
    async def run(self, prompt: str, **kwargs) -> Any:
        """Run the agent with a prompt."""
        ...