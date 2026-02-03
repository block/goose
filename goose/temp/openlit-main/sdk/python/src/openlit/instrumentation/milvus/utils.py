"""
Utility functions for Milvus instrumentation.
"""

import time
from urllib.parse import urlparse
from opentelemetry.trace import Status, StatusCode
from openlit.__helpers import (
    common_db_span_attributes,
    record_db_metrics,
)
from openlit.semcov import SemanticConvention

# Operation mapping for simple span naming
DB_OPERATION_MAP = {
    "milvus.create_collection": SemanticConvention.DB_OPERATION_CREATE_COLLECTION,
    "milvus.drop_collection": SemanticConvention.DB_OPERATION_DELETE_COLLECTION,
    "milvus.insert": SemanticConvention.DB_OPERATION_INSERT,
    "milvus.upsert": SemanticConvention.DB_OPERATION_UPSERT,
    "milvus.search": SemanticConvention.DB_OPERATION_SEARCH,
    "milvus.query": SemanticConvention.DB_OPERATION_QUERY,
    "milvus.get": SemanticConvention.DB_OPERATION_GET,
    "milvus.delete": SemanticConvention.DB_OPERATION_DELETE,
}


def object_count(obj):
    """
    Counts length of object if it exists, else returns 0.
    """
    return len(obj) if obj else 0


def set_server_address_and_port(instance):
    """
    Extracts server address and port from Milvus client instance.

    Args:
        instance: Milvus client instance

    Returns:
        tuple: (server_address, server_port)
    """
    server_address = "localhost"
    server_port = 19530  # Default Milvus port

    # Try getting uri from multiple potential attributes
    client_config = getattr(instance, "_client_config", None)
    if client_config:
        uri = getattr(client_config, "uri", None)
        if uri and isinstance(uri, str):
            if uri.startswith(("http://", "https://")):
                try:
                    parsed = urlparse(uri)
                    server_address = parsed.hostname or server_address
                    server_port = parsed.port or server_port
                except Exception:
                    pass
            else:
                # Handle cases like "localhost:19530" or just "localhost"
                if ":" in uri:
                    parts = uri.split(":")
                    server_address = parts[0]
                    try:
                        server_port = int(parts[1])
                    except (ValueError, IndexError):
                        pass
                else:
                    server_address = uri

    return server_address, server_port


def common_milvus_logic(
    scope,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    instance=None,
    endpoint=None,
):
    """
    Process Milvus database request and generate telemetry.

    Args:
        scope: Scope object containing span, response, and operation details
        environment: Deployment environment
        application_name: Name of the application
        metrics: Metrics dictionary for recording telemetry
        capture_message_content: Flag to capture message content
        disable_metrics: Flag to disable metrics collection
        version: Version of the Milvus client
        instance: Milvus client instance
        endpoint: Operation endpoint for differentiation
    """
    scope._end_time = time.time()

    # Set common database span attributes using helper
    common_db_span_attributes(
        scope,
        SemanticConvention.DB_SYSTEM_MILVUS,
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

    if scope._db_operation == SemanticConvention.DB_OPERATION_CREATE_COLLECTION:
        collection_name = scope._kwargs.get("collection_name", "unknown")

        # Standard database attributes
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_DIMENSION,
            scope._kwargs.get("dimension", -1),
        )

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"dimension={scope._kwargs.get('dimension', 'None')}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_DELETE_COLLECTION:
        collection_name = scope._kwargs.get("collection_name", "unknown")

        # Standard database attributes
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_INSERT:
        collection_name = scope._kwargs.get("collection_name", "unknown")
        data = scope._kwargs.get("data", [])

        # Standard database attributes
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )
        scope._span.set_attribute(
            SemanticConvention.DB_VECTOR_COUNT, object_count(data)
        )
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(data))

        # Response metrics
        if scope._response and scope._response.get("insert_count"):
            scope._span.set_attribute(
                SemanticConvention.DB_RESPONSE_RETURNED_ROWS,
                scope._response["insert_count"],
            )

        if scope._response and scope._response.get("cost"):
            scope._span.set_attribute(
                SemanticConvention.DB_OPERATION_COST, scope._response["cost"]
            )

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} data_count={object_count(data)}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_UPSERT:
        collection_name = scope._kwargs.get("collection_name", "unknown")
        data = scope._kwargs.get("data", [])

        # Standard database attributes
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )
        scope._span.set_attribute(
            SemanticConvention.DB_VECTOR_COUNT, object_count(data)
        )
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(data))

        # Response metrics
        if scope._response and scope._response.get("upsert_count"):
            scope._span.set_attribute(
                SemanticConvention.DB_RESPONSE_RETURNED_ROWS,
                scope._response["upsert_count"],
            )

        if scope._response and scope._response.get("cost"):
            scope._span.set_attribute(
                SemanticConvention.DB_OPERATION_COST, scope._response["cost"]
            )

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} data_count={object_count(data)}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_SEARCH:
        collection_name = scope._kwargs.get("collection_name", "unknown")
        data = scope._kwargs.get("data", [])

        # Standard database attributes
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(data))
        scope._span.set_attribute(
            SemanticConvention.DB_VECTOR_QUERY_TOP_K, scope._kwargs.get("limit", -1)
        )

        # Search specific attributes
        scope._span.set_attribute(
            SemanticConvention.DB_FILTER, str(scope._kwargs.get("filter", ""))
        )

        # Response metrics
        if scope._response and isinstance(scope._response, list):
            scope._span.set_attribute(
                SemanticConvention.DB_RESPONSE_RETURNED_ROWS, len(scope._response)
            )

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"data={str(data)[:100]}... "
            f"limit={scope._kwargs.get('limit', 'None')}",
        )

    elif scope._db_operation in [
        SemanticConvention.DB_OPERATION_QUERY,
        SemanticConvention.DB_OPERATION_GET,
    ]:
        collection_name = scope._kwargs.get("collection_name", "unknown")
        output_fields = scope._kwargs.get("output_fields", [])

        # Standard database attributes
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(output_fields))

        # Query specific attributes
        scope._span.set_attribute(
            SemanticConvention.DB_FILTER, str(scope._kwargs.get("filter", ""))
        )

        # Response metrics
        if scope._response and isinstance(scope._response, list):
            scope._span.set_attribute(
                SemanticConvention.DB_RESPONSE_RETURNED_ROWS, len(scope._response)
            )

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"output_fields={output_fields} "
            f"filter={scope._kwargs.get('filter', 'None')}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_DELETE:
        collection_name = scope._kwargs.get("collection_name", "unknown")

        # Standard database attributes
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )
        scope._span.set_attribute(
            SemanticConvention.DB_FILTER, str(scope._kwargs.get("filter", ""))
        )

        # Response metrics
        if scope._response and scope._response.get("delete_count"):
            scope._span.set_attribute(
                SemanticConvention.DB_RESPONSE_RETURNED_ROWS,
                scope._response["delete_count"],
            )

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"filter={scope._kwargs.get('filter', 'None')}",
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Record metrics using helper
    if not disable_metrics:
        record_db_metrics(
            metrics,
            SemanticConvention.DB_SYSTEM_MILVUS,
            scope._server_address,
            scope._server_port,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            scope._db_operation,
        )


def process_milvus_response(
    response,
    db_operation,
    server_address,
    server_port,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    capture_message_content,
    disable_metrics,
    version,
    instance,
    args,
    **kwargs,
):
    """
    Process Milvus response and generate telemetry.

    Args:
        response: Response from Milvus operation
        db_operation: Database operation type
        server_address: Server address
        server_port: Server port
        environment: Deployment environment
        application_name: Application name
        metrics: Metrics dictionary
        start_time: Start time of the operation
        span: OpenTelemetry span
        capture_message_content: Flag to capture message content
        disable_metrics: Flag to disable metrics
        version: Milvus client version
        instance: Milvus client instance
        args: Positional arguments
        **kwargs: Keyword arguments

    Returns:
        Original response
    """

    # Create a scope object to hold all the context
    scope = type("GenericScope", (), {})()
    scope._response = response
    scope._db_operation = db_operation
    scope._server_address = server_address
    scope._server_port = server_port
    scope._start_time = start_time
    scope._span = span
    scope._kwargs = kwargs

    # Process the response using common logic
    common_milvus_logic(
        scope,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        instance,
    )

    return response
