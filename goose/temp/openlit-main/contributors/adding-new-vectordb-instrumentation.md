# Vector Database Instrumentation Guide

This guide provides step-by-step instructions for adding a new vector database instrumentation to OpenLIT, following the established patterns and conventions learned from Pinecone and ChromaDB implementations.

## Table of Contents
1. [File Structure](#file-structure)
2. [Code Structure](#code-structure)
3. [Naming Conventions](#naming-conventions)
4. [Code Style Guidelines](#code-style-guidelines)
5. [Implementation Steps](#implementation-steps)
6. [Testing](#testing)
7. [Integration](#integration)

## File Structure

Every vector database instrumentation follows a consistent **3-4 file structure**:

```
sdk/python/src/openlit/instrumentation/{database_name}/
├── __init__.py          # Instrumentor class with dependency requirements
├── {database_name}.py   # Sync wrapper functions
├── async_{database_name}.py  # Async wrapper functions (only if database supports async)
└── utils.py            # Telemetry processing logic
```

### Example Structure (using "pinecone" as example):
```
sdk/python/src/openlit/instrumentation/pinecone/
├── __init__.py
├── pinecone.py
├── async_pinecone.py    # Only if async operations exist
└── utils.py
```

**Note**: If the database doesn't support async operations (like ChromaDB), omit the `async_{database_name}.py` file.

## Code Structure

### 1. DB_OPERATION_MAP Pattern

All vector database instrumentations use a centralized operation mapping in `utils.py`:

```python
# Operation mapping for simple span naming
DB_OPERATION_MAP = {
    "{database}.create_collection": SemanticConvention.DB_OPERATION_CREATE_COLLECTION,
    "{database}.add": SemanticConvention.DB_OPERATION_INSERT,
    "{database}.get": SemanticConvention.DB_OPERATION_GET,
    "{database}.query": SemanticConvention.DB_OPERATION_GET,  # Both get() and query() are GET operations
    "{database}.update": SemanticConvention.DB_OPERATION_UPDATE,
    "{database}.upsert": SemanticConvention.DB_OPERATION_UPSERT,
    "{database}.delete": SemanticConvention.DB_OPERATION_DELETE,
}
```

**Key Learning**: Multiple endpoints can map to the same semantic operation. Use the `endpoint` parameter to differentiate implementation details.

### 2. General Wrap Pattern

Use the **general_wrap** pattern for efficiency:

```python
def general_wrap(gen_ai_endpoint, version, environment, application_name, tracer, 
                pricing_info, capture_message_content, metrics, disable_metrics):
    """
    Create a wrapper for {database} operations using the general wrap pattern.
    """
    def wrapper(wrapped, instance, args, kwargs):
        # CRITICAL: Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)
        
        # Get server address and port using the standard helper
        server_address, server_port = set_server_address_and_port(instance)
        
        db_operation = DB_OPERATION_MAP.get(gen_ai_endpoint, "unknown")
        if db_operation == "create_collection":
            namespace = kwargs.get("name") or (args[0] if args else "unknown")
        else:
            namespace = getattr(instance, "name", "unknown")  # For collection-based databases
        span_name = f"{db_operation} {namespace}"
        
        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)
            
            try:
                # Process response and generate telemetry
                response = process_vectordb_response(
                    response, db_operation, server_address, server_port,
                    environment, application_name, metrics, start_time, span,
                    capture_message_content, disable_metrics, version, 
                    instance, args, endpoint=gen_ai_endpoint, **kwargs
                )
                
            except Exception as e:
                handle_exception(span, e)
                
            return response
    
    return wrapper
```

### 3. Simplified Telemetry Pattern

Based on ChromaDB learnings, use a simplified, consistent pattern:

```python
def common_vectordb_logic(scope, environment, application_name, 
    metrics, capture_message_content, disable_metrics, version, 
    instance=None, endpoint=None):
    """
    Process vector database request and generate telemetry.
    """
    scope._end_time = time.time()

    # Set common database span attributes using helper
    common_db_span_attributes(scope, SemanticConvention.DB_SYSTEM_{DATABASE_NAME}, 
        scope._server_address, scope._server_port, environment, application_name, version)

    # Set DB operation specific attributes
    scope._span.set_attribute(SemanticConvention.DB_OPERATION_NAME, scope._db_operation)
    scope._span.set_attribute(SemanticConvention.DB_CLIENT_OPERATION_DURATION, 
        scope._end_time - scope._start_time)

    # Handle operations with endpoint differentiation
    if scope._db_operation == SemanticConvention.DB_OPERATION_GET:
        collection_name = getattr(instance, "name", "unknown")
        
        # Differentiate between different GET operations
        if endpoint == "{database}.get":
            query = scope._kwargs.get("ids", [])
            
            # Standard database attributes
            scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(query))
            scope._span.set_attribute(SemanticConvention.DB_COLLECTION_NAME, collection_name)
            scope._span.set_attribute(SemanticConvention.DB_VECTOR_COUNT, object_count(query))
            
            # Database-specific attributes
            scope._span.set_attribute(SemanticConvention.DB_FILTER, 
                str(scope._kwargs.get("where", "")))
            
            # Response metrics
            if scope._response and scope._response.get('ids'):
                returned_rows = object_count(scope._response['ids'])  # Adjust for response structure
                scope._span.set_attribute(SemanticConvention.DB_RESPONSE_RETURNED_ROWS, returned_rows)
            
            scope._span.set_attribute(SemanticConvention.DB_QUERY_SUMMARY,
                f"{scope._db_operation} {collection_name} "
                f"ids={query} "
                f"limit={scope._kwargs.get('limit', 'None')}")
        
        elif endpoint == "{database}.query":
            # Handle query-specific parameters
            query_texts = scope._kwargs.get("query_texts", [])
            query_embeddings = scope._kwargs.get("query_embeddings", [])
            
            # Create query content representation
            if query_texts:
                query_content = f"texts={query_texts}"
            elif query_embeddings:
                query_content = f"embeddings={len(query_embeddings)} vectors"
            else:
                query_content = "no query provided"
            
            # Standard database attributes
            scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, query_content)
            scope._span.set_attribute(SemanticConvention.DB_COLLECTION_NAME, collection_name)
            scope._span.set_attribute(SemanticConvention.DB_VECTOR_QUERY_TOP_K, 
                scope._kwargs.get("n_results", 10))
            
            # Response metrics (adjust for nested response structure)
            if scope._response and scope._response.get('ids'):
                returned_rows = object_count(scope._response['ids'][0]) if scope._response['ids'] else 0
                scope._span.set_attribute(SemanticConvention.DB_RESPONSE_RETURNED_ROWS, returned_rows)
            
            scope._span.set_attribute(SemanticConvention.DB_QUERY_SUMMARY,
                f"{scope._db_operation} {collection_name} "
                f"n_results={scope._kwargs.get('n_results', 10)} "
                f"{query_content}")

    elif scope._db_operation == SemanticConvention.DB_OPERATION_INSERT:
        collection_name = getattr(instance, "name", "unknown")
        query = scope._kwargs.get("ids", [])
        
        # Standard database attributes
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(query))
        scope._span.set_attribute(SemanticConvention.DB_COLLECTION_NAME, collection_name)
        scope._span.set_attribute(SemanticConvention.DB_VECTOR_COUNT, object_count(query))
        
        scope._span.set_attribute(SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"ids={query} "
            f"documents={scope._kwargs.get('documents', 'None')}")

    # ... similar patterns for other operations

    scope._span.set_status(Status(StatusCode.OK))

    # Record metrics using helper
    if not disable_metrics:
        record_db_metrics(metrics, SemanticConvention.DB_SYSTEM_{DATABASE_NAME}, 
            scope._server_address, scope._server_port, environment, application_name, 
            scope._start_time, scope._end_time)
```

### 4. Key Telemetry Patterns

**Essential Attributes for All Operations:**
- `DB_QUERY_TEXT`: String representation of the query/operation
- `DB_COLLECTION_NAME`: Collection name (for collection-based databases)
- `DB_VECTOR_COUNT`: Count of primary query parameter
- `DB_QUERY_SUMMARY`: Comprehensive operation summary

**Query Summary Format:**
```python
f"{scope._db_operation} {collection_name} key_param={value} optional_param={value or 'None'}"
```

**Response Structure Handling:**
```python
# For flat arrays (like ChromaDB GET)
returned_rows = object_count(scope._response['ids'])

# For nested arrays (like ChromaDB QUERY)
returned_rows = object_count(scope._response['ids'][0]) if scope._response['ids'] else 0
```

## Naming Conventions

### 1. File Names
- Use lowercase with underscores: `{database_name}.py`
- Async files: `async_{database_name}.py` (only if needed)
- No hyphens, use underscores

### 2. Function Names
- Use `snake_case` consistently
- Standard function names:
  - `general_wrap`
  - `async_general_wrap` (if needed)
  - `common_vectordb_logic`
  - `process_vectordb_response`

### 3. Variable Names
- Use `snake_case` for all variables
- **Simplified naming**: Use `query` for primary operation parameter
- Standard variable names:
  - `server_address`, `server_port`
  - `start_time`, `end_time`
  - `db_operation`
  - `collection_name` (for collection-based databases)

### 4. Class Names
- Use `PascalCase` for classes
- Instrumentor class: `{DatabaseName}Instrumentor`

## Code Style Guidelines

### 1. Quotes
- **Always use double quotes** for strings: `"example"`
- **Never use single quotes** for strings: ❌ `'example'`
- Consistent across all files including inline comments, error messages, and string literals

### 2. Indentation
- **4 spaces** for indentation
- No tabs
- **Multi-line definitions**: Use **only one level** of indentation for continuation lines
  ```python
  # ✅ Correct: One level indentation for multi-line function definitions
  def general_wrap(gen_ai_endpoint, version, environment, application_name,
      tracer, pricing_info, capture_message_content, metrics, disable_metrics):
  
  # ✅ Correct: One level indentation for multi-line function calls
  response = process_vectordb_response(
      response, db_operation, server_address, server_port,
      environment, application_name, metrics, start_time, span,
      capture_message_content, disable_metrics, version, 
      instance, args, endpoint=gen_ai_endpoint, **kwargs
  )
  
  # ❌ Wrong: Don't align with opening parenthesis
  def general_wrap(gen_ai_endpoint, version, environment, application_name,
                   tracer, pricing_info, capture_message_content, metrics, disable_metrics):
  ```

### 3. Comments
- **Triple double quotes** for docstrings:
  ```python
  def function_name():
      """
      Function description.
      """
  ```
- **Standard inline comments** for specific actions:
  ```python
  # Get server address and port using the standard helper
  server_address, server_port = set_server_address_and_port(instance)
  
  # Process response and generate telemetry
  response = process_vectordb_response(...)
  ```

### 4. Import Order
Follow this specific order:
```python
# Standard library imports
import time

# OpenTelemetry imports
from opentelemetry.trace import Status, StatusCode, SpanKind
from opentelemetry import context as context_api

# OpenLIT imports
from openlit.__helpers import (
    common_db_span_attributes,
    record_db_metrics,
    handle_exception,
    set_server_address_and_port,
)
from openlit.semcov import SemanticConvention
```

### 5. Function Parameters
Standard parameter order:
```python
def function_name(gen_ai_endpoint, version, environment, application_name, tracer, 
                 pricing_info, capture_message_content, metrics, disable_metrics):
```

## Implementation Steps

### Step 1: Create Directory Structure
```bash
mkdir -p sdk/python/src/openlit/instrumentation/{database_name}
```

### Step 2: Create __init__.py (Compact Pattern)

**For databases with many operations, use the compact data structure approach:**

```python
"""
OpenLIT {DatabaseName} Instrumentation
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.{database_name}.{database_name} import general_wrap
# Only import async if the database supports it
# from openlit.instrumentation.{database_name}.async_{database_name} import async_general_wrap

_instruments = ("{database-package} >= {version}",)

# Operations to wrap for both sync and async clients
{DATABASE_NAME}_OPERATIONS = [
    ("method_name", "{database}.endpoint"),
    ("create_collection", "{database}.create_collection"),
    ("upsert", "{database}.upsert"),
    ("query", "{database}.query"),
    ("delete", "{database}.delete"),
    # ... add all operations here
]

class {DatabaseName}Instrumentor(BaseInstrumentor):
    """
    An instrumentor for {DatabaseName}'s client library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        version = importlib.metadata.version("{database-package}")
        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics")

        # Wrap sync operations
        for method_name, endpoint in {DATABASE_NAME}_OPERATIONS:
            wrap_function_wrapper(
                "{database_module}",
                f"{ClientClass}.{method_name}",
                general_wrap(
                    endpoint, version, environment, application_name, tracer,
                    pricing_info, capture_message_content, metrics, disable_metrics
                ),
            )

        # Wrap async operations (only if database supports them)
        # for method_name, endpoint in {DATABASE_NAME}_OPERATIONS:
        #     wrap_function_wrapper(
        #         "{database_module}",
        #         f"{AsyncClientClass}.{method_name}",
        #         async_general_wrap(
        #             endpoint, version, environment, application_name, tracer,
        #             pricing_info, capture_message_content, metrics, disable_metrics
        #         ),
        #     )

    def _uninstrument(self, **kwargs):
        pass
```

**For databases with few operations, use the direct approach:**

```python
# Direct wrapping for databases with < 10 operations
wrap_function_wrapper(
    "{database_module}",
    "{Class}.{method}",
    general_wrap(
        "{database}.{operation}",
        version, environment, application_name, tracer,
        pricing_info, capture_message_content, metrics, disable_metrics
    ),
)
```

### Step 3: Create Wrapper Files
Create sync (and async if needed) wrapper files following the general_wrap pattern.

### Step 4: Create utils.py
Include functions in **standard order**:
1. `DB_OPERATION_MAP` with proper semantic operation mapping
2. `object_count` helper function
3. `set_server_address_and_port` function for server address extraction
4. `common_vectordb_logic` with endpoint differentiation
5. `process_vectordb_response` function

**Server Address Extraction Function:**
```python
def set_server_address_and_port(instance):
    """
    Extracts server address and port from {database} client instance.
    
    Args:
        instance: {Database} client instance
        
    Returns:
        tuple: (server_address, server_port)
    """
    server_address = "default-host"
    server_port = default_port
    
    # Database-specific extraction logic
    # For Qdrant: instance.init_options
    # For others: instance._client.base_url, instance.config.host, etc.
    
    return server_address, server_port
```

**Usage in wrapper files:**
```python
# Replace generic helper with database-specific function
from openlit.instrumentation.{database}.utils import set_server_address_and_port

# In wrapper function
server_address, server_port = set_server_address_and_port(instance)
```

### Step 5: Database-Specific Adaptations

**Server Address Patterns:**
- Pinecone: `"pinecone.io", 443`
- ChromaDB: `"localhost", 8000`
- Adjust for your database's default

**Collection vs Namespace:**
- Collection-based (ChromaDB): Use `DB_COLLECTION_NAME`
- Namespace-based (Pinecone): Use `DB_NAMESPACE`

**Response Structure:**
- Flat arrays: `object_count(response['ids'])`
- Nested arrays: `object_count(response['ids'][0])`

## Testing

### Create Test File
Create `sdk/python/tests/test_{database_name}.py`:

```python
"""
Tests for {DatabaseName} functionality.
"""

import pytest
import openlit
from {database_package} import {ClientClass}

# Initialize client
client = {ClientClass}()

# Initialize OpenLIT
openlit.init(environment="openlit-python-testing", application_name="openlit-python-{database}-test")

def test_sync_{database}_operation():
    """
    Tests synchronous operation.
    """
    try:
        response = client.operation(
            # parameters
        )
        assert response  # Add appropriate assertions
        
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate limit exceeded:", e)
        else:
            raise

# Add async tests only if database supports async
# @pytest.mark.asyncio
# async def test_async_{database}_operation():
#     """
#     Tests asynchronous operation.
#     """
#     pass
```

## Integration

### Step 1: Add to Base Instrumentation
Add to `sdk/python/src/openlit/instrumentation/__init__.py`:

```python
from openlit.instrumentation.{database_name} import {DatabaseName}Instrumentor

def auto_instrument(**kwargs):
    """
    Auto-instrument supported libraries
    """
    # ... existing instrumentors ...
    
    # {DatabaseName} Instrumentation
    try:
        {DatabaseName}Instrumentor().instrument(**kwargs)
    except Exception as e:
        logger.warning(f"Failed to instrument {DatabaseName}: {e}")
```

### Step 2: Add Semantic Conventions
Add any new semantic conventions to `sdk/python/src/openlit/semcov/__init__.py`.

## Consistency Requirements

**CRITICAL**: All vector database instrumentations must follow **identical patterns** for maintainability and developer experience:

### **Code Style Consistency:**
- **Quotes**: Always use double quotes (`"`) - never single quotes (`'`)
- **Comments**: Use standardized inline comments (see Code Style Guidelines)
- **Function Order**: Follow the exact order specified in utils.py structure
- **Import Order**: Standard library → OpenTelemetry → OpenLIT imports
- **Variable Names**: Use `snake_case` consistently

### **Structural Consistency:**
- **Exception Handling**: `wrapped()` call outside try block, telemetry processing inside
- **Server Address**: Use `set_server_address_and_port(instance)` function signature
- **Span Creation**: Always use `tracer.start_as_current_span()` - never `tracer.start_span()`
- **Suppression Check**: Always include `context_api._SUPPRESS_INSTRUMENTATION_KEY` check

## Key Reminders

1. **Always use `tracer.start_as_current_span()`** - Never use `tracer.start_span()`
2. **Include suppression check** - Always check `context_api._SUPPRESS_INSTRUMENTATION_KEY`
3. **Use simplified patterns** - Focus on essential telemetry attributes
4. **Handle endpoint differentiation** - Use `endpoint` parameter for operations mapping to same semantic type
5. **Semantic operation mapping** - Multiple endpoints can map to same DB operation
6. **Response structure awareness** - Handle flat vs nested response arrays correctly
7. **Collection vs namespace** - Use appropriate semantic convention for your database type
8. **Default values** - Use `"None"` for missing values in query summaries (double quotes)
9. **Test thoroughly** - Include both sync and async tests (if applicable)

## Example Implementation

For complete reference implementations, see:

### **Qdrant** (Optimized Pattern):
- **Structure**: 4-file structure with async support
- **Lines**: 80 (__init__.py), 55 (sync), 55 (async), 328 (utils.py)
- **Features**: Compact __init__.py with data structure, `set_server_address_and_port` function
- **Pattern**: Collection-based, endpoint differentiation, comprehensive operation coverage

### **Pinecone** (Optimized Pattern):
- **Structure**: 4-file structure with async support  
- **Lines**: 78 (__init__.py), 56 (sync), 56 (async), 227 (utils.py)
- **Features**: Compact __init__.py with data structure, `set_server_address_and_port` function
- **Pattern**: Namespace-based, comprehensive telemetry

### **ChromaDB** (Simplified Pattern):
- **Structure**: 3-file structure (no async support)
- **Lines**: 90 (__init__.py), 57 (sync), 273 (utils.py)
- **Features**: Collection-based, endpoint differentiation, `set_server_address_and_port` function
- **Pattern**: Direct wrapping approach for fewer operations

### **Recommended Approach**:
1. **< 10 operations**: Use ChromaDB pattern (direct wrapping)
2. **10-20 operations**: Use Pinecone pattern (traditional __init__.py)
3. **20+ operations**: Use Qdrant pattern (compact data structure)

This guide ensures consistency across all vector database instrumentations while accommodating database-specific requirements and maintaining high quality standards. 