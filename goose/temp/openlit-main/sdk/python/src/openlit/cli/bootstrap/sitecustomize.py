"""
OpenLIT Auto-Initialization Bootstrap

This module automatically initializes OpenLIT when Python starts
if OPENLIT_AUTO_INSTRUMENT environment variable is set.

This follows the same pattern as OpenTelemetry's auto-instrumentation.
"""

import os
import logging

# Import the centralized configuration
try:
    # First try absolute import
    from openlit.cli.config import build_config_from_environment
except ImportError:
    try:
        # Try relative import as fallback
        from ..config import build_config_from_environment
    except ImportError:
        # Final fallback - build config manually from environment
        def build_config_from_environment():
            """Build configuration from environment variables as fallback."""

            config = {}

            # Map environment variables to OpenLIT parameters
            env_mappings = {
                "OTEL_SERVICE_NAME": "application_name",
                "OTEL_DEPLOYMENT_ENVIRONMENT": "environment",
                "OTEL_EXPORTER_OTLP_ENDPOINT": "otlp_endpoint",
                "OTEL_EXPORTER_OTLP_HEADERS": "otlp_headers",
                "OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT": "capture_message_content",
                "OPENLIT_DISABLED_INSTRUMENTORS": "disabled_instrumentors",
                "OPENLIT_DISABLE_BATCH": "disable_batch",
                "OPENLIT_DISABLE_METRICS": "disable_metrics",
                "OPENLIT_COLLECT_GPU_STATS": "collect_gpu_stats",
                "OPENLIT_DETAILED_TRACING": "detailed_tracing",
                "OPENLIT_PRICING_JSON": "pricing_json",
                "OPENLIT_CAPTURE_PARAMETERS": "capture_parameters",
                "OPENLIT_ENABLE_SQLCOMMENTER": "enable_sqlcommenter",
            }

            for env_var, param_name in env_mappings.items():
                if env_value := os.environ.get(env_var):
                    # Handle boolean values
                    if env_var in [
                        "OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT",
                        "OPENLIT_DISABLE_BATCH",
                        "OPENLIT_DISABLE_METRICS",
                        "OPENLIT_COLLECT_GPU_STATS",
                        "OPENLIT_DETAILED_TRACING",
                        "OPENLIT_CAPTURE_PARAMETERS",
                        "OPENLIT_ENABLE_SQLCOMMENTER",
                    ]:
                        config[param_name] = env_value.lower() in ("true", "1", "yes")
                    # Handle CSV values
                    elif env_var == "OPENLIT_DISABLED_INSTRUMENTORS":
                        config[param_name] = [
                            item.strip()
                            for item in env_value.split(",")
                            if item.strip()
                        ]
                    # Handle JSON values
                    elif env_var == "OTEL_EXPORTER_OTLP_HEADERS":
                        try:
                            import json

                            config[param_name] = json.loads(env_value)
                        except (json.JSONDecodeError, ImportError):
                            pass
                    else:
                        config[param_name] = env_value

            return config


# Set up logging for auto-instrumentation
logger = logging.getLogger(__name__)


def initialize(*, swallow_exceptions: bool = True) -> None:
    """
    Setup auto-instrumentation, called by the sitecustomize module

    :param swallow_exceptions: Whether or not to propagate instrumentation exceptions to the caller.
                              Exceptions are logged and swallowed by default.
    """
    # Prevents auto-instrumentation of subprocesses if code execs another python process
    if "PYTHONPATH" in os.environ:
        current_dir = os.path.dirname(os.path.abspath(__file__))
        python_path = os.environ["PYTHONPATH"].split(os.pathsep)
        # Remove current directory from PYTHONPATH to prevent subprocess instrumentation
        python_path = [path for path in python_path if path != current_dir]
        os.environ["PYTHONPATH"] = os.pathsep.join(python_path)

    try:
        # Import OpenLIT and initialize
        import openlit

        # Build configuration from environment variables
        config = build_config_from_environment()

        # Initialize OpenLIT with environment variables
        openlit.init(**config)

    except ImportError as e:
        if not swallow_exceptions:
            raise
        logger.error("OpenLIT not found. Please ensure openlit is installed: %s", e)
    except Exception as e:
        if not swallow_exceptions:
            raise
        # Log error but don't break the application
        logger.error("OpenLIT auto-instrumentation failed: %s", e)


# Auto-initialize when module is imported (like OpenTelemetry does)
initialize()
