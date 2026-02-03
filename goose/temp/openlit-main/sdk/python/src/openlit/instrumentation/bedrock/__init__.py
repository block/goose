"""Initializer of Auto Instrumentation of AWS Bedrock Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.bedrock.bedrock import converse, converse_stream

_instruments = ("boto3 >= 1.34.138",)


class BedrockInstrumentor(BaseInstrumentor):
    """
    An instrumentor for AWS Bedrock client library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        version = importlib.metadata.version("boto3")
        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics")

        # sync
        wrap_function_wrapper(
            "botocore.client",
            "ClientCreator.create_client",
            converse(
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

        # streaming
        wrap_function_wrapper(
            "botocore.client",
            "ClientCreator.create_client",
            converse_stream(
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
        pass
