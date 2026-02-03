"""
Letta OpenTelemetry instrumentation utility functions following framework guide patterns
"""

import json
import time
from opentelemetry.trace import Status, StatusCode, SpanKind
from openlit.__helpers import (
    common_framework_span_attributes,
    handle_exception,
    get_chat_model_cost,
)
from openlit.semcov import SemanticConvention


def process_letta_response(
    span,
    response,
    kwargs,
    operation_type,
    instance,
    start_time,
    environment,
    application_name,
    version,
    endpoint,
    capture_message_content=False,
    pricing_info=None,
):
    """Process Letta response and set appropriate span attributes using common helpers"""

    end_time = time.time()

    # Create proper scope object for common_framework_span_attributes
    scope = type("LettaScope", (), {})()
    scope._span = span
    scope._start_time = start_time
    scope._end_time = end_time

    # Create model wrapper for framework span attributes
    class LettaModelWrapper:  # pylint: disable=too-few-public-methods
        """Model wrapper for Letta instances to provide consistent interface"""

        def __init__(self, original_instance, model_name):
            self._original = original_instance
            self.model_name = model_name

        def __getattr__(self, name):
            return getattr(self._original, name) if self._original else None

    # Extract model name from various sources (priority order)
    model_name = "gpt-4o"  # Default Letta model

    # Priority 1: Response LLM config (most accurate)
    if (
        response
        and hasattr(response, "llm_config")
        and hasattr(response.llm_config, "model")
    ):
        model_name = response.llm_config.model
    # Priority 2: Kwargs model parameter
    elif "model" in kwargs:
        model_name = kwargs["model"]
    # Priority 3: Instance model if available
    elif instance and hasattr(instance, "model"):
        model_name = instance.model

    model_instance = LettaModelWrapper(instance, model_name)

    # Get server address and port for Letta agent
    server_address = "api.letta.com"
    server_port = 443

    # Try to get actual server info from instance if available
    if instance:
        # Try various common client attributes for base URL
        base_url = None

        # Method 1: Check Letta-specific _client_wrapper._base_url pattern
        if hasattr(instance, "_client_wrapper") and hasattr(
            instance._client_wrapper, "_base_url"
        ):
            base_url = instance._client_wrapper._base_url
        # Method 2: Check if instance has direct _client with base_url
        elif hasattr(instance, "_client") and hasattr(instance._client, "base_url"):
            base_url = instance._client.base_url
        # Method 3: Check if instance itself has base_url
        elif hasattr(instance, "base_url"):
            base_url = instance.base_url
        # Method 4: Check if instance has _base_url
        elif hasattr(instance, "_base_url"):
            base_url = instance._base_url
        # Method 5: Check SDK configuration patterns
        elif hasattr(instance, "sdk_configuration") and hasattr(
            instance.sdk_configuration, "server_url"
        ):
            base_url = instance.sdk_configuration.server_url
        # Method 6: Check config.host pattern (common in other clients)
        elif hasattr(instance, "config") and hasattr(instance.config, "host"):
            base_url = instance.config.host

        if base_url:
            try:
                from urllib.parse import urlparse

                parsed = urlparse(str(base_url))
                if parsed.hostname:
                    server_address = parsed.hostname
                    server_port = parsed.port or (
                        443 if parsed.scheme == "https" else 80
                    )
            except Exception:
                pass

    # Set common framework span attributes using helper
    common_framework_span_attributes(
        scope,
        SemanticConvention.GEN_AI_SYSTEM_LETTA,
        server_address,
        server_port,
        environment,
        application_name,
        version,
        operation_type,  # Use operation_type as endpoint
        model_instance,
    )

    # Set Letta-specific attributes
    _set_letta_specific_attributes(span, kwargs, response, operation_type)

    # Set content attributes for chat operations
    if operation_type == "chat" and capture_message_content:
        _set_content_attributes(span, kwargs, response)

    # Calculate cost for chat operations
    if operation_type == "chat" and pricing_info and response:
        _calculate_cost(span, response, pricing_info, model_name)

    span.set_status(Status(StatusCode.OK))


def _set_letta_specific_attributes(span, kwargs, response, operation_type):
    """Set Letta-specific span attributes with comprehensive API coverage"""

    # Extract agent ID from kwargs or response
    agent_id = None
    if "agent_id" in kwargs:
        agent_id = str(kwargs["agent_id"])
    elif hasattr(response, "id"):
        agent_id = str(response.id)

    if agent_id:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_ID, agent_id)

    # Extract comprehensive agent information from response
    if response:
        # Basic agent info
        if hasattr(response, "name"):
            span.set_attribute(SemanticConvention.GEN_AI_AGENT_NAME, str(response.name))
        if hasattr(response, "slug"):
            span.set_attribute(SemanticConvention.GEN_AI_AGENT_SLUG, str(response.slug))
        if hasattr(response, "description"):
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_DESCRIPTION, str(response.description)
            )
        if hasattr(response, "agent_type"):
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_TYPE, response.agent_type
            )
        if hasattr(response, "system"):
            # Truncate long system instructions
            instructions = str(response.system)
            if len(instructions) > 2000:
                instructions = instructions[:2000] + "..."
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_INSTRUCTIONS, instructions
            )

        # Extract comprehensive LLM configuration
        if hasattr(response, "llm_config"):
            llm_config = response.llm_config
            _set_llm_config_attributes(span, llm_config)

        # Set comprehensive usage metrics for chat operations
        if operation_type == "chat" and hasattr(response, "usage"):
            usage = response.usage
            _set_usage_attributes(span, usage)

    # Extract request parameters from kwargs
    _set_request_attributes(span, kwargs, operation_type)

    # Also check kwargs for agent name/slug info
    if "name" in kwargs:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_NAME, str(kwargs["name"]))
    if "slug" in kwargs:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_SLUG, str(kwargs["slug"]))


def _set_llm_config_attributes(span, llm_config):
    """Set LLM configuration attributes from Letta llm_config using semantic conventions"""
    if not llm_config:
        return

    try:
        # Core model configuration (using semantic conventions)
        if hasattr(llm_config, "model"):
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_MODEL, llm_config.model
            )
        if hasattr(llm_config, "provider_name"):
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_PROVIDER, llm_config.provider_name
            )
        if hasattr(llm_config, "model_endpoint"):
            span.set_attribute(
                SemanticConvention.GEN_AI_ENDPOINT, llm_config.model_endpoint
            )

        # Model parameters (using semantic conventions)
        if hasattr(llm_config, "temperature") and llm_config.temperature is not None:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, llm_config.temperature
            )
        if hasattr(llm_config, "max_tokens") and llm_config.max_tokens is not None:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, llm_config.max_tokens
            )
        if (
            hasattr(llm_config, "frequency_penalty")
            and llm_config.frequency_penalty is not None
        ):
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY,
                llm_config.frequency_penalty,
            )

        # Additional configuration parameters (using semantic conventions)
        if (
            hasattr(llm_config, "context_window")
            and llm_config.context_window is not None
        ):
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_CONTEXT_WINDOW,
                llm_config.context_window,
            )
        if (
            hasattr(llm_config, "enable_reasoner")
            and llm_config.enable_reasoner is not None
        ):
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_ENABLE_REASONER,
                llm_config.enable_reasoner,
            )
        if (
            hasattr(llm_config, "reasoning_effort")
            and llm_config.reasoning_effort is not None
        ):
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_REASONING_EFFORT,
                llm_config.reasoning_effort,
            )
        if (
            hasattr(llm_config, "max_reasoning_tokens")
            and llm_config.max_reasoning_tokens is not None
        ):
            span.set_attribute(
                SemanticConvention.GEN_AI_USAGE_REASONING_TOKENS,
                llm_config.max_reasoning_tokens,
            )
        if hasattr(llm_config, "handle"):
            span.set_attribute(
                SemanticConvention.GEN_AI_MODEL_HANDLE, llm_config.handle
            )

    except Exception:
        # Ignore errors in attribute setting
        pass


def _set_usage_attributes(span, usage):
    """Set comprehensive usage attributes from Letta usage statistics"""
    if not usage:
        return

    try:
        # Token usage (using OpenTelemetry semantic conventions)
        if hasattr(usage, "prompt_tokens") and usage.prompt_tokens is not None:
            span.set_attribute(
                SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, usage.prompt_tokens
            )
        if hasattr(usage, "completion_tokens") and usage.completion_tokens is not None:
            span.set_attribute(
                SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, usage.completion_tokens
            )
        if hasattr(usage, "total_tokens") and usage.total_tokens is not None:
            span.set_attribute(
                SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS, usage.total_tokens
            )

        # Agent-specific usage metrics (using semantic conventions)
        if hasattr(usage, "step_count") and usage.step_count is not None:
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_STEP_COUNT, usage.step_count
            )
        if hasattr(usage, "run_ids") and usage.run_ids is not None:
            span.set_attribute(SemanticConvention.GEN_AI_RUN_ID, str(usage.run_ids))
        if hasattr(usage, "steps_messages") and usage.steps_messages is not None:
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_STEP_MESSAGES, str(usage.steps_messages)
            )

    except Exception:
        # Ignore errors in attribute setting
        pass


def _set_request_attributes(span, kwargs, operation_type):
    """Set request-specific attributes from kwargs"""
    try:
        # Common request parameters
        if "model" in kwargs:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_MODEL, str(kwargs["model"])
            )
        if "temperature" in kwargs:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, kwargs["temperature"]
            )
        if "max_tokens" in kwargs:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, kwargs["max_tokens"]
            )
        if "frequency_penalty" in kwargs:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY,
                kwargs["frequency_penalty"],
            )
        if "stream" in kwargs:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_IS_STREAM, kwargs["stream"]
            )

        # General request parameters (using semantic conventions)
        if "run_async" in kwargs:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_ASYNC, kwargs["run_async"]
            )
        if "return_message_sequence_no" in kwargs:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_RETURN_SEQUENCE_NO,
                kwargs["return_message_sequence_no"],
            )
        if "include_final_message" in kwargs:
            span.set_attribute(
                SemanticConvention.GEN_AI_REQUEST_INCLUDE_FINAL_MESSAGE,
                kwargs["include_final_message"],
            )

        # Message-specific attributes (using semantic conventions)
        if operation_type == "chat" and "messages" in kwargs:
            if isinstance(kwargs["messages"], list):
                span.set_attribute(
                    SemanticConvention.GEN_AI_REQUEST_MESSAGE_COUNT,
                    len(kwargs["messages"]),
                )

    except Exception:
        # Ignore errors in attribute setting
        pass


def _set_content_attributes(span, kwargs, response):
    """Set content attributes for chat operations"""
    try:
        # Input content
        if "messages" in kwargs:
            messages_str = json.dumps(str(kwargs["messages"]))
            span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, messages_str)

        # Output content
        if response and hasattr(response, "messages"):
            completion_str = str(response.messages)
            span.set_attribute(
                SemanticConvention.GEN_AI_CONTENT_COMPLETION, completion_str
            )
    except Exception:
        pass


def _calculate_cost(span, response, pricing_info, model_name):
    """Calculate and set cost attributes using comprehensive usage data"""
    try:
        usage = None

        # Extract usage from various response structures
        if hasattr(response, "usage"):
            usage = response.usage
        elif isinstance(response, list) and response:
            # Check if response is a list of messages with usage at the end
            for item in reversed(response):
                if (
                    hasattr(item, "message_type")
                    and item.message_type == "usage_statistics"
                ):
                    usage = item
                    break

        if (
            usage
            and hasattr(usage, "prompt_tokens")
            and hasattr(usage, "completion_tokens")
        ):
            # Use the model from LLM config if available, otherwise use provided model_name
            actual_model = model_name
            if hasattr(response, "llm_config") and hasattr(
                response.llm_config, "model"
            ):
                actual_model = response.llm_config.model

            cost = get_chat_model_cost(
                actual_model,
                pricing_info,
                usage.prompt_tokens,
                usage.completion_tokens,
            )
            span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)
    except Exception:
        pass


def get_span_name(operation_type, endpoint, instance=None, kwargs=None, response=None):
    """Generate proper span name following OpenTelemetry patterns"""

    # For chat operations, follow LiteLLM pattern: "chat {model}"
    if operation_type == "chat":
        model = "gpt-4o"  # Default
        if kwargs and "model" in kwargs:
            model = kwargs["model"]
        elif (
            response
            and hasattr(response, "llm_config")
            and hasattr(response.llm_config, "model")
        ):
            model = response.llm_config.model
        return f"chat {model}"

    # For agent operations, follow CrewAI pattern: "operation_type agent_name"
    elif operation_type in ["create_agent", "invoke_agent"]:
        agent_name = "agent"

        # Try to get agent name/slug from various sources with better extraction
        if response and hasattr(response, "name"):
            agent_name = response.name
        elif response and hasattr(response, "slug"):
            agent_name = response.slug
        elif kwargs and "name" in kwargs:
            agent_name = kwargs["name"]
        elif kwargs and "slug" in kwargs:
            agent_name = kwargs["slug"]
        elif instance and hasattr(instance, "name"):
            agent_name = instance.name
        elif instance and hasattr(instance, "slug"):
            agent_name = instance.slug
        elif kwargs and "agent_id" in kwargs:
            # Try to get agent name from agent_id lookup if possible
            agent_id = str(kwargs["agent_id"])
            agent_name = f"agent-{agent_id[:8]}"

        # Clean up agent name (remove spaces, special chars for better span names)
        if agent_name and agent_name != "agent":
            agent_name = agent_name.replace(" ", "_").replace("-", "_").lower()
            # Limit length for readability
            if len(agent_name) > 20:
                agent_name = agent_name[:20]

        return f"{operation_type} {agent_name}"

    # For workflow operations
    elif operation_type == "workflow":
        return f"workflow {endpoint.split('.')[-1]}"

    # For tool operations
    elif operation_type == "execute_tool":
        return f"tool {endpoint.split('.')[-1]}"

    # Default fallback
    return f"{operation_type} {endpoint.split('.')[-1]}"


class TracedLettaStream:
    """Traced streaming wrapper for Letta message operations"""

    def __init__(
        self,
        wrapped_stream,
        span,
        span_name,
        kwargs,
        operation_type,
        instance,
        start_time,
        environment,
        application_name,
        version,
        endpoint,
        capture_content,
        pricing_info,
        tracer,
    ):
        self.__wrapped__ = wrapped_stream
        self._span = span
        self._span_name = span_name
        self._kwargs = kwargs
        self._operation_type = operation_type
        self._instance = instance
        self._start_time = start_time
        self._environment = environment
        self._application_name = application_name
        self._version = version
        self._endpoint = endpoint
        self._capture_content = capture_content
        self._pricing_info = pricing_info
        self._tracer = tracer

        # Response tracking
        self._response_content = ""
        self._response_messages = []
        self._chunk_count = 0
        self._ttft = 0
        self._finalized = False

    def __iter__(self):
        return self

    def __next__(self):
        try:
            chunk = next(self.__wrapped__)
            self._process_chunk(chunk)
            return chunk
        except StopIteration:
            try:
                # Following LiteLLM pattern: create new span context for finalization
                with self._tracer.start_as_current_span(
                    self._span_name, kind=SpanKind.CLIENT
                ) as finalization_span:
                    # Process the streaming response with all collected data
                    self._process_streaming_response(finalization_span)
            except Exception as e:
                handle_exception(self._span, e)
            raise
        except Exception as e:
            handle_exception(self._span, e)
            raise

    def _process_chunk(self, chunk):
        """Process individual chunk"""
        self._chunk_count += 1

        # Calculate TTFT on first chunk
        if self._chunk_count == 1:
            self._ttft = time.time() - self._start_time

        # Accumulate response content and messages
        self._response_messages.append(chunk)

        # Extract content from various message types
        if hasattr(chunk, "content") and chunk.content:
            self._response_content += str(chunk.content)
        elif hasattr(chunk, "message") and hasattr(chunk.message, "content"):
            self._response_content += str(chunk.message.content)

        # Set streaming attributes while span is still active (using semantic conventions)
        if self._span.is_recording():
            self._span.set_attribute(
                SemanticConvention.GEN_AI_STREAMING_CHUNK_COUNT, self._chunk_count
            )
            if self._ttft > 0:
                self._span.set_attribute(
                    SemanticConvention.GEN_AI_SERVER_TTFT, self._ttft
                )

    def _process_streaming_response(self, span):
        """Process the complete streaming response (following LiteLLM pattern)"""
        try:
            # Create a synthetic response object from collected messages for processing
            # This allows us to reuse the same processing logic
            synthetic_response = type("StreamResponse", (), {})()
            synthetic_response.messages = self._response_messages

            # Extract usage from response messages if available
            for msg in self._response_messages:
                if (
                    hasattr(msg, "message_type")
                    and msg.message_type == "usage_statistics"
                ):
                    synthetic_response.usage = msg
                    break

            # Process the streaming response with all collected data
            process_letta_response(
                span,
                synthetic_response,  # Pass synthetic response with messages
                self._kwargs,
                self._operation_type,
                self._instance,
                self._start_time,
                self._environment,
                self._application_name,
                self._version,
                self._endpoint,
                self._capture_content,
                self._pricing_info,
            )

            # Set streaming-specific attributes (using semantic conventions)
            span.set_attribute(
                SemanticConvention.GEN_AI_STREAMING_CHUNK_COUNT, self._chunk_count
            )
            span.set_attribute(SemanticConvention.GEN_AI_SERVER_TTFT, self._ttft)
            span.set_attribute(SemanticConvention.GEN_AI_REQUEST_IS_STREAM, True)
            span.set_attribute(
                SemanticConvention.GEN_AI_STREAMING_RESPONSE_COUNT,
                len(self._response_messages),
            )

            # Set completion content if available
            if self._capture_content and self._response_messages:
                completion_content = str(self._response_messages)
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION, completion_content
                )
            elif self._capture_content and self._response_content:
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION, self._response_content
                )

        except Exception:
            # Ignore errors during finalization
            pass

    def _finalize_span(self):
        """Finalize span with streaming metrics"""
        if self._finalized:
            return  # Already finalized

        self._finalized = True

        try:
            # Check if span is still recording before setting attributes
            if self._span.is_recording():
                # Set streaming-specific attributes using semantic conventions
                self._span.set_attribute(
                    SemanticConvention.GEN_AI_SERVER_TTFT, self._ttft
                )
                self._span.set_attribute(
                    SemanticConvention.GEN_AI_STREAMING_CHUNK_COUNT, self._chunk_count
                )
                self._span.set_attribute(
                    SemanticConvention.GEN_AI_REQUEST_IS_STREAM, True
                )

                # Set content if enabled
                if self._capture_content and self._response_content:
                    self._span.set_attribute(
                        SemanticConvention.GEN_AI_CONTENT_COMPLETION,
                        self._response_content,
                    )
                elif self._capture_content and self._response_messages:
                    # Fallback: Use response messages if no direct content
                    completion_content = str(self._response_messages)
                    self._span.set_attribute(
                        SemanticConvention.GEN_AI_CONTENT_COMPLETION, completion_content
                    )

                # Set response messages if available
                if self._response_messages:
                    self._span.set_attribute(
                        SemanticConvention.GEN_AI_STREAMING_RESPONSE_COUNT,
                        len(self._response_messages),
                    )

                self._span.set_status(Status(StatusCode.OK))
        except Exception:
            # Ignore any errors during finalization
            pass

    def close(self):
        """Close the wrapped stream"""
        if not self._finalized:
            try:
                # Close the underlying generator properly
                if hasattr(self.__wrapped__, "close"):
                    self.__wrapped__.close()
                elif hasattr(self.__wrapped__, "__del__"):
                    try:
                        # Force generator cleanup using del
                        del self.__wrapped__
                    except Exception:
                        pass
            except Exception:
                pass  # Ignore cleanup errors
            finally:
                self._finalized = True

    def __enter__(self):
        """Context manager entry"""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit - finalize immediately"""
        if not self._finalized:
            self._finalize_span()

    def __del__(self):
        """Destructor cleanup"""
        try:
            self.close()
        except Exception:
            pass  # Ignore cleanup errors

    def __getattr__(self, name):
        return getattr(self.__wrapped__, name)


# Operation type mappings for Letta endpoints
OPERATION_TYPE_MAP = {
    # Agent operations
    "create": "create_agent",
    "retrieve": "invoke_agent",
    "modify": "invoke_agent",
    "delete": "invoke_agent",
    "list": "workflow",
    # Message operations (chat)
    "create_stream": "chat",
    "create_message": "chat",  # Renamed from duplicate "create"
    "create_async": "chat",
    "cancel": "chat",
    "reset": "chat",
    # Tool operations
    "attach": "execute_tool",
    "detach": "execute_tool",
    # Memory/Context operations - workflow
    # These remain as "workflow"
}
