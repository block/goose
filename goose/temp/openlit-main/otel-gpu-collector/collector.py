import openlit
import os
import logging
import sys
import json
import signal
import time
from threading import Event


# Create a JSON logger
class JSONFormatter(logging.Formatter):
    def format(self, record):
        log_record = {
            "level": record.levelname,
            "message": record.getMessage(),
            "timestamp": time.strftime(
                "%Y-%m-%d %H:%M:%S", time.localtime(record.created)
            ),
            "name": record.name,
            "application_name": record.application_name,
            "environment": record.environment,
        }
        if record.exc_info:
            log_record["exception"] = self.formatException(record.exc_info)
        return json.dumps(log_record)


# Custom filter to add fixed information to log records
class ContextFilter(logging.Filter):
    def __init__(self, application_name, environment):
        super().__init__()
        self.application_name = application_name
        self.environment = environment

    def filter(self, record):
        record.application_name = self.application_name
        record.environment = self.environment
        return True


def setup_logger(application_name, environment):
    logger = logging.getLogger("otel-gpu-collector")
    logger.setLevel(logging.INFO)

    # Log to stdout
    handler = logging.StreamHandler(sys.stdout)
    handler.setFormatter(JSONFormatter())

    logger.addHandler(handler)

    # Add context filter to logger
    logger.addFilter(ContextFilter(application_name, environment))

    return logger


# Load environment variables
application_name = os.getenv("GPU_APPLICATION_NAME", "default")
environment = os.getenv("GPU_ENVIRONMENT", "default")

# Initialize logger
logger = setup_logger(application_name, environment)

# Event to control the infinite loop
keep_running = Event()


def signal_handler(sig, frame):
    global keep_running
    logger.info("Received termination signal", extra={"signal": sig})
    keep_running.set()


def main():
    logger.info("Starting otel-gpu-collector")

    try:
        # Log environment variables
        logger.info("Environment variables loaded")

        # Initialize OpenLit with provided parameters
        logger.info("Initializing GPU OpenTelemetry Instrumentation library OpenLIT")
        openlit.init(
            collect_gpu_stats=True,
            application_name=application_name,
            environment=environment,
        )
        logger.info("Initialization complete")

        # Register signal handlers
        signal.signal(signal.SIGTERM, signal_handler)
        signal.signal(signal.SIGINT, signal_handler)

        # Log that signal handlers are registered
        logger.info(
            "Signal handlers registered", extra={"signals": ["SIGTERM", "SIGINT"]}
        )

        # Keep the script running indefinitely
        global keep_running
        keep_running.wait()

    except Exception as e:
        logger.error("An unexpected error occurred", exc_info=True)
    finally:
        logger.info("Shutting down GPU Monitor Service")


if __name__ == "__main__":
    main()
