import logging

from fastapi import FastAPI
from opentelemetry import metrics, trace
from opentelemetry._logs import set_logger_provider
from opentelemetry.exporter.otlp.proto.grpc._log_exporter import OTLPLogExporter
from opentelemetry.exporter.otlp.proto.grpc.metric_exporter import OTLPMetricExporter
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.instrumentation.botocore import BotocoreInstrumentor
from opentelemetry.instrumentation.fastapi import FastAPIInstrumentor
from opentelemetry.instrumentation.httpx import HTTPXClientInstrumentor
from opentelemetry.instrumentation.logging import LoggingInstrumentor
from opentelemetry.instrumentation.openai_v2 import OpenAIInstrumentor
from opentelemetry.instrumentation.psycopg import PsycopgInstrumentor
from opentelemetry.instrumentation.sqlalchemy import SQLAlchemyInstrumentor
from opentelemetry.sdk._logs import LoggerProvider, LoggingHandler
from opentelemetry.sdk._logs.export import BatchLogRecordProcessor
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import PeriodicExportingMetricReader
from opentelemetry.sdk.resources import SERVICE_NAME, Resource
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor

from aci.common.enums import Environment

logger = logging.getLogger(__name__)


def setup_telemetry(
    app: FastAPI,
    environment: Environment,
    otlp_endpoint: str,
) -> None:
    """
    Setup OpenTelemetry instrumentation for traces, metrics, and logs.

    Args:
        app: FastAPI application instance
        environment: Current environment (LOCAL, DEV, PROD, etc.)
        otlp_endpoint: OTLP collector endpoint for traces, metrics, and logs
        (share the same endpoint for all signals)
    """

    logger.info(
        f"Setting up OpenTelemetry for "
        f"service={app.title}, "
        f"environment={environment}, "
        f"otlp_endpoint={otlp_endpoint}"
    )
    # ==================== RESOURCE SETUP ====================
    resource = Resource(
        attributes={
            SERVICE_NAME: app.title,
            "environment": environment,
        }
    )

    # NOTE: we use sidecar collector in both local and production so "insecure" is always True
    # ==================== TRACES ====================
    logger.info("Configuring OTLP trace")
    trace_provider = TracerProvider(resource=resource)
    otlp_trace_exporter = OTLPSpanExporter(endpoint=otlp_endpoint, insecure=True)
    trace_provider.add_span_processor(BatchSpanProcessor(otlp_trace_exporter))
    trace.set_tracer_provider(trace_provider)

    # ==================== METRICS ====================
    logger.info("Configuring OTLP metric")
    metric_readers = []
    otlp_metric_exporter = OTLPMetricExporter(endpoint=otlp_endpoint, insecure=True)
    metric_readers.append(
        PeriodicExportingMetricReader(otlp_metric_exporter, export_interval_millis=60000)
    )
    meter_provider = MeterProvider(resource=resource, metric_readers=metric_readers)
    metrics.set_meter_provider(meter_provider)

    # ==================== LOGS ====================
    logger.info("Configuring OTLP log")
    log_provider = LoggerProvider(resource=resource)
    otlp_log_exporter = OTLPLogExporter(endpoint=otlp_endpoint, insecure=True)
    log_provider.add_log_record_processor(BatchLogRecordProcessor(otlp_log_exporter))
    set_logger_provider(log_provider)
    # Attach OTLP handler to root logger
    handler = LoggingHandler(level=logging.NOTSET, logger_provider=log_provider)
    logging.getLogger().addHandler(handler)

    # ==================== INSTRUMENTATION ====================
    logger.info("Instrumenting libraries with OpenTelemetry")

    # FastAPI instrumentation (exclude health checks and docs endpoints using regex)
    FastAPIInstrumentor.instrument_app(
        app, excluded_urls=".*/health$|.*/docs.*|.*/redoc.*|.*/openapi.json$"
    )

    # HTTPX instrumentation (for outgoing HTTP requests)
    HTTPXClientInstrumentor().instrument()

    # SQLAlchemy instrumentation (for database queries)
    SQLAlchemyInstrumentor().instrument(enable_commenter=True)

    # Psycopg instrumentation (for PostgreSQL queries)
    PsycopgInstrumentor().instrument()

    # Botocore/Boto3 instrumentation (for AWS SDK calls)
    BotocoreInstrumentor().instrument()  # type: ignore

    # OpenAI instrumentation (for OpenAI API calls)
    OpenAIInstrumentor().instrument()  # type: ignore

    # Logging instrumentation (adds trace context to logs)
    LoggingInstrumentor().instrument(set_logging_format=True)

    logger.info("OpenTelemetry instrumentation setup complete")


def get_tracer(name: str) -> trace.Tracer:
    """
    Get a tracer instance for manual instrumentation.

    Args:
        name: Name of the tracer (typically __name__ of the module)

    Returns:
        Tracer instance
    """
    return trace.get_tracer(name)


def get_meter(name: str) -> metrics.Meter:
    """
    Get a meter instance for custom metrics.

    Args:
        name: Name of the meter (typically __name__ of the module)

    Returns:
        Meter instance for creating counters, histograms, gauges, etc.

    Example:
        meter = get_meter(__name__)
        request_counter = meter.create_counter(
            "http_requests_total",
            description="Total HTTP requests",
            unit="1"
        )
        request_counter.add(1, {"method": "GET", "endpoint": "/api/v1/users"})
    """
    return metrics.get_meter(name)
