"""
OpenLIT CLI

This module provides CLI-based auto-instrumentation functionality similar to
OpenTelemetry's approach, allowing users to instrument their applications
without code changes using the existing OpenLIT instrumentations.

All openlit.init() parameters can be set via CLI arguments or environment variables.
CLI arguments take precedence over environment variables.
"""

from .main import run

__all__ = ["run"]
