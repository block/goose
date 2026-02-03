# pylint: disable=duplicate-code, line-too-long, ungrouped-imports
"""
Setups up OpenTelemetry Meter
"""

import os
import logging
from opentelemetry import metrics
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import (
    PeriodicExportingMetricReader,
    ConsoleMetricExporter,
)
from opentelemetry.sdk.resources import (
    SERVICE_NAME,
    TELEMETRY_SDK_NAME,
    DEPLOYMENT_ENVIRONMENT,
)
from opentelemetry.sdk.resources import Resource
from openlit.semcov import SemanticConvention
from openlit.__helpers import parse_exporters

if os.environ.get("OTEL_EXPORTER_OTLP_PROTOCOL") == "grpc":
    from opentelemetry.exporter.otlp.proto.grpc.metric_exporter import (
        OTLPMetricExporter,
    )
else:
    from opentelemetry.exporter.otlp.proto.http.metric_exporter import (
        OTLPMetricExporter,
    )

logger = logging.getLogger(__name__)

# Global flag to check if the meter provider initialization is complete.
METER_SET = False


_DB_CLIENT_OPERATION_DURATION_BUCKETS = [0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1, 5, 10]

_GEN_AI_CLIENT_OPERATION_DURATION_BUCKETS = [
    0.01,
    0.02,
    0.04,
    0.08,
    0.16,
    0.32,
    0.64,
    1.28,
    2.56,
    5.12,
    10.24,
    20.48,
    40.96,
    81.92,
]

_GEN_AI_SERVER_TBT = [
    0.01,
    0.025,
    0.05,
    0.075,
    0.1,
    0.15,
    0.2,
    0.3,
    0.4,
    0.5,
    0.75,
    1.0,
    2.5,
]

_GEN_AI_SERVER_TFTT = [
    0.001,
    0.005,
    0.01,
    0.02,
    0.04,
    0.06,
    0.08,
    0.1,
    0.25,
    0.5,
    0.75,
    1.0,
    2.5,
    5.0,
    7.5,
    10.0,
]

_GEN_AI_CLIENT_TOKEN_USAGE_BUCKETS = [
    1,
    4,
    16,
    64,
    256,
    1024,
    4096,
    16384,
    65536,
    262144,
    1048576,
    4194304,
    16777216,
    67108864,
]

# MCP-specific bucket boundaries for performance and size metrics
_MCP_CLIENT_OPERATION_DURATION_BUCKETS = [
    0.001,  # 1ms
    0.005,  # 5ms
    0.01,  # 10ms
    0.05,  # 50ms
    0.1,  # 100ms
    0.5,  # 500ms
    1.0,  # 1s
    2.0,  # 2s
    5.0,  # 5s
    10.0,  # 10s
]

_MCP_PAYLOAD_SIZE_BUCKETS = [
    100,  # 100 bytes
    500,  # 500 bytes
    1024,  # 1KB
    5120,  # 5KB
    10240,  # 10KB
    51200,  # 50KB
    102400,  # 100KB
    512000,  # 500KB
    1048576,  # 1MB
    5242880,  # 5MB
]


def setup_meter(application_name, environment, meter, otlp_endpoint, otlp_headers):
    """
    Sets up OpenTelemetry metrics with a counter for total requests.

    Params:
        application_name (str): The name of the application for which metrics are being collected.
        otlp_endpoint (str): The OTLP exporter endpoint for metrics.
        otlp_headers (dict): Headers for the OTLP request.

    Returns:
        A dictionary containing the meter and created metrics for easy access.
    """

    # pylint: disable=global-statement
    global METER_SET

    try:
        if meter is None and not METER_SET:
            # Create a resource with the service name attribute.
            resource = Resource.create(
                attributes={
                    SERVICE_NAME: application_name,
                    DEPLOYMENT_ENVIRONMENT: environment,
                    TELEMETRY_SDK_NAME: "openlit",
                }
            )

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
                # Now, we have either converted the dict to a string or used the provided string.
                os.environ["OTEL_EXPORTER_OTLP_HEADERS"] = headers_str

            # Check for OTEL_METRICS_EXPORTER env var for multiple exporters support
            exporters_config = parse_exporters("OTEL_METRICS_EXPORTER")
            metric_readers = []

            if exporters_config is not None:
                # New behavior: use specified exporters from OTEL_METRICS_EXPORTER
                for exporter_name in exporters_config:
                    if exporter_name == "otlp":
                        metric_exporter = OTLPMetricExporter()
                        metric_readers.append(
                            PeriodicExportingMetricReader(metric_exporter)
                        )
                    elif exporter_name == "console":
                        metric_exporter = ConsoleMetricExporter()
                        metric_readers.append(
                            PeriodicExportingMetricReader(metric_exporter)
                        )
                    elif exporter_name == "none":
                        # "none" means no exporter, skip
                        continue
                    else:
                        logger.warning("Unknown metric exporter: %s", exporter_name)

                # Warn if no valid exporters were configured
                if not metric_readers:
                    logger.warning(
                        "OTEL_METRICS_EXPORTER is set but no valid exporters configured. "
                        "Metric export is disabled. Valid exporters: otlp, console"
                    )
            else:
                # Default behavior: use OTEL_EXPORTER_OTLP_ENDPOINT check
                if os.getenv("OTEL_EXPORTER_OTLP_ENDPOINT"):
                    metric_exporter = OTLPMetricExporter()
                else:
                    metric_exporter = ConsoleMetricExporter()
                metric_readers.append(PeriodicExportingMetricReader(metric_exporter))

            meter_provider = MeterProvider(
                resource=resource, metric_readers=metric_readers
            )

            metrics.set_meter_provider(meter_provider)

            meter = metrics.get_meter(__name__, version="0.1.0")

            METER_SET = True

        # Define and create the metrics
        metrics_dict = {
            # OTel Semconv
            "genai_client_usage_tokens": meter.create_histogram(
                name=SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE,
                description="Measures number of input and output tokens used",
                unit="{token}",
                explicit_bucket_boundaries_advisory=_GEN_AI_CLIENT_TOKEN_USAGE_BUCKETS,
            ),
            "genai_client_operation_duration": meter.create_histogram(
                name=SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION,
                description="GenAI operation duration",
                unit="s",
                explicit_bucket_boundaries_advisory=_GEN_AI_CLIENT_OPERATION_DURATION_BUCKETS,
            ),
            "genai_server_tbt": meter.create_histogram(
                name=SemanticConvention.GEN_AI_SERVER_TBT,
                description="Time per output token generated after the first token for successful responses",
                unit="s",
                explicit_bucket_boundaries_advisory=_GEN_AI_SERVER_TBT,
            ),
            "genai_server_ttft": meter.create_histogram(
                name=SemanticConvention.GEN_AI_SERVER_TTFT,
                description="Time to generate first token for successful responses",
                unit="s",
                explicit_bucket_boundaries_advisory=_GEN_AI_SERVER_TFTT,
            ),
            "db_client_operation_duration": meter.create_histogram(
                name=SemanticConvention.DB_CLIENT_OPERATION_DURATION,
                description="DB operation duration",
                unit="s",
                explicit_bucket_boundaries_advisory=_DB_CLIENT_OPERATION_DURATION_BUCKETS,
            ),
            # Extra
            "genai_requests": meter.create_counter(
                name=SemanticConvention.GEN_AI_REQUESTS,
                description="Number of requests to GenAI",
                unit="1",
            ),
            "genai_prompt_tokens": meter.create_counter(
                name=SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS,
                description="Number of prompt tokens processed.",
                unit="1",
            ),
            "genai_completion_tokens": meter.create_counter(
                name=SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS,
                description="Number of completion tokens processed.",
                unit="1",
            ),
            "genai_reasoning_tokens": meter.create_counter(
                name=SemanticConvention.GEN_AI_USAGE_REASONING_TOKENS,
                description="Number of reasoning thought tokens processed.",
                unit="1",
            ),
            "genai_cost": meter.create_histogram(
                name=SemanticConvention.GEN_AI_USAGE_COST,
                description="The distribution of GenAI request costs.",
                unit="USD",
            ),
            "db_requests": meter.create_counter(
                name=SemanticConvention.DB_REQUESTS,
                description="Number of requests to VectorDBs",
                unit="1",
            ),
            # MCP-specific metrics for business intelligence and operational insights
            "mcp_requests": meter.create_counter(
                name=SemanticConvention.MCP_REQUESTS,
                description="Number of requests to MCP servers by operation type",
                unit="1",
            ),
            "mcp_client_operation_duration": meter.create_histogram(
                name=SemanticConvention.MCP_CLIENT_OPERATION_DURATION_METRIC,
                description="MCP client operation duration",
                unit="s",
                explicit_bucket_boundaries_advisory=_MCP_CLIENT_OPERATION_DURATION_BUCKETS,
            ),
            "mcp_request_size": meter.create_histogram(
                name=SemanticConvention.MCP_REQUEST_SIZE,
                description="Size of MCP request payloads",
                unit="By",
                explicit_bucket_boundaries_advisory=_MCP_PAYLOAD_SIZE_BUCKETS,
            ),
            "mcp_response_size": meter.create_histogram(
                name=SemanticConvention.MCP_RESPONSE_SIZE_METRIC,
                description="Size of MCP response payloads",
                unit="By",
                explicit_bucket_boundaries_advisory=_MCP_PAYLOAD_SIZE_BUCKETS,
            ),
            "mcp_tool_calls": meter.create_counter(
                name=SemanticConvention.MCP_TOOL_CALLS,
                description="Number of MCP tool calls by tool name",
                unit="1",
            ),
            "mcp_resource_reads": meter.create_counter(
                name=SemanticConvention.MCP_RESOURCE_READS,
                description="Number of MCP resource reads by resource type",
                unit="1",
            ),
            "mcp_prompt_gets": meter.create_counter(
                name=SemanticConvention.MCP_PROMPT_GETS,
                description="Number of MCP prompt retrievals by prompt name",
                unit="1",
            ),
            "mcp_transport_usage": meter.create_counter(
                name=SemanticConvention.MCP_TRANSPORT_USAGE,
                description="MCP transport type usage (stdio, sse, websocket)",
                unit="1",
            ),
            "mcp_errors": meter.create_counter(
                name=SemanticConvention.MCP_ERRORS,
                description="Number of MCP operation errors by operation type",
                unit="1",
            ),
            "mcp_operation_success_rate": meter.create_histogram(
                name=SemanticConvention.MCP_OPERATION_SUCCESS_RATE,
                description="MCP operation success rate by operation type",
                unit="1",
            ),
        }

        return metrics_dict, None

    # pylint: disable=broad-exception-caught
    except Exception as err:
        return None, err
