"""
Base agent classes and utilities for agents.
"""

import logging
from abc import ABC, abstractmethod
from typing import Any, TypeVar, cast

from pydantic import BaseModel, Field

from src.core.utils.timeout import execute_with_timeout

logger = logging.getLogger(__name__)

T = TypeVar("T")


class AgentResult(BaseModel):
    """Result from an agent execution."""

    success: bool
    message: str
    data: dict[str, Any] = Field(default_factory=dict)
    metadata: dict[str, Any] = Field(default_factory=dict)


class BaseAgent(ABC):
    """
    Base class for all Watchflow agents with enhanced error handling and retry logic.

    Features:
    - Retry logic with exponential backoff for structured output
    - Timeout handling for all operations
    - Enhanced error reporting and logging
    - Performance metrics tracking
    """

    def __init__(self, max_retries: int = 3, retry_delay: float = 1.0, agent_name: str | None = None):
        self.max_retries = max_retries
        self.retry_delay = retry_delay
        self.agent_name = agent_name
        # Lazy import to avoid circular imports and heavy initialization at module load.
        from src.integrations.providers import get_chat_model

        self.llm = get_chat_model(agent=agent_name)
        self.graph = self._build_graph()
        logger.info(f"🔧 {self.__class__.__name__} initialized with max_retries={max_retries}, agent_name={agent_name}")

    @abstractmethod
    def _build_graph(self) -> Any:
        """
        Build the LangGraph workflow.
        Returns Any here to allow specific subclasses to define their unique CompiledStateGraph parameters.
        """
        pass

    async def _retry_structured_output(self, llm: Any, output_model: Any, prompt: str, **kwargs: Any) -> T:
        """
        Retry structured output with exponential backoff.

        Args:
            llm: The LLM client
            output_model: Pydantic model for structured output
            prompt: The prompt to send to the LLM
            **kwargs: Additional arguments for the LLM call

        Returns:
            Structured output conforming to the output_model

        Raises:
            Exception: If all retries fail
        """
        from src.core.utils.retry import retry_async

        structured_llm = llm.with_structured_output(output_model)

        async def _invoke_structured() -> T:
            """Inner function to invoke structured LLM."""
            response = await structured_llm.ainvoke(prompt, **kwargs)
            return cast("T", response)

        return await retry_async(
            _invoke_structured,
            max_retries=self.max_retries,
            initial_delay=self.retry_delay,
            exceptions=(Exception,),
        )

    async def _execute_with_timeout(self, coro: Any, timeout: float = 60.0) -> Any:
        """
        Execute a coroutine with timeout handling.

        Args:
            coro: The coroutine to execute
            timeout: Timeout in seconds (default: 60s for showcase stability)

        Returns:
            The result of the coroutine

        Raises:
            TimeoutError: If the operation times out
        """
        return await execute_with_timeout(
            coro,
            timeout=timeout,
            timeout_message=f"Operation timed out after {timeout} seconds",
        )

    @abstractmethod
    async def execute(self, **kwargs: Any) -> AgentResult:
        """
        Execute the agent with given parameters.
        Explicitly typed **kwargs to satisfy MyPy strict mode.
        """
        pass
