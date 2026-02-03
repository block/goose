"""OpenLLMetry auto-instrumentation sitecustomize.py"""

import os
import sys

sys.path.insert(0, "/openlit-sdk")

try:
    # Use traceloop-sdk for OpenLLMetry instrumentation
    from traceloop.sdk import Traceloop

    # Initialize Traceloop with environment variables
    Traceloop.init(
        app_name=os.environ.get("TRACELOOP_APP_NAME", "openllmetry-app"),
        api_endpoint=os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT")
        or os.environ.get("TRACELOOP_API_ENDPOINT", "http://localhost:4318"),
        headers=os.environ.get("OTEL_EXPORTER_OTLP_HEADERS"),
        disable_batch=os.environ.get("TRACELOOP_DISABLE_BATCH", "false").lower()
        == "true",
    )

    print("✅ OpenLLMetry auto-instrumentation initialized with traceloop-sdk!")
    print(f"🎯 App: {os.environ.get('TRACELOOP_APP_NAME', 'openllmetry-app')}")
    print(f"🔗 Endpoint: {os.environ.get('OTEL_EXPORTER_OTLP_ENDPOINT', 'default')}")

except Exception as e:
    print(f"⚠️ OpenLLMetry initialization failed: {e}")
    # Fallback to basic OpenTelemetry setup
    try:
        from opentelemetry import trace
        from opentelemetry.exporter.otlp.proto.http.trace_exporter import (
            OTLPSpanExporter,
        )
        from opentelemetry.sdk.trace import TracerProvider
        from opentelemetry.sdk.trace.export import BatchSpanProcessor

        tracer_provider = TracerProvider()
        otlp_exporter = OTLPSpanExporter(
            endpoint=os.environ.get(
                "OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318/v1/traces"
            )
        )
        tracer_provider.add_span_processor(BatchSpanProcessor(otlp_exporter))
        trace.set_tracer_provider(tracer_provider)

        print("✅ OpenLLMetry fallback initialization successful!")
    except Exception as fallback_e:
        print(f"❌ OpenLLMetry fallback failed: {fallback_e}")
        import traceback

        traceback.print_exc()
