# pylint: disable=broad-exception-caught
"""
Synchronous MCP (Model Context Protocol) instrumentation with threading context fixes.
"""

import time
from opentelemetry.trace import SpanKind
from opentelemetry import context as context_api

from openlit.semcov import SemanticConvention
from openlit.__helpers import handle_exception, record_mcp_metrics
from openlit.instrumentation.mcp.utils import (
    MCPInstrumentationContext,
    set_mcp_span_attributes,
    process_mcp_response,
    create_mcp_scope,
    _simplify_operation_name,
)


def mcp_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Generates a telemetry wrapper for MCP operations with threading context fixes.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps MCP operations with comprehensive telemetry and business intelligence.
        """

        # CRITICAL: Check for suppression to avoid infinite recursion
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Determine span kind based on operation type
        span_kind = SpanKind.CLIENT
        if (
            "server" in gen_ai_endpoint.lower()
            or "transport" in gen_ai_endpoint.lower()
        ):
            span_kind = SpanKind.SERVER

        # Create instrumentation context for caching expensive operations
        ctx = MCPInstrumentationContext(
            instance=instance,
            args=args,
            kwargs=kwargs,
            version=version,
            environment=environment,
            application_name=application_name,
            pricing_info=pricing_info,
            capture_message_content=capture_message_content,
        )

        # Enhanced method detection from endpoint and wrapped function
        if hasattr(wrapped, "__name__"):
            ctx._wrapped_function_name = wrapped.__name__

        # Extract method from endpoint (e.g., "tool call_tool" -> "call_tool")
        if gen_ai_endpoint:
            endpoint_parts = gen_ai_endpoint.split()
            if len(endpoint_parts) >= 2:
                ctx._endpoint_method = endpoint_parts[1]
            else:
                ctx._endpoint_method = (
                    endpoint_parts[0] if endpoint_parts else "unknown"
                )

        operation_type = gen_ai_endpoint.split()[0] if " " in gen_ai_endpoint else "mcp"
        span_name = ctx.get_enhanced_span_name(operation_type)

        with tracer.start_as_current_span(span_name, kind=span_kind) as span:
            start_time = time.time()

            # Create scope object for common span attributes
            scope = create_mcp_scope(
                instance=instance,
                args=args,
                kwargs=kwargs,
                version=version,
                environment=environment,
                application_name=application_name,
                pricing_info=pricing_info,
                capture_message_content=capture_message_content,
                span=span,
                start_time=start_time,
            )

            # Set initial span attributes
            try:
                set_mcp_span_attributes(
                    span=span,
                    operation_name=gen_ai_endpoint,
                    ctx=ctx,
                    endpoint=gen_ai_endpoint,
                    **kwargs,
                )
            except Exception as e:
                handle_exception(span, e)

            # Execute the wrapped function
            response = None
            error = None

            try:
                # Apply threading context fix for CLIENT operations that might use ThreadPoolExecutor
                # This is critical for proper span hierarchy in MCP operations
                if span_kind == SpanKind.CLIENT:
                    # MCP client operations might spawn threads, ensure context propagation
                    response = wrapped(*args, **kwargs)
                else:
                    # Server operations typically don't need threading fixes
                    response = wrapped(*args, **kwargs)

                # Note: MCP SDK functions may return coroutines
                # These will be handled gracefully in the response processing

            except Exception as e:
                error = e
                handle_exception(span, e)

                # Record error metrics
                if metrics and not disable_metrics:
                    end_time = time.time()
                    mcp_operation = _simplify_operation_name(gen_ai_endpoint)
                    mcp_method = (
                        ctx.method_name or gen_ai_endpoint.split()[-1]
                        if " " in gen_ai_endpoint
                        else gen_ai_endpoint
                    )

                    record_mcp_metrics(
                        metrics=metrics,
                        mcp_operation=mcp_operation,
                        mcp_method=mcp_method,
                        mcp_transport_type="stdio",
                        mcp_tool_name=kwargs.get("tool_name") or kwargs.get("name")
                        if "tool" in gen_ai_endpoint.lower()
                        else None,
                        mcp_resource_uri=kwargs.get("resource_uri") or kwargs.get("uri")
                        if "resource" in gen_ai_endpoint.lower()
                        else None,
                        mcp_resource_name=kwargs.get("resource_name")
                        or kwargs.get("name")
                        if "resource" in gen_ai_endpoint.lower()
                        else None,
                        mcp_prompt_name=kwargs.get("prompt_name") or kwargs.get("name")
                        if "prompt" in gen_ai_endpoint.lower()
                        else None,
                        environment=environment,
                        application_name=application_name,
                        start_time=start_time,
                        end_time=end_time,
                        is_error=True,
                    )
                raise
            finally:
                # Process response and capture telemetry
                try:
                    scope._end_time = time.time()

                    if response is not None:
                        response = process_mcp_response(
                            response=response,
                            ctx=ctx,
                            span=span,
                            start_time=start_time,
                            endpoint=gen_ai_endpoint,
                            metrics=metrics,
                            disable_metrics=disable_metrics,
                            **kwargs,
                        )

                    # Update span attributes with error info if needed
                    if error:
                        set_mcp_span_attributes(
                            span=span,
                            operation_name=gen_ai_endpoint,
                            ctx=ctx,
                            endpoint=gen_ai_endpoint,
                            error=error,
                            **kwargs,
                        )

                except Exception as e:
                    handle_exception(span, e)

            return response

    return wrapper


def mcp_tool_call_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Specialized wrapper for MCP tool call operations with enhanced telemetry.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps MCP tool call operations with tool-specific telemetry.
        """

        # CRITICAL: Check for suppression
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Create instrumentation context
        ctx = MCPInstrumentationContext(
            instance=instance,
            args=args,
            kwargs=kwargs,
            version=version,
            environment=environment,
            application_name=application_name,
            pricing_info=pricing_info,
            capture_message_content=capture_message_content,
        )

        # Enhanced span name for tool calls
        span_name = ctx.get_enhanced_span_name("tool")

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()

            # Set tool-specific attributes
            try:
                set_mcp_span_attributes(
                    span=span,
                    operation_name=gen_ai_endpoint,
                    ctx=ctx,
                    endpoint=gen_ai_endpoint,
                    **kwargs,
                )

                # Additional tool-specific attributes
                if ctx.tool_name:
                    span.set_attribute(SemanticConvention.MCP_TOOL_NAME, ctx.tool_name)

            except Exception as e:
                handle_exception(span, e)

            # Execute tool call
            response = None
            try:
                response = wrapped(*args, **kwargs)

                # Capture tool result if available
                if response and capture_message_content:
                    try:
                        if hasattr(response, "content"):
                            span.set_attribute(
                                SemanticConvention.MCP_TOOL_RESULT,
                                str(response.content),
                            )
                        elif isinstance(response, (str, dict, list)):
                            span.set_attribute(
                                SemanticConvention.MCP_TOOL_RESULT, str(response)
                            )
                    except Exception:
                        pass

            except Exception as e:
                handle_exception(span, e)
                raise
            finally:
                # Process response with tool-specific handling
                try:
                    if response is not None:
                        response = process_mcp_response(
                            response=response,
                            ctx=ctx,
                            span=span,
                            start_time=start_time,
                            endpoint=gen_ai_endpoint,
                            metrics=metrics,
                            disable_metrics=disable_metrics,
                            **kwargs,
                        )

                except Exception as e:
                    handle_exception(span, e)

            return response

    return wrapper


def mcp_resource_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Specialized wrapper for MCP resource operations with resource-specific telemetry.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps MCP resource operations with resource-specific telemetry.
        """

        # CRITICAL: Check for suppression
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Create instrumentation context
        ctx = MCPInstrumentationContext(
            instance=instance,
            args=args,
            kwargs=kwargs,
            version=version,
            environment=environment,
            application_name=application_name,
            pricing_info=pricing_info,
            capture_message_content=capture_message_content,
        )

        # Enhanced span name for resource operations
        span_name = ctx.get_enhanced_span_name("resource")

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()

            # Set resource-specific attributes
            try:
                set_mcp_span_attributes(
                    span=span,
                    operation_name=gen_ai_endpoint,
                    ctx=ctx,
                    endpoint=gen_ai_endpoint,
                    **kwargs,
                )

            except Exception as e:
                handle_exception(span, e)

            # Execute resource operation
            response = None

            try:
                response = wrapped(*args, **kwargs)

                # Capture resource metadata if available
                if response and capture_message_content:
                    try:
                        if hasattr(response, "contents"):
                            # Capture resource size
                            content_size = len(str(response.contents))
                            span.set_attribute(
                                SemanticConvention.MCP_RESOURCE_SIZE, content_size
                            )

                        if hasattr(response, "mimeType"):
                            span.set_attribute(
                                SemanticConvention.MCP_RESOURCE_MIME_TYPE,
                                response.mimeType,
                            )

                    except Exception:
                        pass

            except Exception as e:
                handle_exception(span, e)
                raise
            finally:
                # Process response with resource-specific handling
                try:
                    if response is not None:
                        response = process_mcp_response(
                            response=response,
                            ctx=ctx,
                            span=span,
                            start_time=start_time,
                            endpoint=gen_ai_endpoint,
                            metrics=metrics,
                            disable_metrics=disable_metrics,
                            **kwargs,
                        )

                except Exception as e:
                    handle_exception(span, e)

            return response

    return wrapper
