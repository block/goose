"""
Pinecone OpenTelemetry instrumentation utility functions
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
    "pinecone.create_collection": SemanticConvention.DB_OPERATION_CREATE_COLLECTION,
    "pinecone.upsert": SemanticConvention.DB_OPERATION_UPSERT,
    "pinecone.query": SemanticConvention.DB_OPERATION_QUERY,
    "pinecone.search": SemanticConvention.DB_OPERATION_SEARCH,
    "pinecone.fetch": SemanticConvention.DB_OPERATION_FETCH,
    "pinecone.update": SemanticConvention.DB_OPERATION_UPDATE,
    "pinecone.delete": SemanticConvention.DB_OPERATION_DELETE,
    "pinecone.upsert_records": SemanticConvention.DB_OPERATION_UPSERT,
    "pinecone.search_records": SemanticConvention.DB_OPERATION_QUERY,
}


def object_count(obj):
    """
    Counts length of object if it exists, else returns 0.
    """
    return len(obj) if obj else 0


def set_server_address_and_port(instance):
    """
    Extracts server address and port from Pinecone client instance.

    Args:
        instance: Pinecone client instance

    Returns:
        tuple: (server_address, server_port)
    """
    server_address = "pinecone.io"
    server_port = 443

    # Try getting base_url from multiple potential attributes
    base_client = getattr(instance, "_client", None)
    base_url = getattr(base_client, "base_url", None)

    if not base_url:
        # Attempt to get host from instance.config.host (used by Pinecone)
        config = getattr(instance, "config", None)
        base_url = getattr(config, "host", None)

    if base_url:
        if isinstance(base_url, str):
            # Check if its a full URL or just a hostname
            if base_url.startswith(("http://", "https://")):
                try:
                    url = urlparse(base_url)
                    if url.hostname:
                        server_address = url.hostname
                    if url.port:
                        server_port = url.port
                except Exception:
                    pass
            else:
                # Just a hostname
                server_address = base_url

    return server_address, server_port


def common_vectordb_logic(
    scope,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    instance=None,
):
    """
    Process vector database request and generate telemetry.
    """

    scope._end_time = time.time()

    # Set common database span attributes using helper
    common_db_span_attributes(
        scope,
        SemanticConvention.DB_SYSTEM_PINECONE,
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

    # Set Create Index operation specific attributes
    if scope._db_operation == SemanticConvention.DB_OPERATION_CREATE_COLLECTION:
        # Standard database attributes
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, scope._kwargs.get("name", "")
        )

        # Vector database specific attributes (extensions)
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_DIMENSION,
            scope._kwargs.get("dimension", -1),
        )
        scope._span.set_attribute(
            SemanticConvention.DB_SEARCH_SIMILARITY_METRIC,
            scope._kwargs.get("metric", "cosine"),
        )
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_SPEC, str(scope._kwargs.get("spec", ""))
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_SEARCH:
        namespace = scope._kwargs.get("namespace", "default") or (
            scope._args[0] if scope._args else "unknown"
        )
        query = scope._kwargs.get("query", {})

        # Extract query text or vector from different possible locations
        query_text = query.get("inputs", {}).get("text", "")
        query_vector = query.get("vector", {})
        query_content = query_text or str(query_vector)

        # Standard database attributes
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, query_content)
        scope._span.set_attribute(SemanticConvention.DB_NAMESPACE, namespace)

        # Vector database specific attributes (extensions)
        scope._span.set_attribute(
            SemanticConvention.DB_VECTOR_QUERY_TOP_K, query.get("top_k", -1)
        )
        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {namespace} "
            f"top_k={query.get('top_k', -1)} "
            f"text={query_text} "
            f"vector={query_vector}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_QUERY:
        namespace = scope._kwargs.get("namespace", "default") or (
            scope._args[0] if scope._args else "unknown"
        )
        query = scope._kwargs.get("vector", [])

        # Standard database attributes
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(query))
        scope._span.set_attribute(SemanticConvention.DB_NAMESPACE, namespace)

        # Vector database specific attributes (extensions)
        scope._span.set_attribute(
            SemanticConvention.DB_VECTOR_QUERY_TOP_K, scope._kwargs.get("top_k", "")
        )
        scope._span.set_attribute(
            SemanticConvention.DB_FILTER, str(scope._kwargs.get("filter", ""))
        )
        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {namespace} "
            f"top_k={scope._kwargs.get('top_k', -1)} "
            f"filtered={scope._kwargs.get('filter', '')} "
            f"vector={scope._kwargs.get('vector', '')}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_FETCH:
        namespace = scope._kwargs.get("namespace", "default") or (
            scope._args[0] if scope._args else "unknown"
        )
        query = scope._kwargs.get("ids", [])

        # Standard database attributes
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(query))
        scope._span.set_attribute(SemanticConvention.DB_NAMESPACE, namespace)

        # Vector database specific attributes (extensions)
        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {namespace} ids={query}",
        )
        scope._span.set_attribute(
            SemanticConvention.DB_RESPONSE_RETURNED_ROWS,
            object_count(scope._response.vectors),
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_UPDATE:
        namespace = scope._kwargs.get("namespace") or (
            scope._args[0] if scope._args else "unknown"
        )
        query = scope._kwargs.get("id", "")

        # Standard database attributes
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, query)
        scope._span.set_attribute(SemanticConvention.DB_NAMESPACE, namespace)

        # Vector database specific attributes (extensions)
        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {namespace} "
            f"id={query} "
            f"values={scope._kwargs.get('values', [])} "
            f"set_metadata={scope._kwargs.get('set_metadata', '')}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_UPSERT:
        namespace = scope._kwargs.get("namespace") or (
            scope._args[0] if scope._args else "unknown"
        )
        query = scope._kwargs.get("vectors") or (
            scope._args[1] if len(scope._args) > 1 else None
        )

        # Standard database attributes
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(query))
        scope._span.set_attribute(SemanticConvention.DB_NAMESPACE, namespace)

        # Vector database specific attributes (extensions)
        scope._span.set_attribute(
            SemanticConvention.DB_VECTOR_COUNT, object_count(query)
        )
        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {namespace} vectors_count={object_count(query)}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_DELETE:
        namespace = scope._kwargs.get("namespace") or (
            scope._args[0] if scope._args else "unknown"
        )
        query = scope._kwargs.get("ids") or (
            scope._args[1] if len(scope._args) > 1 else None
        )

        # Standard database attributes
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(query))
        scope._span.set_attribute(SemanticConvention.DB_NAMESPACE, namespace)

        # Vector database specific attributes (extensions)
        scope._span.set_attribute(
            SemanticConvention.DB_ID_COUNT, object_count(scope._kwargs.get("ids"))
        )
        scope._span.set_attribute(
            SemanticConvention.DB_FILTER, str(scope._kwargs.get("filter", ""))
        )
        scope._span.set_attribute(
            SemanticConvention.DB_DELETE_ALL, scope._kwargs.get("delete_all", False)
        )
        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {namespace} "
            f"ids={query} "
            f"filter={scope._kwargs.get('filter', '')} "
            f"delete_all={scope._kwargs.get('delete_all', False)}",
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Record metrics using helper
    if not disable_metrics:
        record_db_metrics(
            metrics,
            SemanticConvention.DB_SYSTEM_PINECONE,
            scope._server_address,
            scope._server_port,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            scope._db_operation,
        )


def process_vectordb_response(
    response,
    db_operation,
    server_address,
    server_port,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
    instance=None,
    args=None,
    **kwargs,
):
    """
    Process vector database response and generate telemetry following OpenTelemetry conventions.
    """

    scope = type("GenericScope", (), {})()

    scope._start_time = start_time
    scope._span = span
    scope._kwargs = kwargs
    scope._args = args or []
    scope._db_operation = db_operation
    scope._response = response
    scope._server_address = server_address
    scope._server_port = server_port

    common_vectordb_logic(
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
