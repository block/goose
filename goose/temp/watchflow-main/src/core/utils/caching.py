"""
Caching utilities for async operations.

Provides async-friendly caching with TTL support and decorators
for caching function results.
"""

import logging
from collections.abc import Callable
from datetime import datetime
from functools import wraps
from typing import Any

from cachetools import TTLCache  # type: ignore

logger = logging.getLogger(__name__)


class AsyncCache:
    """
    Async-friendly cache with TTL support.

    This cache stores values with timestamps and automatically
    expires entries based on TTL.

    Example:
        cache = AsyncCache(maxsize=100, ttl=3600)
        cache.set("key", "value")
        value = cache.get("key")
    """

    def __init__(self, maxsize: int = 100, ttl: int = 3600):
        """
        Initialize async cache.

        Args:
            maxsize: Maximum number of entries in cache
            ttl: Time to live in seconds
        """
        self._cache: dict[str, dict[str, Any]] = {}
        self.maxsize = maxsize
        self.ttl = ttl

    def get(self, key: str) -> Any | None:
        """
        Get cached value if not expired.

        Args:
            key: Cache key

        Returns:
            Cached value or None if not found or expired
        """
        if key not in self._cache:
            return None

        cached_data = self._cache[key]
        age = datetime.now().timestamp() - cached_data.get("timestamp", 0)

        if age >= self.ttl:
            del self._cache[key]
            logger.debug(f"Cache entry '{key}' expired (age: {age:.2f}s, ttl: {self.ttl}s)")
            return None

        logger.debug(f"Cache hit for '{key}'")
        return cached_data.get("value")

    def set(self, key: str, value: Any) -> None:
        """
        Set cached value with timestamp.

        Args:
            key: Cache key
            value: Value to cache
        """
        if len(self._cache) >= self.maxsize:
            # Remove oldest entry
            oldest_key = min(
                self._cache.keys(),
                key=lambda k: self._cache[k].get("timestamp", 0),
            )
            logger.debug(f"Cache full, evicting oldest entry '{oldest_key}'")
            del self._cache[oldest_key]

        self._cache[key] = {
            "value": value,
            "timestamp": datetime.now().timestamp(),
        }
        logger.debug(f"Cached entry '{key}'")

    def clear(self) -> None:
        """Clear all cached values."""
        count = len(self._cache)
        self._cache.clear()
        logger.debug(f"Cleared {count} cache entries")

    def invalidate(self, key: str) -> None:
        """
        Invalidate a specific cache entry.

        Args:
            key: Cache key to invalidate
        """
        if key in self._cache:
            del self._cache[key]
            logger.debug(f"Invalidated cache entry '{key}'")

    def size(self) -> int:
        """
        Get current cache size.

        Returns:
            Number of entries in cache
        """
        return len(self._cache)


# Simple module-level cache used by recommendations API
_GLOBAL_CACHE = AsyncCache(maxsize=1024, ttl=3600)


async def get_cache(key: str) -> Any | None:
    """
    Async helper to fetch from the module-level cache.
    """
    return _GLOBAL_CACHE.get(key)


async def set_cache(key: str, value: Any, ttl: int | None = None) -> None:
    """
    Async helper to store into the module-level cache.
    """
    if ttl and ttl != _GLOBAL_CACHE.ttl:
        _GLOBAL_CACHE.ttl = ttl
    _GLOBAL_CACHE.set(key, value)


def cached_async(
    cache: AsyncCache | TTLCache | None = None,
    key_func: Callable[..., str] | None = None,
    ttl: int | None = None,
    maxsize: int = 100,
) -> Any:
    """
    Decorator for caching async function results.

    Args:
        cache: Cache instance to use (creates new AsyncCache if None)
        key_func: Function to generate cache key from function arguments
        ttl: Time to live in seconds (only used if cache is None)
        maxsize: Maximum cache size (only used if cache is None)

    Returns:
        Decorated async function with caching

    Example:
        @cached_async(ttl=3600, key_func=lambda repo, *args: f"repo:{repo}")
        async def fetch_repo_data(repo: str):
            return await api_call(repo)
    """
    if cache is None:
        # SIM108: Use ternary operator
        cache = AsyncCache(maxsize=maxsize, ttl=ttl) if ttl else TTLCache(maxsize=maxsize, ttl=ttl or 3600)

    def decorator(func: Callable[..., Any]) -> Callable[..., Any]:
        @wraps(func)
        async def wrapper(*args: Any, **kwargs: Any) -> Any:
            # Generate cache key
            # SIM108: Use ternary operator
            cache_key = key_func(*args, **kwargs) if key_func else f"{func.__name__}:{args}:{kwargs}"

            # Check cache
            # SIM108: Use ternary operator or unified interface
            # Both AsyncCache and TTLCache support .get()
            cached_value = cache.get(cache_key)

            if cached_value is not None:
                logger.debug(f"Cache hit for {func.__name__} with key '{cache_key}'")
                return cached_value

            # Cache miss - execute function
            logger.debug(f"Cache miss for {func.__name__} with key '{cache_key}'")
            result = await func(*args, **kwargs)

            # Store in cache
            if isinstance(cache, AsyncCache):
                cache.set(cache_key, result)
            else:
                # TTLCache
                cache[cache_key] = result

            return result

        return wrapper

    return decorator
