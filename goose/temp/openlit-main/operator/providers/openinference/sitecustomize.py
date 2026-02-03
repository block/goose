"""
OpenInference auto-instrumentation sitecustomize.py
Using Pure OpenTelemetry approach that works with OpenLIT
Based on successful comprehensive local test results
"""

import os
import sys

sys.path.insert(0, "/openlit-sdk")


def setup_openinference():
    """Setup comprehensive OpenInference instrumentation using pure OpenTelemetry"""

    print("🚀 COMPREHENSIVE OpenInference → OpenLIT Auto-Instrumentation")
    print("📋 Using Pure OpenTelemetry + ALL Available OpenInference Instrumentors")
    print("=" * 80)

    try:
        # 1. Setup Pure OpenTelemetry (that works with OpenLIT)
        from opentelemetry import trace
        from opentelemetry.exporter.otlp.proto.http.trace_exporter import (
            OTLPSpanExporter,
        )
        from opentelemetry.sdk.trace import TracerProvider
        from opentelemetry.sdk.trace.export import SimpleSpanProcessor
        from opentelemetry.sdk.resources import Resource
        from opentelemetry.semconv.resource import ResourceAttributes

        print("🔧 Setting up Pure OpenTelemetry...")

        # Get configuration from environment variables
        service_name = os.environ.get("OTEL_SERVICE_NAME", "openinference-app")
        otlp_endpoint = os.environ.get(
            "OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318/v1/traces"
        )
        environment = os.environ.get("OTEL_DEPLOYMENT_ENVIRONMENT", "production")

        # Create resource with service name
        resource = Resource.create(
            {
                ResourceAttributes.SERVICE_NAME: service_name,
                ResourceAttributes.SERVICE_VERSION: "1.0.0",
                ResourceAttributes.DEPLOYMENT_ENVIRONMENT: environment,
            }
        )

        # Create tracer provider
        tracer_provider = TracerProvider(resource=resource)

        # Create OTLP exporter (that works with OpenLIT)
        otlp_exporter = OTLPSpanExporter(endpoint=otlp_endpoint)

        # Create span processor (use Simple for immediate sending)
        span_processor = SimpleSpanProcessor(otlp_exporter)
        tracer_provider.add_span_processor(span_processor)

        # Set global tracer provider
        trace.set_tracer_provider(tracer_provider)

        print("✅ Pure OpenTelemetry configured for OpenLIT!")
        print(f"🎯 Service: {service_name}")
        print(f"🔗 Endpoint: {otlp_endpoint}")
        print(f"🌍 Environment: {environment}")

        # 2. Enable ALL Available OpenInference Instrumentors
        print("\n📦 Enabling ALL OpenInference Instrumentors...")

        instrumentors = []

        # Enable all available instrumentors
        instrumentor_configs = [
            ("openinference.instrumentation.openai", "OpenAIInstrumentor", "OpenAI"),
            (
                "openinference.instrumentation.anthropic",
                "AnthropicInstrumentor",
                "Anthropic",
            ),
            (
                "openinference.instrumentation.langchain",
                "LangChainInstrumentor",
                "LangChain",
            ),
            (
                "openinference.instrumentation.llama_index",
                "LlamaIndexInstrumentor",
                "LlamaIndex",
            ),
            ("openinference.instrumentation.bedrock", "BedrockInstrumentor", "Bedrock"),
            (
                "openinference.instrumentation.mistralai",
                "MistralAIInstrumentor",
                "MistralAI",
            ),
            ("openinference.instrumentation.groq", "GroqInstrumentor", "Groq"),
            (
                "openinference.instrumentation.vertexai",
                "VertexAIInstrumentor",
                "VertexAI",
            ),
            ("openinference.instrumentation.dspy", "DSPyInstrumentor", "DSPy"),
            (
                "openinference.instrumentation.instructor",
                "InstructorInstrumentor",
                "Instructor",
            ),
            ("openinference.instrumentation.litellm", "LiteLLMInstrumentor", "LiteLLM"),
            (
                "openinference.instrumentation.haystack",
                "HaystackInstrumentor",
                "Haystack",
            ),
            (
                "openinference.instrumentation.guardrails",
                "GuardrailsInstrumentor",
                "Guardrails",
            ),
            ("openinference.instrumentation.portkey", "PortkeyInstrumentor", "Portkey"),
        ]

        for module_name, class_name, display_name in instrumentor_configs:
            try:
                module = __import__(module_name, fromlist=[class_name])
                instrumentor_class = getattr(module, class_name)
                instrumentor_class().instrument()
                instrumentors.append(display_name)
                print(f"✅ {display_name} Instrumentor enabled")
            except Exception as e:
                # Don't print warnings for missing optional dependencies
                if "No module named" not in str(e) and "DependencyConflict" not in str(
                    e
                ):
                    print(f"⚠️ {display_name} Instrumentor failed: {e}")

        print(
            f"\n📊 Successfully enabled {len(instrumentors)} instrumentors: {', '.join(instrumentors)}"
        )
        print("\n" + "=" * 80)
        print("🎉 COMPREHENSIVE OPENINFERENCE AUTO-INSTRUMENTATION COMPLETED!")
        print("📊 Configuration Summary:")
        print(f"   • Service: {service_name}")
        print(f"   • Endpoint: {otlp_endpoint} (Pure OpenTelemetry)")
        print(f"   • Environment: {environment}")
        print(f"   • Instrumentors: {len(instrumentors)} enabled")
        print(f"   • Auto-Instrumentation: ✅ All available frameworks")
        print("\n🔗 This approach works with OpenLIT while Phoenix OTEL doesn't!")

        return True

    except Exception as e:
        print(f"❌ OpenInference comprehensive setup failed: {e}")
        import traceback

        traceback.print_exc()
        return False


# Initialize OpenInference instrumentation
try:
    setup_openinference()
except Exception as e:
    print(f"❌ Failed to initialize OpenInference: {e}")
    import traceback

    traceback.print_exc()
