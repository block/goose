"""Retry extension for automatic retry with exponential backoff"""

import logging
import random
import time
from functools import wraps
from typing import Callable, TypeVar, cast

from .base import Extension

logger = logging.getLogger(__name__)

T = TypeVar("T")


class RetryExtension(Extension):
    """Add retry logic to client methods"""

    def __init__(
        self,
        client,
        max_retries: int = 3,
        backoff_factor: float = 2.0,
        max_wait: float = 30.0,
        retry_on: tuple = (Exception,),
    ):
        """Initialize the retry extension.

        Args:
            client: The GooseClient instance to extend
            max_retries: Maximum number of retry attempts
            backoff_factor: Exponential backoff multiplier
            max_wait: Maximum wait time between retries in seconds
            retry_on: Tuple of exception types to retry on
        """
        super().__init__(client)
        self.max_retries = max_retries
        self.backoff_factor = backoff_factor
        self.max_wait = max_wait
        self.retry_on = retry_on

    def install(self):
        """Wrap client methods with retry logic"""
        # Store original methods
        self._original_methods = {}

        # Methods to wrap with retry
        methods_to_wrap = [
            "create_session",
            "send_message",
            "list_sessions",
            "get_session",
            "delete_session",
            "list_tools",
            "list_extensions",
            "health_check",
        ]

        for method_name in methods_to_wrap:
            if hasattr(self.client, method_name):
                original_method = getattr(self.client, method_name)
                self._original_methods[method_name] = original_method
                wrapped_method = self._with_retry(original_method)
                setattr(self.client, method_name, wrapped_method)
                logger.debug(f"Wrapped {method_name} with retry logic")

    def uninstall(self):
        """Restore original methods"""
        for method_name, original_method in self._original_methods.items():
            setattr(self.client, method_name, original_method)
            logger.debug(f"Restored original {method_name}")
        self._original_methods.clear()

    def _with_retry(self, func: Callable[..., T]) -> Callable[..., T]:
        """Add retry logic to a function.

        Args:
            func: The function to wrap

        Returns:
            The wrapped function with retry logic
        """

        @wraps(func)
        def wrapper(*args, **kwargs) -> T:
            last_error = None

            for attempt in range(self.max_retries + 1):
                try:
                    return func(*args, **kwargs)
                except self.retry_on as e:
                    last_error = e

                    if attempt < self.max_retries:
                        # Calculate wait time with exponential backoff
                        wait = min(
                            self.backoff_factor**attempt + random.uniform(0, 1), self.max_wait
                        )

                        logger.warning(
                            f"Attempt {attempt + 1} failed for {func.__name__}: {e}. "
                            f"Retrying in {wait:.2f} seconds..."
                        )
                        time.sleep(wait)
                    else:
                        logger.error(
                            f"All {self.max_retries + 1} attempts failed for {func.__name__}"
                        )

            # If we get here, all retries failed
            if last_error:
                raise last_error
            else:
                # This shouldn't happen, but handle it gracefully
                raise RuntimeError(f"Retry logic failed unexpectedly for {func.__name__}")

        return cast(Callable[..., T], wrapper)
