"""
ChromaDB OpenTelemetry instrumentation utility functions
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
    "chroma.create_collection": SemanticConvention.DB_OPERATION_CREATE_COLLECTION,
    "chroma.add": SemanticConvention.DB_OPERATION_INSERT,
    "chroma.get": SemanticConvention.DB_OPERATION_GET,
    "chroma.peek": SemanticConvention.DB_OPERATION_PEEK,
    "chroma.query": SemanticConvention.DB_OPERATION_GET,
    "chroma.update": SemanticConvention.DB_OPERATION_UPDATE,
    "chroma.upsert": SemanticConvention.DB_OPERATION_UPSERT,
    "chroma.delete": SemanticConvention.DB_OPERATION_DELETE,
}


def object_count(obj):
    """
    Counts length of object if it exists, else returns 0.
    """
    return len(obj) if obj else 0


def set_server_address_and_port(instance):
    """
    Extracts server address and port from ChromaDB client instance.

    Args:
        instance: ChromaDB client instance

    Returns:
        tuple: (server_address, server_port)
    """
    server_address = "localhost"
    server_port = 8000

    # Try getting base_url from multiple potential attributes
    base_client = getattr(instance, "_client", None)
    base_url = getattr(base_client, "base_url", None)

    if not base_url:
        # Attempt to get endpoint from instance._config.endpoint
        config = getattr(instance, "_config", None)
        base_url = getattr(config, "endpoint", None)

    if not base_url:
        # Attempt to get server_url from instance.sdk_configuration.server_url
        config = getattr(instance, "sdk_configuration", None)
        base_url = getattr(config, "server_url", None)

    if base_url:
        if isinstance(base_url, str):
            # Check if it a full URL or just a hostname
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
    endpoint=None,
):
    """
    Process vector database request and generate telemetry.
    """

    scope._end_time = time.time()

    # Set common database span attributes using helper
    common_db_span_attributes(
        scope,
        SemanticConvention.DB_SYSTEM_CHROMA,
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

    # Set collection name from instance
    if hasattr(instance, "name"):
        scope._span.set_attribute(SemanticConvention.DB_COLLECTION_NAME, instance.name)

    # Set Create Collection operation specific attributes
    if scope._db_operation == SemanticConvention.DB_OPERATION_CREATE_COLLECTION:
        # Standard database attributes
        collection_name = scope._kwargs.get("name") or (
            scope._args[0] if scope._args else "unknown"
        )
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )

        # Vector database specific attributes (extensions)
        metadata = scope._kwargs.get("metadata", {})
        if metadata:
            scope._span.set_attribute(
                SemanticConvention.DB_COLLECTION_METADATA, str(metadata)
            )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_INSERT:
        collection_name = getattr(instance, "name", "unknown")
        query = scope._kwargs.get("ids", [])

        # Standard database attributes
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(query))
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )
        scope._span.set_attribute(
            SemanticConvention.DB_VECTOR_COUNT, object_count(query)
        )

        # Vector database specific attributes (extensions)
        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"ids={query} "
            f"documents={scope._kwargs.get('documents', [])}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_GET:
        collection_name = getattr(instance, "name", "unknown")

        # Handle different GET operations based on endpoint
        if endpoint == "chroma.get":
            # Collection.get() - retrieve documents by IDs
            query = scope._kwargs.get("ids", [])

            # Standard database attributes
            scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(query))
            scope._span.set_attribute(
                SemanticConvention.DB_COLLECTION_NAME, collection_name
            )
            scope._span.set_attribute(
                SemanticConvention.DB_VECTOR_COUNT, object_count(query)
            )

            # Vector database specific attributes (extensions)
            scope._span.set_attribute(
                SemanticConvention.DB_FILTER, str(scope._kwargs.get("where", ""))
            )

            scope._span.set_attribute(
                SemanticConvention.DB_QUERY_SUMMARY,
                f"{scope._db_operation} {collection_name} "
                f"ids={query} "
                f"limit={scope._kwargs.get('limit', 'None')} "
                f"offset={scope._kwargs.get('offset', 'None')}",
            )

        elif endpoint == "chroma.query":
            query_texts = scope._kwargs.get("query_texts", [])
            query_embeddings = scope._kwargs.get("query_embeddings", [])

            # Create comprehensive query text (can be either embeddings or texts)
            if query_texts:
                query_content = f"texts={query_texts}"
            elif query_embeddings:
                query_content = f"embeddings={len(query_embeddings) if query_embeddings else 0} vectors"
            else:
                query_content = "no query provided"

            # Standard database attributes
            scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, query_content)
            scope._span.set_attribute(
                SemanticConvention.DB_COLLECTION_NAME, collection_name
            )

            # Vector database specific attributes (extensions)
            scope._span.set_attribute(
                SemanticConvention.DB_VECTOR_QUERY_TOP_K,
                scope._kwargs.get("n_results", 10),
            )
            scope._span.set_attribute(
                SemanticConvention.DB_FILTER, str(scope._kwargs.get("where", ""))
            )

            # Extract response metrics if available
            if scope._response:
                # Get number of results returned
                if hasattr(scope._response, "get") and scope._response.get("ids"):
                    returned_rows = (
                        object_count(scope._response["ids"][0])
                        if scope._response["ids"]
                        else 0
                    )
                    scope._span.set_attribute(
                        SemanticConvention.DB_RESPONSE_RETURNED_ROWS, returned_rows
                    )

            scope._span.set_attribute(
                SemanticConvention.DB_QUERY_SUMMARY,
                f"{scope._db_operation} {collection_name} "
                f"n_results={scope._kwargs.get('n_results', 10)} "
                f"{query_content} "
                f"filter={scope._kwargs.get('where', 'None')}",
            )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_UPDATE:
        collection_name = getattr(instance, "name", "unknown")
        query = scope._kwargs.get("ids", [])

        # Standard database attributes
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(query))
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )
        scope._span.set_attribute(
            SemanticConvention.DB_VECTOR_COUNT, object_count(query)
        )

        # Vector database specific attributes (extensions)
        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"ids={query} "
            f"embeddings={scope._kwargs.get('embeddings', 'None')} "
            f"metadatas={scope._kwargs.get('metadatas', 'None')} "
            f"documents={scope._kwargs.get('documents', 'None')}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_UPSERT:
        collection_name = getattr(instance, "name", "unknown")
        query = scope._kwargs.get("ids", [])

        # Standard database attributes
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(query))
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )
        scope._span.set_attribute(
            SemanticConvention.DB_VECTOR_COUNT, object_count(query)
        )

        # Vector database specific attributes (extensions)
        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"ids={query} "
            f"embeddings={scope._kwargs.get('embeddings', 'None')} "
            f"metadatas={scope._kwargs.get('metadatas', 'None')} "
            f"documents={scope._kwargs.get('documents', 'None')}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_DELETE:
        collection_name = getattr(instance, "name", "unknown")
        query = scope._kwargs.get("ids", [])

        # Standard database attributes
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, str(query))
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )
        scope._span.set_attribute(
            SemanticConvention.DB_VECTOR_COUNT, object_count(query)
        )

        # Vector database specific attributes (extensions)
        scope._span.set_attribute(
            SemanticConvention.DB_FILTER, str(scope._kwargs.get("where", ""))
        )
        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"ids={query} "
            f"filter={scope._kwargs.get('where', 'None')} "
            f"delete_all={scope._kwargs.get('delete_all', False)}",
        )

    elif scope._db_operation == SemanticConvention.DB_OPERATION_PEEK:
        collection_name = getattr(instance, "name", "unknown")
        query = f"PEEK limit={scope._kwargs.get('limit', '')}"

        # Standard database attributes
        scope._span.set_attribute(SemanticConvention.DB_QUERY_TEXT, query)
        scope._span.set_attribute(
            SemanticConvention.DB_COLLECTION_NAME, collection_name
        )
        scope._span.set_attribute(
            SemanticConvention.DB_VECTOR_COUNT, scope._kwargs.get("limit", "")
        )

        # Vector database specific attributes (extensions)
        scope._span.set_attribute(
            SemanticConvention.DB_QUERY_SUMMARY,
            f"{scope._db_operation} {collection_name} "
            f"limit={scope._kwargs.get('limit', 'None')}",
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Record metrics using helper
    if not disable_metrics:
        record_db_metrics(
            metrics,
            SemanticConvention.DB_SYSTEM_CHROMA,
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
    endpoint=None,
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
        endpoint,
    )

    return response
