"""Base auto-instrumentation sitecustomize.py for custom configurations"""

import os
import sys

sys.path.insert(0, "/openlit-sdk")

try:
    # Basic OpenTelemetry setup for custom configurations
    from opentelemetry import trace
    from opentelemetry.exporter.otlp.proto.http.trace_exporter import OTLPSpanExporter
    from opentelemetry.sdk.trace import TracerProvider
    from opentelemetry.sdk.trace.export import BatchSpanProcessor

    # Configure OpenTelemetry with environment variables
    tracer_provider = TracerProvider()

    # Support both HTTP and gRPC endpoints
    endpoint = os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT")
    if endpoint:
        if endpoint.endswith("/v1/traces"):
            # HTTP endpoint
            otlp_exporter = OTLPSpanExporter(endpoint=endpoint)
        else:
            # gRPC endpoint - let user handle this with their custom packages
            print(f"🔧 gRPC endpoint detected: {endpoint}")
            print("📦 Make sure to include gRPC exporter in CUSTOM_PACKAGES")
            otlp_exporter = OTLPSpanExporter(endpoint=f"{endpoint}/v1/traces")

        tracer_provider.add_span_processor(BatchSpanProcessor(otlp_exporter))
        trace.set_tracer_provider(tracer_provider)

        print("✅ Base OpenTelemetry initialized for custom instrumentation!")
        print(f"🎯 Service: {os.environ.get('OTEL_SERVICE_NAME', 'custom-app')}")
        print(f"🔗 Endpoint: {endpoint}")
    else:
        print("⚠️  No OTEL_EXPORTER_OTLP_ENDPOINT provided")
        print("🔧 Set up tracing in your custom packages")

except Exception as e:
    print(f"⚠️ Base instrumentation setup failed: {e}")
    print("🔧 Your custom packages should handle instrumentation setup")
    import traceback

    traceback.print_exc()
