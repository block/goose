# Core Utilities Module

This module provides shared utilities for retry logic, caching, logging, metrics, and timeout handling that can be used across the Watchflow codebase.

## Modules

### `retry.py` - Retry Utilities

Provides decorators and functions for retrying async operations with exponential backoff.

**Functions:**
- `retry_with_backoff()` - Decorator for retrying async functions
- `retry_async()` - Function for retrying async function calls

**Example:**
```python
from src.core.utils.retry import retry_with_backoff

@retry_with_backoff(max_retries=3, initial_delay=1.0)
async def fetch_data():
    return await api_call()
```

### `timeout.py` - Timeout Utilities

Provides functions for executing async operations with timeout handling.

**Functions:**
- `execute_with_timeout()` - Execute coroutine with timeout
- `timeout_decorator()` - Decorator for adding timeout to async functions

**Example:**
```python
from src.core.utils.timeout import execute_with_timeout

result = await execute_with_timeout(
    long_operation(),
    timeout=60.0
)
```

### `caching.py` - Caching Utilities

Provides async-friendly caching with TTL support.

**Classes:**
- `AsyncCache` - Cache with TTL and automatic expiration

**Functions:**
- `cached_async()` - Decorator for caching async function results

**Example:**
```python
from src.core.utils.caching import AsyncCache, cached_async

cache = AsyncCache(maxsize=100, ttl=3600)

@cached_async(cache=cache, key_func=lambda repo: f"repo:{repo}")
async def fetch_repo_data(repo: str):
    return await api_call(repo)
```

### `logging.py` - Structured Logging Utilities

Provides context managers and decorators for structured operation logging.

**Functions:**
- `log_operation()` - Context manager for structured operation logging
- `log_function_call()` - Decorator for logging function calls

**Example:**
```python
from src.core.utils.logging import log_operation

async with log_operation("rule_evaluation", repo=repo, pr=pr_number):
    result = await evaluate_rules(...)
```

### `metrics.py` - Performance Metrics Utilities

Provides utilities for tracking and recording performance metrics.

**Functions:**
- `track_metrics()` - Context manager for tracking operation metrics
- `metrics_decorator()` - Decorator for tracking function call metrics

**Example:**
```python
from src.core.utils.metrics import track_metrics

async with track_metrics("rule_evaluation", rule_count=5) as metrics:
    result = await evaluate_rules(...)
    metrics["violations_found"] = len(result.violations)
```

## Usage in Codebase

### Updated Files

The following files have been updated to use the new utilities:

1. **`src/agents/base.py`**
   - `_retry_structured_output()` now uses `retry_async()`
   - `_execute_with_timeout()` now uses `execute_with_timeout()`

2. **`src/integrations/contributors.py`**
   - Replaced manual cache implementation with `AsyncCache`

### Migration Guide

If you have code using the old patterns, here's how to migrate:

**Old retry pattern:**
```python
for attempt in range(max_retries):
    try:
        return await func()
    except Exception as e:
        if attempt == max_retries - 1:
            raise
        await asyncio.sleep(delay * (2 ** attempt))
```

**New retry pattern:**
```python
from src.core.utils.retry import retry_async

result = await retry_async(
    func,
    max_retries=3,
    initial_delay=1.0
)
```

**Old timeout pattern:**
```python
try:
    return await asyncio.wait_for(coro, timeout=30.0)
except TimeoutError:
    raise Exception("Operation timed out")
```

**New timeout pattern:**
```python
from src.core.utils.timeout import execute_with_timeout

result = await execute_with_timeout(coro, timeout=30.0)
```

**Old cache pattern:**
```python
cache: dict[str, dict] = {}
if key in cache:
    cached_data = cache[key]
    if time.time() - cached_data["timestamp"] < ttl:
        return cached_data["value"]
```

**New cache pattern:**
```python
from src.core.utils.caching import AsyncCache

cache = AsyncCache(maxsize=100, ttl=3600)
cached_value = cache.get(key)
if cached_value is not None:
    return cached_value
```

## Benefits

1. **Code Reusability** - Common patterns extracted into reusable utilities
2. **Consistency** - Same retry/cache/timeout behavior across the codebase
3. **Maintainability** - Single place to update retry/cache logic
4. **Testability** - Utilities can be tested independently
5. **Type Safety** - Full type hints for better IDE support
