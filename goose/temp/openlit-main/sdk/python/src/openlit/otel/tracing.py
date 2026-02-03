"""
Setups up OpenTelemetry tracer
"""

import os
import logging
from opentelemetry import trace
from opentelemetry.sdk.resources import (
    SERVICE_NAME,
    TELEMETRY_SDK_NAME,
    DEPLOYMENT_ENVIRONMENT,
)
from opentelemetry.sdk.resources import Resource
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor, SimpleSpanProcessor
from opentelemetry.sdk.trace.export import ConsoleSpanExporter
from openlit.__helpers import parse_exporters

# pylint: disable=ungrouped-imports
if os.environ.get("OTEL_EXPORTER_OTLP_PROTOCOL") == "grpc":
    from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
else:
    from opentelemetry.exporter.otlp.proto.http.trace_exporter import OTLPSpanExporter
# pylint: enable=ungrouped-imports

logger = logging.getLogger(__name__)

# Global flag to check if the tracer provider initialization is complete.
TRACER_SET = False


def setup_tracing(
    application_name, environment, tracer, otlp_endpoint, otlp_headers, disable_batch
):
    """
    Sets up tracing with OpenTelemetry.
    Initializes the tracer provider and configures the span processor and exporter.
    """

    # If an external tracer is provided, return it immediately.
    if tracer is not None:
        return tracer

    # Proceed with setting up a new tracer or configuration only if TRACER_SET is False.
    # pylint: disable=global-statement
    global TRACER_SET

    try:
        # Disable Haystack Auto Tracing
        os.environ["HAYSTACK_AUTO_TRACE_ENABLED"] = "false"

        if not TRACER_SET:
            # Create a resource with the service name attribute.
            resource = Resource.create(
                attributes={
                    SERVICE_NAME: application_name,
                    DEPLOYMENT_ENVIRONMENT: environment,
                    TELEMETRY_SDK_NAME: "openlit",
                }
            )

            # Initialize the TracerProvider with the created resource.
            trace.set_tracer_provider(TracerProvider(resource=resource))

            # Only set environment variables if you have a non-None value.
            if otlp_endpoint is not None:
                os.environ["OTEL_EXPORTER_OTLP_ENDPOINT"] = otlp_endpoint

            if otlp_headers is not None:
                if isinstance(otlp_headers, dict):
                    headers_str = ",".join(
                        f"{key}={value}" for key, value in otlp_headers.items()
                    )
                else:
                    headers_str = otlp_headers

                os.environ["OTEL_EXPORTER_OTLP_HEADERS"] = headers_str

            # Check for OTEL_TRACES_EXPORTER env var for multiple exporters support
            exporters_config = parse_exporters("OTEL_TRACES_EXPORTER")
            processors_added = False

            if exporters_config is not None:
                # New behavior: use specified exporters from OTEL_TRACES_EXPORTER
                for exporter_name in exporters_config:
                    if exporter_name == "otlp":
                        span_exporter = OTLPSpanExporter()
                        span_processor = (
                            BatchSpanProcessor(span_exporter)
                            if not disable_batch
                            else SimpleSpanProcessor(span_exporter)
                        )
                        trace.get_tracer_provider().add_span_processor(span_processor)
                        processors_added = True
                    elif exporter_name == "console":
                        span_exporter = ConsoleSpanExporter()
                        span_processor = SimpleSpanProcessor(span_exporter)
                        trace.get_tracer_provider().add_span_processor(span_processor)
                        processors_added = True
                    elif exporter_name == "none":
                        # "none" means no exporter, skip
                        continue
                    else:
                        logger.warning("Unknown trace exporter: %s", exporter_name)

                # Warn if no valid exporters were configured
                if not processors_added:
                    logger.warning(
                        "OTEL_TRACES_EXPORTER is set but no valid exporters configured. "
                        "Trace export is disabled. Valid exporters: otlp, console"
                    )
            else:
                # Default behavior: use OTEL_EXPORTER_OTLP_ENDPOINT check
                if os.getenv("OTEL_EXPORTER_OTLP_ENDPOINT"):
                    span_exporter = OTLPSpanExporter()
                    # pylint: disable=line-too-long
                    span_processor = (
                        BatchSpanProcessor(span_exporter)
                        if not disable_batch
                        else SimpleSpanProcessor(span_exporter)
                    )
                else:
                    span_exporter = ConsoleSpanExporter()
                    span_processor = SimpleSpanProcessor(span_exporter)

                trace.get_tracer_provider().add_span_processor(span_processor)

            TRACER_SET = True

        return trace.get_tracer(__name__)

    # pylint: disable=bare-except
    except:
        return None
