# pylint: disable=useless-return, bad-staticmethod-argument, disable=duplicate-code
"""Initializer of Auto Instrumentation of MultiOn Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.multion.multion import multion_wrap

from openlit.instrumentation.multion.async_multion import async_multion_wrap

_instruments = ("multion >= 1.3.8",)


class MultiOnInstrumentor(BaseInstrumentor):
    """
    An instrumentor for multion's client library.
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
        version = importlib.metadata.version("multion")

        # Synchronus
        wrap_function_wrapper(
            "multion.client",
            "MultiOn.browse",
            multion_wrap(
                "multion.browse",
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
            "multion.client",
            "MultiOn.retrieve",
            multion_wrap(
                "multion.retrieve",
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
            "multion.sessions.client",
            "SessionsClient.create",
            multion_wrap(
                "multion.sessions.create",
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

        # Asynchronus
        wrap_function_wrapper(
            "multion.client",
            "AsyncMultiOn.browse",
            async_multion_wrap(
                "multion.browse",
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
            "multion.client",
            "AsyncMultiOn.retrieve",
            async_multion_wrap(
                "multion.retrieve",
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
            "multion.sessions.client",
            "AsyncSessionsClient.create",
            async_multion_wrap(
                "multion.sessions.create",
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
