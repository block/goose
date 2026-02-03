"""OpenLIT auto-instrumentation sitecustomize.py using CLI bootstrap approach"""

import os
import sys

# Use configurable instrumentation path from environment or fallback to default
instrumentation_path = os.environ.get("PYTHONPATH", "/instrumentation-packages").split(
    ":"
)[0]
sys.path.insert(0, instrumentation_path)

try:
    # Use OpenLIT CLI bootstrap initialization (proper auto-instrumentation approach)
    # Simply importing the module will automatically call initialize()
    import openlit.cli.bootstrap.sitecustomize

    print("✅ OpenLIT auto-instrumentation initialized via CLI bootstrap!")
    print(f"🎯 Service: {os.environ.get('OTEL_SERVICE_NAME', 'openlit-app')}")
    print(f"🔗 Endpoint: {os.environ.get('OTEL_EXPORTER_OTLP_ENDPOINT', 'default')}")
    print(
        f"🌍 Environment: {os.environ.get('OTEL_DEPLOYMENT_ENVIRONMENT', 'production')}"
    )
    print("🚀 Using OpenLIT CLI bootstrap for zero-code instrumentation")

except Exception as e:
    print(f"⚠️ OpenLIT CLI bootstrap failed: {e}")
    # Fallback to manual init if CLI approach fails
    try:
        import openlit

        openlit.init(
            otlp_endpoint=os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT"),
            otlp_headers=os.environ.get("OTEL_EXPORTER_OTLP_HEADERS"),
            application_name=os.environ.get("OTEL_SERVICE_NAME", "openlit-app"),
            environment=os.environ.get("OTEL_DEPLOYMENT_ENVIRONMENT", "production"),
            capture_message_content=os.environ.get(
                "OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT", "true"
            ).lower()
            == "true",
            detailed_tracing=os.environ.get("OPENLIT_DETAILED_TRACING", "true").lower()
            == "true",
        )

        print("✅ OpenLIT fallback initialization successful!")
    except Exception as fallback_e:
        print(f"❌ OpenLIT fallback failed: {fallback_e}")
        import traceback

        traceback.print_exc()
