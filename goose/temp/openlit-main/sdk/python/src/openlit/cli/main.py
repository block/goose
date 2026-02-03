#!/usr/bin/env python3
"""
OpenLIT Auto-Instrumentation CLI

This module provides a CLI tool for auto-instrumenting Python applications
with OpenLIT observability without requiring code changes.

Usage:
    openlit-instrument --application_name myapp --otlp_endpoint https://cloud.openlit.io python app.py
"""

import os
import sys
import argparse
import logging
from shutil import which

from openlit.cli.config import get_cli_parameters

logger = logging.getLogger(__name__)


def parse_arguments() -> tuple:
    """Parse command line arguments and return parsed args and target command."""
    # Build environment variables help text
    env_vars_help = []
    cli_params = get_cli_parameters()
    for param_name, config in cli_params.items():
        env_var = config.get("env_var", "")
        help_text = config.get("cli_help", "")
        env_vars_help.append(f"  {env_var:<50} {help_text}")

    parser = argparse.ArgumentParser(
        prog="openlit-instrument",
        description="Auto-instrument Python applications with OpenLIT observability",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=f"""
Examples:
  # Using CLI arguments
  openlit-instrument --service-name myapp --deployment-environment production python app.py
  
  # Using environment variables (takes precedence over CLI args)
  OTEL_SERVICE_NAME=myapp OTEL_DEPLOYMENT_ENVIRONMENT=production openlit-instrument python app.py
  
  # Mixed usage
  openlit-instrument --otlp-endpoint https://cloud.openlit.io python app.py
  
  # Disable specific instrumentations
  openlit-instrument --disabled-instrumentors chromadb,pinecone python main.py

Environment Variables (take precedence over CLI arguments):
  Configure OpenLIT using these environment variables:
{chr(10).join(env_vars_help)}
        """,
    )

    # Dynamically add CLI arguments from PARAMETER_CONFIG
    cli_params = get_cli_parameters()
    for param_name, config in cli_params.items():
        cli_arg = f"--{param_name}"  # Use underscores like function parameters
        cli_help = (
            f"{config.get('cli_help', '')} (equivalent to {config.get('env_var', '')})"
        )
        cli_type = config.get("cli_type", str)

        if cli_type == bool:
            parser.add_argument(cli_arg, action="store_true", help=cli_help)
        else:
            parser.add_argument(cli_arg, help=cli_help)

    parser.add_argument("--version", action="version", version="%(prog)s 1.0.0")

    # Use OpenTelemetry's approach: command and command_args with REMAINDER
    parser.add_argument("command", help="Your Python application.")
    parser.add_argument(
        "command_args",
        help="Arguments for your application.",
        nargs=argparse.REMAINDER,
    )

    args = parser.parse_args()

    # Reconstruct target command from OpenTelemetry's format
    target_command = [args.command] + args.command_args

    return args, target_command


def set_environment_from_cli_args(args) -> None:
    """Set environment variables from CLI arguments (only if env vars are not already set)."""
    # Dynamically map CLI arguments to environment variables from PARAMETER_CONFIG
    cli_params = get_cli_parameters()

    for param_name, config in cli_params.items():
        env_var = config.get("env_var")
        if not env_var:
            continue

        # Only set if environment variable is not already set (env vars take precedence)
        if env_var not in os.environ:
            cli_value = getattr(args, param_name, None)
            if cli_value is not None:
                # Handle boolean values
                if isinstance(cli_value, bool):
                    # Skip setting env var if boolean is False and default is True
                    # (this means the flag wasn't provided, just the store_true default)
                    if cli_value is False and config.get("default") is True:
                        continue
                    os.environ[env_var] = "true" if cli_value else "false"
                else:
                    os.environ[env_var] = str(cli_value)


def setup_python_path() -> None:
    """Setup PYTHONPATH exactly like OpenTelemetry does."""
    python_path = os.environ.get("PYTHONPATH")

    python_path = [] if not python_path else python_path.split(os.pathsep)
    # Add current working directory (like OpenTelemetry does)
    cwd_path = os.getcwd()
    if cwd_path not in python_path:
        python_path.insert(0, cwd_path)

    # Add bootstrap directory path
    current_dir = os.path.dirname(os.path.abspath(__file__))
    bootstrap_path = os.path.join(current_dir, "bootstrap")

    # Ensure the directory exists
    os.makedirs(bootstrap_path, exist_ok=True)

    # Remove bootstrap path if it exists, then add at beginning
    python_path = [path for path in python_path if path != bootstrap_path]
    python_path.insert(0, bootstrap_path)

    os.environ["PYTHONPATH"] = os.pathsep.join(python_path)


def run() -> None:
    """Main entry point for openlit-instrument CLI."""
    try:
        # Parse CLI arguments and target command
        args, target_command = parse_arguments()

        # Set environment variables from CLI arguments (env vars take precedence)
        set_environment_from_cli_args(args)

        # Setup Python path for auto-initialization
        setup_python_path()

        # Execute target application using OpenTelemetry's approach
        executable = which(target_command[0])
        if not executable:
            logger.warning(
                "Command not found: %s. Attempting direct execution as fallback.",
                target_command[0],
            )
            # Fallback: try to execute directly (user's PATH might have it)
            try:
                os.execvpe(target_command[0], target_command, os.environ)
            except (FileNotFoundError, OSError) as e:
                logger.error("Failed to execute command %s: %s", target_command[0], e)
                logger.error(
                    "OpenLIT instrumentation failed, but this should not break your application"
                )
                return
        else:
            os.execl(executable, executable, *target_command[1:])

    except KeyboardInterrupt:
        # Only acceptable exit - user explicitly interrupted
        sys.exit(130)
    except Exception as e:
        logger.error("OpenLIT CLI failed: %s", e)
        logger.error(
            "This should not prevent your application from running. Consider running without openlit-instrument."
        )
        # Don't exit - let the process continue


if __name__ == "__main__":
    run()
