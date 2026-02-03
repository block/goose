# pylint: disable=duplicate-code, broad-exception-caught, too-many-statements, unused-argument, possibly-used-before-assignment, too-many-branches
"""
Module for monitoring Psycopg (PostgreSQL) async operations.
"""

import time
from opentelemetry.trace import SpanKind
from opentelemetry import context as context_api
from openlit.__helpers import handle_exception
from openlit.instrumentation.psycopg.utils import (
    parse_sql_operation,
    extract_table_name,
    extract_connection_info,
    extract_database_name,
    process_cursor_response,
    process_connection_response,
    inject_sql_comment,
)
from openlit.semcov import SemanticConvention


def async_execute_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
    capture_parameters=False,
    enable_sqlcommenter=False,
):
    """
    Generates a telemetry wrapper for AsyncCursor.execute operations.

    Args:
        capture_parameters: If True, captures query parameters in spans (security risk!)
        enable_sqlcommenter: If True, injects trace context as SQL comments
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AsyncCursor.execute operation with tracing and logging.
        """
        # CRITICAL: Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        # Extract query and params from args/kwargs
        query = args[0] if args else kwargs.get("query", "")
        params = args[1] if len(args) > 1 else kwargs.get("params", None)

        # Parse operation and table
        db_operation = parse_sql_operation(query)
        table_name = extract_table_name(query)

        # Get connection from cursor
        connection = getattr(instance, "connection", None)

        # Server address calculation
        server_address, server_port = extract_connection_info(connection)
        database_name = extract_database_name(connection)

        # Span naming: use operation + table
        span_name = f"{db_operation} {table_name}"

        # CRITICAL: Use tracer.start_as_current_span() for proper context
        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()

            # Inject SQLCommenter if enabled (must be done inside span context)
            modified_query = inject_sql_comment(
                query, application_name, enable_sqlcommenter
            )
            if modified_query != query:
                args = (modified_query,) + args[1:] if args else args

            response = await wrapped(*args, **kwargs)

            try:
                # Process response with endpoint information
                response = process_cursor_response(
                    response,
                    db_operation,
                    table_name,
                    query,  # Use original query for logging
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
                    cursor=instance,
                    connection=connection,
                    endpoint=gen_ai_endpoint,
                    capture_parameters=capture_parameters,
                    params=params,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def async_executemany_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
    capture_parameters=False,
    enable_sqlcommenter=False,
):
    """
    Generates a telemetry wrapper for AsyncCursor.executemany operations.

    Note: For executemany, capture_parameters only captures first batch item
    to avoid huge traces.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AsyncCursor.executemany operation with tracing and logging.
        """
        # CRITICAL: Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        # Extract query from args/kwargs
        query = args[0] if args else kwargs.get("query", "")
        params_seq = args[1] if len(args) > 1 else kwargs.get("params_seq", [])

        # Parse operation and table
        db_operation = parse_sql_operation(query)
        table_name = extract_table_name(query)

        # Get connection from cursor
        connection = getattr(instance, "connection", None)

        # Server address calculation
        server_address, server_port = extract_connection_info(connection)
        database_name = extract_database_name(connection)

        # Span naming: use operation + table + batch indicator
        params_list = list(params_seq) if params_seq else []
        batch_size = len(params_list)
        span_name = f"{db_operation} {table_name} (batch)"

        # CRITICAL: Use tracer.start_as_current_span() for proper context
        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()

            # Inject SQLCommenter if enabled
            modified_query = inject_sql_comment(
                query, application_name, enable_sqlcommenter
            )
            if modified_query != query:
                args = (modified_query,) + args[1:] if args else args

            response = await wrapped(*args, **kwargs)

            try:
                # Add batch-specific attribute
                span.set_attribute("db.batch.size", batch_size)

                # For executemany, only capture first param set to avoid huge traces
                first_params = params_list[0] if params_list else None

                # Process response with endpoint information
                response = process_cursor_response(
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
                    cursor=instance,
                    connection=connection,
                    endpoint=gen_ai_endpoint,
                    capture_parameters=capture_parameters,
                    params=first_params,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def async_copy_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
    capture_parameters=False,
    enable_sqlcommenter=False,
):
    """
    Generates a telemetry wrapper for AsyncCursor.copy operations.

    Note: COPY operations don't typically have parameters, so capture_parameters
    has limited effect here. SQLCommenter is also not injected for COPY statements.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AsyncCursor.copy operation with tracing and logging.
        """
        # CRITICAL: Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        # Extract statement from args/kwargs
        statement = args[0] if args else kwargs.get("statement", "")

        # Parse operation and table
        db_operation = SemanticConvention.DB_OPERATION_COPY
        table_name = extract_table_name(statement)

        # Get connection from cursor
        connection = getattr(instance, "connection", None)

        # Server address calculation
        server_address, server_port = extract_connection_info(connection)
        database_name = extract_database_name(connection)

        # Span naming
        span_name = f"{db_operation} {table_name}"

        # CRITICAL: Use tracer.start_as_current_span() for proper context
        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                # Process response with endpoint information
                response = process_cursor_response(
                    response,
                    db_operation,
                    table_name,
                    statement,
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
                    cursor=instance,
                    connection=connection,
                    endpoint=gen_ai_endpoint,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def async_commit_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
    capture_parameters=False,
    enable_sqlcommenter=False,
):
    """
    Generates a telemetry wrapper for AsyncConnection.commit operations.

    Note: capture_parameters and enable_sqlcommenter are accepted for API
    consistency but not used for commit operations.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AsyncConnection.commit operation with tracing and logging.
        """
        # CRITICAL: Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        db_operation = SemanticConvention.DB_OPERATION_COMMIT

        # Server address calculation
        server_address, server_port = extract_connection_info(instance)
        database_name = extract_database_name(instance)

        # Span naming
        span_name = f"{db_operation}"

        # CRITICAL: Use tracer.start_as_current_span() for proper context
        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                # Process response with endpoint information
                response = process_connection_response(
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
                    connection=instance,
                    endpoint=gen_ai_endpoint,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def async_rollback_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
    capture_parameters=False,
    enable_sqlcommenter=False,
):
    """
    Generates a telemetry wrapper for AsyncConnection.rollback operations.

    Note: capture_parameters and enable_sqlcommenter are accepted for API
    consistency but not used for rollback operations.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AsyncConnection.rollback operation with tracing and logging.
        """
        # CRITICAL: Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        db_operation = SemanticConvention.DB_OPERATION_ROLLBACK

        # Server address calculation
        server_address, server_port = extract_connection_info(instance)
        database_name = extract_database_name(instance)

        # Span naming
        span_name = f"{db_operation}"

        # CRITICAL: Use tracer.start_as_current_span() for proper context
        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                # Process response with endpoint information
                response = process_connection_response(
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
                    connection=instance,
                    endpoint=gen_ai_endpoint,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def async_callproc_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
    capture_parameters=False,
    enable_sqlcommenter=False,
):
    """
    Generates a telemetry wrapper for AsyncCursor.callproc operations (stored procedures).

    Note: enable_sqlcommenter is not applicable to callproc (procedure calls
    don't use SQL strings). capture_parameters captures procedure arguments.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AsyncCursor.callproc operation with tracing and logging.
        """
        # CRITICAL: Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        # Extract procedure name and parameters from args
        proc_name = args[0] if args else kwargs.get("procname", "unknown")
        proc_params = args[1] if len(args) > 1 else kwargs.get("parameters", None)

        # Parse operation
        db_operation = SemanticConvention.DB_OPERATION_CALL

        # Get connection from cursor
        connection = getattr(instance, "connection", None)

        # Server address calculation
        server_address, server_port = extract_connection_info(connection)
        database_name = extract_database_name(connection)

        # Span naming: use CALL + procedure name
        span_name = f"{db_operation} {proc_name}"

        # CRITICAL: Use tracer.start_as_current_span() for proper context
        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                # Process response with endpoint information
                response = process_cursor_response(
                    response,
                    db_operation,
                    proc_name,
                    f"CALL {proc_name}",
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
                    cursor=instance,
                    connection=connection,
                    endpoint=gen_ai_endpoint,
                    capture_parameters=capture_parameters,
                    params=proc_params,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper


def async_pool_getconn_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
    capture_parameters=False,
    enable_sqlcommenter=False,
):
    """
    Generates a telemetry wrapper for AsyncConnectionPool.getconn operations.

    Note: capture_parameters and enable_sqlcommenter are accepted for API
    consistency but not used for pool operations.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AsyncConnectionPool.getconn operation with tracing and logging.
        """
        # CRITICAL: Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        db_operation = "pool.getconn"

        # Span naming
        span_name = "PostgreSQL pool.getconn"

        # CRITICAL: Use tracer.start_as_current_span() for proper context
        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)
            end_time = time.time()

            try:
                # Set pool-specific attributes
                span.set_attribute(
                    SemanticConvention.DB_SYSTEM_NAME,
                    SemanticConvention.DB_SYSTEM_POSTGRESQL,
                )
                span.set_attribute(SemanticConvention.DB_OPERATION_NAME, db_operation)
                span.set_attribute(
                    SemanticConvention.DB_CLIENT_OPERATION_DURATION,
                    end_time - start_time,
                )

                # Try to get pool stats
                if hasattr(instance, "get_stats"):
                    try:
                        stats = instance.get_stats()
                        span.set_attribute("db.pool.size", stats.get("pool_size", 0))
                        span.set_attribute(
                            "db.pool.available", stats.get("pool_available", 0)
                        )
                    except Exception:
                        pass

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper
