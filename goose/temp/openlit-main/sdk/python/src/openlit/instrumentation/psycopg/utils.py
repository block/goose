"""
Utilities for Psycopg (PostgreSQL) instrumentation.
"""

import re
import time
from typing import Tuple, Any, Optional
from opentelemetry.trace import Status, StatusCode, get_current_span
from openlit.__helpers import (
    common_db_span_attributes,
    record_db_metrics,
)
from openlit.semcov import SemanticConvention

# Maximum parameter value length to capture (to avoid huge traces)
MAX_PARAM_LENGTH = 256
# Maximum number of parameters to capture
MAX_PARAMS_COUNT = 50

# Operation mapping for SQL statements
SQL_OPERATION_MAP = {
    "SELECT": SemanticConvention.DB_OPERATION_SELECT,
    "INSERT": SemanticConvention.DB_OPERATION_INSERT,
    "UPDATE": SemanticConvention.DB_OPERATION_UPDATE,
    "DELETE": SemanticConvention.DB_OPERATION_DELETE,
    "COPY": SemanticConvention.DB_OPERATION_COPY,
    "CREATE": SemanticConvention.DB_OPERATION_CREATE,
    "ALTER": SemanticConvention.DB_OPERATION_ALTER,
    "DROP": SemanticConvention.DB_OPERATION_DROP,
    "TRUNCATE": SemanticConvention.DB_OPERATION_TRUNCATE,
    "COMMIT": SemanticConvention.DB_OPERATION_COMMIT,
    "ROLLBACK": SemanticConvention.DB_OPERATION_ROLLBACK,
}

# Patterns for extracting table names from SQL queries
TABLE_PATTERNS = [
    r'INSERT\s+INTO\s+["\']?(\w+)["\']?',
    r'UPDATE\s+["\']?(\w+)["\']?',
    r'DELETE\s+FROM\s+["\']?(\w+)["\']?',
    r'FROM\s+["\']?(\w+)["\']?',
    r'TRUNCATE\s+(?:TABLE\s+)?["\']?(\w+)["\']?',
    r'CREATE\s+(?:TABLE|INDEX)\s+(?:IF\s+NOT\s+EXISTS\s+)?["\']?(\w+)["\']?',
    r'ALTER\s+TABLE\s+["\']?(\w+)["\']?',
    r'DROP\s+(?:TABLE|INDEX)\s+(?:IF\s+EXISTS\s+)?["\']?(\w+)["\']?',
    r'COPY\s+["\']?(\w+)["\']?',
]


def parse_sql_operation(query: Any) -> str:
    """
    Parse SQL statement to detect operation type.

    Args:
        query: SQL query string or SQL composition object

    Returns:
        Operation type constant from SemanticConvention
    """
    if query is None:
        return SemanticConvention.DB_OPERATION_QUERY

    # Handle psycopg sql.SQL and sql.Composed objects
    query_str = str(query).strip()
    if not query_str:
        return SemanticConvention.DB_OPERATION_QUERY

    # Get the first word (SQL command)
    query_upper = query_str.upper()

    # Check for common SQL operations
    for keyword, operation in SQL_OPERATION_MAP.items():
        if query_upper.startswith(keyword):
            return operation

    # Handle WITH ... SELECT/INSERT/UPDATE/DELETE (CTEs)
    if query_upper.startswith("WITH"):
        # Look for the actual operation after the CTE
        for keyword, operation in SQL_OPERATION_MAP.items():
            if keyword in query_upper:
                return operation

    return SemanticConvention.DB_OPERATION_QUERY


def extract_table_name(query: Any) -> str:
    """
    Extract table name from SQL query for span naming.

    Args:
        query: SQL query string or SQL composition object

    Returns:
        Table name or "unknown" if not found
    """
    if query is None:
        return "unknown"

    query_str = str(query)

    for pattern in TABLE_PATTERNS:
        match = re.search(pattern, query_str, re.IGNORECASE)
        if match:
            return match.group(1)

    return "unknown"


def extract_connection_info(connection: Any) -> Tuple[str, int]:
    """
    Extract server address and port from psycopg connection.

    Args:
        connection: Psycopg Connection or AsyncConnection object

    Returns:
        Tuple of (server_address, server_port)
    """
    default_address = "localhost"
    default_port = 5432

    try:
        if hasattr(connection, "info"):
            info = connection.info
            host = getattr(info, "host", None) or default_address
            port = getattr(info, "port", None) or default_port
            return host, int(port)
    except Exception:
        pass

    return default_address, default_port


def extract_database_name(connection: Any) -> str:
    """
    Extract database name from psycopg connection.

    Args:
        connection: Psycopg Connection or AsyncConnection object

    Returns:
        Database name or "unknown"
    """
    try:
        if hasattr(connection, "info"):
            return getattr(connection.info, "dbname", None) or "unknown"
    except Exception:
        pass

    return "unknown"


def detect_special_features(query: Any) -> dict:
    """
    Detect special PostgreSQL features in the query.

    Args:
        query: SQL query string

    Returns:
        Dict with detected features
    """
    features = {}
    if query is None:
        return features

    query_str = str(query).upper()

    # pgvector similarity operators
    if "<=>" in str(query):
        features["similarity_metric"] = "cosine"
    elif "<->" in str(query):
        features["similarity_metric"] = "l2"
    elif "<#>" in str(query):
        features["similarity_metric"] = "inner_product"

    # Full-text search
    if "TSVECTOR" in query_str or "TSQUERY" in query_str:
        features["full_text_search"] = True
    if "WEBSEARCH_TO_TSQUERY" in query_str:
        features["websearch"] = True
    if "TS_RANK" in query_str:
        features["text_ranking"] = True

    return features


def get_query_summary(db_operation: str, table_name: str, query: Any) -> str:
    """
    Generate a human-readable query summary.

    Args:
        db_operation: The detected database operation
        table_name: The table name
        query: The SQL query

    Returns:
        Human-readable summary string
    """
    summary = f"{db_operation} {table_name}"

    features = detect_special_features(query)
    if features.get("similarity_metric"):
        summary += f" (vector {features['similarity_metric']})"
    if features.get("full_text_search"):
        summary += " (full-text)"

    return summary


def sanitize_parameter(value: Any, max_length: int = MAX_PARAM_LENGTH) -> str:
    """
    Sanitize a parameter value for safe inclusion in traces.

    Truncates long values and converts to string representation.
    Does NOT mask sensitive data - that's the user's responsibility
    to not enable capture_parameters with sensitive queries.

    Args:
        value: Parameter value
        max_length: Maximum string length

    Returns:
        Sanitized string representation
    """
    if value is None:
        return "NULL"

    # Handle bytes (don't capture binary data)
    if isinstance(value, (bytes, bytearray)):
        return f"<bytes:{len(value)}>"

    # Handle memoryview
    if isinstance(value, memoryview):
        return f"<memoryview:{len(value)}>"

    # Convert to string
    try:
        str_value = str(value)
    except Exception:
        return "<unrepresentable>"

    # Truncate if too long
    if len(str_value) > max_length:
        return str_value[:max_length] + f"...<truncated:{len(str_value)}>"

    return str_value


def format_parameters(
    params: Any,
    max_count: int = MAX_PARAMS_COUNT,
    max_length: int = MAX_PARAM_LENGTH,
) -> Optional[str]:
    """
    Format query parameters for span attributes.

    Args:
        params: Query parameters (tuple, list, dict, or None)
        max_count: Maximum number of parameters to include
        max_length: Maximum length per parameter value

    Returns:
        Formatted string or None if no parameters
    """
    if params is None:
        return None

    try:
        if isinstance(params, dict):
            # Named parameters: {"name": "value", ...}
            items = list(params.items())[:max_count]
            formatted = {k: sanitize_parameter(v, max_length) for k, v in items}
            if len(params) > max_count:
                formatted["..."] = f"<{len(params) - max_count} more>"
            return str(formatted)

        elif isinstance(params, (list, tuple)):
            # Positional parameters: ($1, $2, ...)
            items = list(params)[:max_count]
            formatted = [sanitize_parameter(v, max_length) for v in items]
            if len(params) > max_count:
                formatted.append(f"<{len(params) - max_count} more>")
            return str(formatted)

        else:
            # Single parameter
            return sanitize_parameter(params, max_length)

    except Exception:
        return "<error formatting parameters>"


def generate_sql_comment(
    traceparent: Optional[str] = None,
    tracestate: Optional[str] = None,
    application_name: Optional[str] = None,
) -> str:
    """
    Generate a SQL comment with trace context (SQLCommenter format).

    This allows database logs to be correlated with application traces.
    Format follows the SQLCommenter specification:
    https://google.github.io/sqlcommenter/

    Args:
        traceparent: W3C Trace Context traceparent header value
        tracestate: W3C Trace Context tracestate header value
        application_name: Application name to include

    Returns:
        SQL comment string to append to queries
    """
    parts = []

    if traceparent:
        # URL-encode single quotes for safety
        parts.append(f"traceparent='{traceparent}'")

    if tracestate:
        parts.append(f"tracestate='{tracestate}'")

    if application_name:
        # Sanitize application name (alphanumeric and underscores only)
        safe_name = re.sub(r"[^a-zA-Z0-9_]", "_", application_name)
        parts.append(f"application='{safe_name}'")

    if not parts:
        return ""

    return "/*" + ",".join(parts) + "*/"


def get_trace_context_for_comment() -> Tuple[Optional[str], Optional[str]]:
    """
    Extract current trace context for SQLCommenter.

    Returns:
        Tuple of (traceparent, tracestate) or (None, None) if no active span
    """
    try:
        span = get_current_span()
        if span is None or not span.is_recording():
            return None, None

        ctx = span.get_span_context()
        if ctx is None or not ctx.is_valid:
            return None, None

        # Format traceparent: version-trace_id-span_id-flags
        # Version is always 00
        trace_id = format(ctx.trace_id, "032x")
        span_id = format(ctx.span_id, "016x")
        flags = format(ctx.trace_flags, "02x")
        traceparent = f"00-{trace_id}-{span_id}-{flags}"

        # Get tracestate if available
        tracestate = None
        if ctx.trace_state:
            # Convert trace state to string
            tracestate_items = []
            for key, value in ctx.trace_state.items():
                tracestate_items.append(f"{key}={value}")
            if tracestate_items:
                tracestate = ",".join(tracestate_items)

        return traceparent, tracestate

    except Exception:
        return None, None


def inject_sql_comment(
    query: Any,
    application_name: Optional[str] = None,
    enable_sqlcommenter: bool = False,
) -> Any:
    """
    Inject SQLCommenter trace context into a SQL query.

    Only modifies string queries. SQL composition objects are returned as-is.
    Only injects if enable_sqlcommenter is True.

    Args:
        query: Original SQL query
        application_name: Application name to include in comment
        enable_sqlcommenter: Whether SQLCommenter is enabled

    Returns:
        Modified query with comment appended, or original query
    """
    if not enable_sqlcommenter:
        return query

    # Only inject into string queries
    if not isinstance(query, str):
        return query

    # Don't inject if query already has a comment at the end
    stripped = query.rstrip()
    if stripped.endswith("*/"):
        return query

    # Get current trace context
    traceparent, tracestate = get_trace_context_for_comment()

    # Generate comment
    comment = generate_sql_comment(traceparent, tracestate, application_name)

    if not comment:
        return query

    # Append comment to query
    # Handle queries that end with semicolon
    if stripped.endswith(";"):
        return stripped[:-1] + " " + comment + ";"
    else:
        return query + " " + comment


def common_psycopg_logic(
    scope,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    connection=None,
    endpoint=None,
    capture_parameters=False,
    params=None,
):
    """
    Process psycopg request and generate telemetry.

    Args:
        scope: Scope object with span and operation data
        environment: Deployment environment
        application_name: Application name
        metrics: Metrics dictionary
        capture_message_content: Whether to capture query text
        disable_metrics: Whether metrics are disabled
        version: SDK version
        connection: Database connection object
        endpoint: API endpoint name
        capture_parameters: Whether to capture query parameters
        params: Query parameters (if capture_parameters is True)
    """
    scope._end_time = time.time()

    # Set common database span attributes using helper
    common_db_span_attributes(
        scope,
        SemanticConvention.DB_SYSTEM_POSTGRESQL,
        scope._server_address,
        scope._server_port,
        environment,
        application_name,
        version,
    )

    # Set DB operation specific attributes
    scope._span.set_attribute(SemanticConvention.DB_OPERATION_NAME, scope._db_operation)
    scope._span.set_attribute(
        SemanticConvention.DB_CLIENT_OPERATION_DURATION,
        scope._end_time - scope._start_time,
    )

    # Set database namespace (database name)
    if scope._database_name and scope._database_name != "unknown":
        scope._span.set_attribute(SemanticConvention.DB_NAMESPACE, scope._database_name)

    # Set table/collection name
    if scope._table_name and scope._table_name != "unknown":
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, scope._table_name
        )

    # Set query text if capture is enabled
    if capture_message_content and scope._query:
        query_str = str(scope._query)
        # Truncate very long queries
        if len(query_str) > 4096:
            query_str = query_str[:4096] + "..."
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, query_str)

    # Set query parameters if capture is enabled
    if capture_parameters and params is not None:
        formatted_params = format_parameters(params)
        if formatted_params:
            scope._span.set_attribute(
                SemanticConvention.DB_QUERY_PARAMETER, formatted_params
            )

    # Set query summary
    scope._span.set_attribute(
        SemanticConvention.DB_QUERY_SUMMARY,
        get_query_summary(scope._db_operation, scope._table_name, scope._query),
    )

    # Set row count if available
    if (
        hasattr(scope, "_rowcount")
        and scope._rowcount is not None
        and scope._rowcount >= 0
    ):
        scope._span.set_attribute(
            SemanticConvention.DB_RESPONSE_RETURNED_ROWS, scope._rowcount
        )

    # Set special features
    features = detect_special_features(scope._query)
    if features.get("similarity_metric"):
        scope._span.set_attribute(
            SemanticConvention.DB_SEARCH_SIMILARITY_METRIC,
            features["similarity_metric"],
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Record metrics using helper
    if not disable_metrics:
        record_db_metrics(
            metrics,
            SemanticConvention.DB_SYSTEM_POSTGRESQL,
            scope._server_address,
            scope._server_port,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            scope._db_operation,
        )


def process_cursor_response(
    response,
    db_operation,
    table_name,
    query,
    server_address,
    server_port,
    database_name,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    capture_message_content,
    disable_metrics,
    version,
    cursor=None,
    connection=None,
    endpoint=None,
    capture_parameters=False,
    params=None,
):
    """
    Process psycopg cursor response and generate telemetry.

    Args:
        response: The cursor response
        db_operation: Database operation type
        table_name: Table name
        query: SQL query
        server_address: PostgreSQL server address
        server_port: PostgreSQL server port
        database_name: Database name
        environment: Deployment environment
        application_name: Application name
        metrics: Metrics dictionary
        start_time: Operation start time
        span: OpenTelemetry span
        capture_message_content: Whether to capture query text
        disable_metrics: Whether metrics are disabled
        version: SDK version
        cursor: Cursor object
        connection: Connection object
        endpoint: API endpoint name
        capture_parameters: Whether to capture query parameters
        params: Query parameters (tuple, list, dict, or None)
    """
    # Create scope object
    scope = type("GenericScope", (), {})()
    scope._span = span
    scope._response = response
    scope._db_operation = db_operation
    scope._table_name = table_name
    scope._query = query
    scope._server_address = server_address
    scope._server_port = server_port
    scope._database_name = database_name
    scope._start_time = start_time

    # Try to get rowcount from cursor
    scope._rowcount = None
    if cursor is not None:
        try:
            scope._rowcount = cursor.rowcount
        except Exception:
            pass

    # Process the response
    common_psycopg_logic(
        scope,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        connection=connection,
        endpoint=endpoint,
        capture_parameters=capture_parameters,
        params=params,
    )

    return response


def process_connection_response(
    response,
    db_operation,
    server_address,
    server_port,
    database_name,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    capture_message_content,
    disable_metrics,
    version,
    connection=None,
    endpoint=None,
):
    """
    Process psycopg connection response (commit/rollback) and generate telemetry.

    Args:
        response: The response
        db_operation: Database operation type (COMMIT/ROLLBACK)
        server_address: PostgreSQL server address
        server_port: PostgreSQL server port
        database_name: Database name
        environment: Deployment environment
        application_name: Application name
        metrics: Metrics dictionary
        start_time: Operation start time
        span: OpenTelemetry span
        capture_message_content: Whether to capture query text
        disable_metrics: Whether metrics are disabled
        version: SDK version
        connection: Connection object
        endpoint: API endpoint name
    """
    # Create scope object
    scope = type("GenericScope", (), {})()
    scope._span = span
    scope._response = response
    scope._db_operation = db_operation
    scope._table_name = "transaction"
    scope._query = db_operation
    scope._server_address = server_address
    scope._server_port = server_port
    scope._database_name = database_name
    scope._start_time = start_time
    scope._rowcount = None

    # Process the response
    common_psycopg_logic(
        scope,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        connection=connection,
        endpoint=endpoint,
    )

    return response
