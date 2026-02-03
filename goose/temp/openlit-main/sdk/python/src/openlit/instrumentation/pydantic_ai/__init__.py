"""Initializer of Auto Instrumentation of Pydantic AI Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.pydantic_ai.pydantic_ai import (
    agent_create,
    agent_run,
    graph_execution,
    user_prompt_processing,
    model_request_processing,
    tool_calls_processing,
)
from openlit.instrumentation.pydantic_ai.async_pydantic_ai import (
    async_agent_run,
)

_instruments = ("pydantic-ai >= 0.2.17",)


class PydanticAIInstrumentor(BaseInstrumentor):
    """
    An instrumentor for Pydantic AI's client library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        application_name = kwargs.get("application_name", "default_application")
        environment = kwargs.get("environment", "default_environment")
        tracer = kwargs.get("tracer")
        metrics = kwargs.get("metrics_dict")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        disable_metrics = kwargs.get("disable_metrics")
        version = importlib.metadata.version("pydantic-ai")

        wrap_function_wrapper(
            "pydantic_ai.agent",
            "Agent.__init__",
            agent_create(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        wrap_function_wrapper(
            "pydantic_ai.agent",
            "Agent.run_sync",
            agent_run(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        wrap_function_wrapper(
            "pydantic_ai.agent",
            "Agent.run",
            async_agent_run(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        # Enhanced instrumentation for richer span hierarchy
        # These wrap internal Pydantic AI graph execution components
        try:
            # Agent.iter() - Graph execution iterator
            wrap_function_wrapper(
                "pydantic_ai.agent",
                "Agent.iter",
                graph_execution(
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            # If Agent.iter doesn't exist, skip this instrumentation
            pass

        try:
            # UserPromptNode.run() - User prompt processing
            wrap_function_wrapper(
                "pydantic_ai._agent_graph",
                "UserPromptNode.run",
                user_prompt_processing(
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            # If UserPromptNode.run doesn't exist, skip this instrumentation
            pass

        try:
            # ModelRequestNode.run() - Model request processing
            wrap_function_wrapper(
                "pydantic_ai._agent_graph",
                "ModelRequestNode.run",
                model_request_processing(
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            # If ModelRequestNode.run doesn't exist, skip this instrumentation
            pass

        try:
            # CallToolsNode.run() - Tool calls processing
            wrap_function_wrapper(
                "pydantic_ai._agent_graph",
                "CallToolsNode.run",
                tool_calls_processing(
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            # If CallToolsNode.run doesn't exist, skip this instrumentation
            pass

    def _uninstrument(self, **kwargs):
        # Proper uninstrumentation logic to revert patched methods
        pass
