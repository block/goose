"""
Centralized configuration for OpenLIT CLI

This module defines all openlit.init() parameters and their corresponding
CLI arguments and environment variables in one place to ensure consistency.
"""

import logging
import os
from typing import Dict, Any, Optional

logger = logging.getLogger(__name__)


# Centralized parameter configuration - SINGLE SOURCE OF TRUTH
# Add new parameters here ONLY - they will automatically appear in both CLI and openlit.init()
PARAMETER_CONFIG = {
    "environment": {
        "default": "default",
        "env_var": "OTEL_DEPLOYMENT_ENVIRONMENT",
        "cli_help": "Deployment environment",
        "cli_type": str,
    },
    "application_name": {
        "default": "default",
        "env_var": "OTEL_SERVICE_NAME",
        "cli_help": "Application name for tracing (deprecated, use service_name)",
        "cli_type": str,
        "deprecated": True,  # Mark as deprecated for future removal
    },
    "service_name": {
        "default": "default",
        "env_var": "OTEL_SERVICE_NAME",  # Same env var as application_name
        "cli_help": "Service name for tracing",
        "cli_type": str,
    },
    "otlp_endpoint": {
        "default": None,
        "env_var": "OTEL_EXPORTER_OTLP_ENDPOINT",
        "cli_help": "OTLP endpoint for exporter",
        "cli_type": str,
    },
    "otlp_headers": {
        "default": None,
        "env_var": "OTEL_EXPORTER_OTLP_HEADERS",
        "cli_help": "OTLP headers as JSON string",
        "cli_type": str,
        "parser": "json",  # Special handling needed
    },
    "disable_batch": {
        "default": False,
        "env_var": "OPENLIT_DISABLE_BATCH",
        "cli_help": "Disable batch span processing",
        "cli_type": bool,
    },
    "capture_message_content": {
        "default": True,
        "env_var": "OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT",
        "cli_help": "Enable capture of message content",
        "cli_type": bool,
        "has_negation": True,  # Supports --no_capture_message_content
    },
    "disabled_instrumentors": {
        "default": None,
        "env_var": "OPENLIT_DISABLED_INSTRUMENTORS",
        "cli_help": "Comma-separated list of instrumentors to disable",
        "cli_type": str,
        "parser": "csv",  # Special handling needed
    },
    "disable_metrics": {
        "default": False,
        "env_var": "OPENLIT_DISABLE_METRICS",
        "cli_help": "Disable metrics collection",
        "cli_type": bool,
    },
    "pricing_json": {
        "default": None,
        "env_var": "OPENLIT_PRICING_JSON",
        "cli_help": "File path or URL to pricing JSON",
        "cli_type": str,
    },
    "collect_gpu_stats": {
        "default": False,
        "env_var": "OPENLIT_COLLECT_GPU_STATS",
        "cli_help": "Enable GPU statistics collection",
        "cli_type": bool,
    },
    "detailed_tracing": {
        "default": True,
        "env_var": "OPENLIT_DETAILED_TRACING",
        "cli_help": "Enable detailed component-level tracing",
        "cli_type": bool,
        "has_negation": True,  # Supports --no_detailed_tracing
    },
    "collect_system_metrics": {
        "default": False,
        "env_var": "OPENLIT_COLLECT_SYSTEM_METRICS",
        "cli_help": "Enable comprehensive system metrics (CPU, memory, disk, network, GPU if detected)",
        "cli_type": bool,
    },
    "capture_parameters": {
        "default": False,
        "env_var": "OPENLIT_CAPTURE_PARAMETERS",
        "cli_help": "Capture database query parameters in spans (security risk - may expose sensitive data)",
        "cli_type": bool,
    },
    "enable_sqlcommenter": {
        "default": False,
        "env_var": "OPENLIT_ENABLE_SQLCOMMENTER",
        "cli_help": "Inject OpenTelemetry trace context as SQL comments for database log correlation",
        "cli_type": bool,
    },
    # Parameters that are not exposed via CLI (internal use only)
    "tracer": {
        "default": None,
        "internal": True,  # Skip in CLI
    },
    "event_logger": {
        "default": None,
        "internal": True,  # Skip in CLI
    },
    "meter": {
        "default": None,
        "internal": True,  # Skip in CLI
    },
}


def get_cli_parameters() -> Dict[str, Dict[str, Any]]:
    """Get parameters that should be exposed in the CLI."""
    return {
        name: config
        for name, config in PARAMETER_CONFIG.items()
        if not config.get("internal", False)
    }


def get_env_var_for_parameter(param_name: str) -> Optional[str]:
    """Get the environment variable name for a given parameter."""
    config = PARAMETER_CONFIG.get(param_name, {})
    return config.get("env_var")


def parse_env_value(param_name: str, env_value: str) -> Any:
    """Parse environment variable value based on parameter configuration."""
    config = PARAMETER_CONFIG.get(param_name, {})
    parser = config.get("parser")
    cli_type = config.get("cli_type", str)

    if cli_type == bool:
        return env_value.lower() in {"true", "1", "yes"}
    elif parser == "json":
        try:
            import json

            return json.loads(env_value)
        except (json.JSONDecodeError, ImportError):
            return None
    elif parser == "csv":
        return [item.strip() for item in env_value.split(",") if item.strip()]
    else:
        return env_value


def build_config_from_environment() -> Dict[str, Any]:
    """Build OpenLIT configuration from environment variables."""
    config = {}

    for param_name, param_config in PARAMETER_CONFIG.items():
        if param_config.get("internal", False):
            continue

        env_var = param_config.get("env_var")
        if not env_var:
            continue

        if env_value := os.environ.get(env_var):
            parsed_value = parse_env_value(param_name, env_value)
            if parsed_value is not None:
                config[param_name] = parsed_value

    return config
