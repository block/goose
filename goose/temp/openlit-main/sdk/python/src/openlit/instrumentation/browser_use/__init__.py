"""
Module for monitoring Browser-Use browser automation framework.
Supports comprehensive instrumentation of agent operations, browser actions, and task management.
"""

import logging
import importlib.metadata
from typing import Collection
from wrapt import wrap_function_wrapper

from opentelemetry.instrumentation.instrumentor import BaseInstrumentor

from openlit.semcov import SemanticConvention

logger = logging.getLogger(__name__)

_instruments = ("browser-use >= 0.1.0",)


class BrowserUseInstrumentor(BaseInstrumentor):
    """
    Instrumentor for Browser-Use browser automation framework
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        """Instrument Browser-Use operations"""

        # Get configuration
        tracer = kwargs.get("tracer")
        application_name = kwargs.get("application_name", "default_application")
        environment = kwargs.get("environment", "default_environment")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", True)
        metrics = kwargs.get("metrics", {})
        disable_metrics = kwargs.get("disable_metrics", False)

        try:
            version = importlib.metadata.version("browser-use")
        except Exception:
            version = "1.0.0"

        # Import the wrapper functions
        from openlit.instrumentation.browser_use.browser_use import general_wrap
        from openlit.instrumentation.browser_use.async_browser_use import (
            async_general_wrap,
        )

        # Instrument Agent async operations
        wrap_function_wrapper(
            "browser_use.agent.service",
            "Agent.run",
            async_general_wrap(
                "agent.run",
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
            "browser_use.agent.service",
            "Agent.step",
            async_general_wrap(
                "agent.step",
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

        # Instrument task management operations
        wrap_function_wrapper(
            "browser_use.agent.service",
            "Agent.pause",
            general_wrap(
                "agent.pause",
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
            "browser_use.agent.service",
            "Agent.resume",
            general_wrap(
                "agent.resume",
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
            "browser_use.agent.service",
            "Agent.stop",
            general_wrap(
                "agent.stop",
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

        # Instrument history operations (async methods)
        wrap_function_wrapper(
            "browser_use.agent.service",
            "Agent.rerun_history",
            async_general_wrap(
                "agent.rerun_history",
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
            "browser_use.agent.service",
            "Agent.load_and_rerun",
            async_general_wrap(
                "agent.load_and_rerun",
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

    def _uninstrument(self, **kwargs):
        """Uninstrument Browser-Use operations"""
        pass
