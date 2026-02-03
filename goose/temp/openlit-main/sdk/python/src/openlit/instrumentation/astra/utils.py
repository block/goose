"""
Utility functions for AstraDB instrumentation.
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
    "astra.create_collection": SemanticConvention.DB_OPERATION_CREATE_COLLECTION,
    "astra.drop_collection": SemanticConvention.DB_OPERATION_DELETE_COLLECTION,
    "astra.insert": SemanticConvention.DB_OPERATION_INSERT,
    "astra.insert_one": SemanticConvention.DB_OPERATION_INSERT,
    "astra.insert_many": SemanticConvention.DB_OPERATION_INSERT,
    "astra.update": SemanticConvention.DB_OPERATION_UPDATE,
    "astra.update_one": SemanticConvention.DB_OPERATION_UPDATE,
    "astra.update_many": SemanticConvention.DB_OPERATION_UPDATE,
    "astra.find": SemanticConvention.DB_OPERATION_SELECT,
    "astra.find_one_and_update": SemanticConvention.DB_OPERATION_REPLACE,
    "astra.replace_one": SemanticConvention.DB_OPERATION_REPLACE,
    "astra.find_one_and_delete": SemanticConvention.DB_OPERATION_FIND_AND_DELETE,
    "astra.delete": SemanticConvention.DB_OPERATION_DELETE,
    "astra.delete_one": SemanticConvention.DB_OPERATION_DELETE,
    "astra.delete_many": SemanticConvention.DB_OPERATION_DELETE,
}


def object_count(obj):
    """
    Counts length of object if it exists, else returns 0.
    """
    if isinstance(obj, list):
        return len(obj)
    elif obj is not None:
        return 1
    return 0


def set_server_address_and_port(instance):
    """
    Extracts server address and port from AstraDB client instance.

    Args:
        instance: AstraDB client instance

    Returns:
        tuple: (server_address, server_port)
    """
    server_address = "astra.datastax.com"
    server_port = 443

    # Try getting api_endpoint from instance or its database
    api_endpoint = getattr(instance, "api_endpoint", None)
    if not api_endpoint:
        # Try getting from database attribute
        database = getattr(instance, "database", None)
        if database:
            api_endpoint = getattr(database, "api_endpoint", None)

    if api_endpoint and isinstance(api_endpoint, str):
        if api_endpoint.startswith(("http://", "https://")):
            try:
                parsed = urlparse(api_endpoint)
                server_address = parsed.hostname or server_address
                server_port = parsed.port or server_port
            except Exception:
                pass
        else:
            # Handle cases like "hostname:port" or just "hostname"
            if ":" in api_endpoint:
                parts = api_endpoint.split(":")
                server_address = parts[0]
                try:
                    server_port = int(parts[1])
                except (ValueError, IndexError):
                    pass
            else:
                server_address = api_endpoint

    return server_address, server_port


def common_astra_logic(
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
    Process AstraDB request and generate telemetry.

    Args:
        scope: Scope object containing span, response, and operation details
        environment: Deployment environment
        application_name: Name of the application
        metrics: Metrics dictionary for recording telemetry
        capture_message_content: Flag to capture message content
        disable_metrics: Flag to disable metrics collection
        version: Version of the AstraDB client
        instance: AstraDB client instance
        endpoint: Operation endpoint for differentiation
    """
    scope._end_time = time.time()

    # Set common database span attributes using helper
    common_db_span_attributes(
        scope,
        SemanticConvention.DB_SYSTEM_ASTRA,
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

    # Get collection name from instance
    collection_name = getattr(instance, "name", "unknown")
    scope._span.set_attribute(SemanticConvention.DB_COLLECTION_NAME, collection_name)

    if scope._db_operation == SemanticConvention.DB_OPERATION_CREATE_COLLECTION:
        # Handle create_collection operation
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_DIMENSION,
            scope._kwargs.get("dimension", -1),
        )
        scope._span.set_attribute(
            SemanticConvention.DB_INDEX_METRIC, str(scope._kwargs.get("metric", ""))
        )

        # Set namespace if available in response
        if scope._response and hasattr(scope._response, "keyspace"):
            scope._span.set_attribute(
                SemanticConvention.DB_NAMESPACE, scope._response.keyspace
            )

        if scope._response and hasattr(scope._response, "name"):
            scope._span.set_attribute(
                SemanticConvention.DB_COLLECTION_NAME, scope._response.name
            )

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"dimension={scope._kwargs.get('dimension', 'None')} "
            f"metric={scope._kwargs.get('metric', 'None')}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_DELETE_COLLECTION:
        # Handle drop_collection operation
        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_INSERT:
        # Handle insert operations (insert_one, insert_many, regular insert)
        documents = (
            scope._args[0] if scope._args else scope._kwargs.get("documents", [])
        )

        scope._span.set_attribute(
            SemanticConvention.DB_DOCUMENTS_COUNT, object_count(documents)
        )
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(documents))

        # Response metrics
        if scope._response and hasattr(scope._response, "inserted_ids"):
            scope._span.set_attribute(
                SemanticConvention.DB_RESPONSE_RETURNED_ROWS,
                len(scope._response.inserted_ids),
            )

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"documents_count={object_count(documents)}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_UPDATE:
        # Handle update operations (update_one, update_many, regular update)
        update_query = (
            scope._args[1] if len(scope._args) > 1 else scope._kwargs.get("update", {})
        )
        filter_query = (
            scope._args[0] if scope._args else scope._kwargs.get("filter", {})
        )

        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(update_query))
        scope._span.set_attribute(SemanticConvention.DB_FILTER, str(filter_query))

        # Response metrics
        if scope._response and hasattr(scope._response, "update_info"):
            scope._span.set_attribute(
                SemanticConvention.DB_RESPONSE_RETURNED_ROWS,
                scope._response.update_info.get("nModified", 0),
            )

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"filter={str(filter_query)[:100]}... "
            f"update={str(update_query)[:100]}...",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_REPLACE:
        # Handle replace operations (find_one_and_update, replace_one)
        filter_query = (
            scope._args[0] if scope._args else scope._kwargs.get("filter", {})
        )

        # Check if it's an upsert operation
        if scope._kwargs.get("upsert"):
            scope._db_operation = SemanticConvention.DB_OPERATION_UPSERT
            scope._span.set_attribute(
                SemanticConvention.DB_OPERATION_NAME,
                SemanticConvention.DB_OPERATION_UPSERT,
            )

        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(filter_query))
        scope._span.set_attribute(SemanticConvention.DB_FILTER, str(filter_query))

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"filter={str(filter_query)[:100]}... "
            f"upsert={scope._kwargs.get('upsert', False)}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_SELECT:
        # Handle find operations
        filter_query = (
            scope._args[0] if scope._args else scope._kwargs.get("filter", {})
        )

        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(filter_query))
        scope._span.set_attribute(SemanticConvention.DB_FILTER, str(filter_query))

        # Response metrics
        if scope._response and hasattr(scope._response, "__len__"):
            scope._span.set_attribute(
                SemanticConvention.DB_RESPONSE_RETURNED_ROWS, len(scope._response)
            )

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"filter={str(filter_query)[:100]}...",
        )

    elif scope._db_operation in [
        SemanticConvention.DB_OPERATION_DELETE,
        SemanticConvention.DB_OPERATION_FIND_AND_DELETE,
    ]:
        # Handle delete operations (delete_one, delete_many, find_one_and_delete)
        filter_query = (
            scope._args[0] if scope._args else scope._kwargs.get("filter", {})
        )

        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(filter_query))
        scope._span.set_attribute(SemanticConvention.DB_FILTER, str(filter_query))

        # Response metrics
        if scope._response and hasattr(scope._response, "deleted_count"):
            scope._span.set_attribute(
                SemanticConvention.DB_RESPONSE_RETURNED_ROWS,
                scope._response.deleted_count,
            )

        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"filter={str(filter_query)[:100]}...",
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Record metrics using helper
    if not disable_metrics:
        record_db_metrics(
            metrics,
            SemanticConvention.DB_SYSTEM_ASTRA,
            scope._server_address,
            scope._server_port,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            scope._db_operation,
        )


def process_astra_response(
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
    Process AstraDB response and generate telemetry.

    Args:
        response: Response from AstraDB operation
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
        version: AstraDB client version
        instance: AstraDB client instance
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
    scope._args = args

    # Process the response using common logic
    common_astra_logic(
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
