# pylint: disable=useless-return, bad-staticmethod-argument, disable=duplicate-code
"""Initializer of Auto Instrumentation of Juelp Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.julep.julep import wrap_julep

from openlit.instrumentation.julep.async_julep import async_wrap_julep

_instruments = ("julep >= 1.42.0",)


class JulepInstrumentor(BaseInstrumentor):
    """
    An instrumentor for Julep's client library.
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
        version = importlib.metadata.version("julep")

        # sync
        wrap_function_wrapper(
            "julep.resources.agents.agents",
            "AgentsResource.create",
            wrap_julep(
                "julep.agents_create",
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
            "julep.resources.tasks",
            "TasksResource.create",
            wrap_julep(
                "julep.task_create",
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
            "julep.resources.executions.executions",
            "ExecutionsResource.create",
            wrap_julep(
                "julep.execution_create",
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

        # async
        wrap_function_wrapper(
            "julep.resources.agents.agents",
            "AsyncAgentsResource.create",
            async_wrap_julep(
                "julep.agents_create",
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
            "julep.resources.tasks",
            "AsyncTasksResource.create",
            async_wrap_julep(
                "julep.task_create",
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
            "julep.resources.executions.executions",
            "AsyncExecutionsResource.create",
            async_wrap_julep(
                "julep.execution_create",
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
        # Proper uninstrumentation logic to revert patched methods
        pass
