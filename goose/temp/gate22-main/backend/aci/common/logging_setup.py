import logging
from logging.handlers import RotatingFileHandler


# the setup is called once at the start of the app
def setup_logging(
    formatter: logging.Formatter | None = None,
    aci_log_level: str = "INFO",
    filters: list[logging.Filter] | None = None,
    include_file_handler: bool = False,
    file_path: str | None = None,
) -> None:
    if filters is None:
        filters = []

    if formatter is None:
        formatter = logging.Formatter("%(asctime)s - %(name)s - %(levelname)s - %(message)s")

    # Set the root logger level to WARNING to reduce noise
    root_logger = logging.getLogger()
    root_logger.setLevel(logging.WARNING)

    # Create a console handler (for output to console)
    console_handler = logging.StreamHandler()
    console_handler.setFormatter(formatter)
    for filter in filters:
        console_handler.addFilter(filter)
    root_logger.addHandler(console_handler)

    if include_file_handler:
        if file_path is None:
            raise ValueError("file_path must be provided if include_file_handler is True")
        file_handler = RotatingFileHandler(file_path, maxBytes=10485760, backupCount=10)
        file_handler.setFormatter(formatter)
        for filter in filters:
            file_handler.addFilter(filter)
        root_logger.addHandler(file_handler)

    # set aci log level
    aci_logger = logging.getLogger("aci")
    aci_logger.setLevel(aci_log_level)


def get_logger(name: str) -> logging.Logger:
    return logging.getLogger(name)
