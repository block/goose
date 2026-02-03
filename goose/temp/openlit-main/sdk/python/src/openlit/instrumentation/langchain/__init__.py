"""
OpenLIT LangChain Instrumentation
"""

import logging
import json
import re
import time
from typing import Any, Collection, Dict, List, Optional, Literal, cast
from uuid import UUID
import importlib.metadata

from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from opentelemetry.trace import SpanKind, Status, StatusCode

# context_api not needed - callbacks execute in different contexts
from opentelemetry.trace import set_span_in_context
from wrapt import wrap_function_wrapper

# Initialize logger
logger = logging.getLogger(__name__)

_instruments = ("langchain-core >= 0.1.20",)


class SpanHolder:
    """Container for span and related metadata with performance optimization via __slots__."""

    __slots__ = (
        "span",
        "token",
        "start_time",
        "children",
        "workflow_name",
        "entity_name",
        "streaming_content",
        "input_tokens",
        "first_token_time",
        "token_timestamps",
        "model_name",
        "model_parameters",
        "prompt_content",
    )

    def __init__(self, span, token=None, start_time=None):
        self.span = span
        self.token = token
        self.start_time = start_time or time.time()
        self.children: List[UUID] = []
        self.workflow_name = ""
        self.entity_name = ""
        self.streaming_content: List[str] = []
        self.input_tokens = 0
        self.first_token_time: Optional[float] = None
        self.token_timestamps: List[float] = []
        self.model_name = "unknown"
        self.model_parameters: Dict[str, Any] = {}
        self.prompt_content = ""

    def get_duration(self) -> float:
        """Calculate duration from start time to now."""
        return time.time() - self.start_time

    def is_streaming(self) -> bool:
        """Check if this is a streaming response."""
        return len(self.streaming_content) > 0


# =============================================================================
# Model Name Extraction (from Langfuse)
# =============================================================================

# Known model paths for specific providers
MODEL_PATHS_BY_ID = [
    # Google
    ("ChatGoogleGenerativeAI", ["kwargs", "model"], "serialized"),
    ("ChatVertexAI", ["kwargs", "model_name"], "serialized"),
    # Mistral
    ("ChatMistralAI", ["kwargs", "model"], "serialized"),
    # OpenAI
    ("OpenAI", ["invocation_params", "model_name"], "kwargs"),
    ("ChatOpenAI", ["invocation_params", "model_name"], "kwargs"),
    ("AzureChatOpenAI", ["invocation_params", "model"], "kwargs"),
    ("AzureChatOpenAI", ["invocation_params", "model_name"], "kwargs"),
    ("AzureChatOpenAI", ["invocation_params", "azure_deployment"], "kwargs"),
    # HuggingFace
    ("HuggingFacePipeline", ["invocation_params", "model_id"], "kwargs"),
    # Bedrock
    ("BedrockChat", ["kwargs", "model_id"], "serialized"),
    ("Bedrock", ["kwargs", "model_id"], "serialized"),
    ("BedrockLLM", ["kwargs", "model_id"], "serialized"),
    ("ChatBedrock", ["kwargs", "model_id"], "serialized"),
    ("ChatBedrockConverse", ["kwargs", "model_id"], "serialized"),
    # Other
    ("LlamaCpp", ["invocation_params", "model_path"], "kwargs"),
    ("WatsonxLLM", ["invocation_params", "model_id"], "kwargs"),
]

# Regex patterns for models where name is in repr string
MODEL_PATTERNS = [
    ("ChatAnthropic", "model", "anthropic"),
    ("Anthropic", "model", "anthropic"),
    ("ChatTongyi", "model_name", None),
    ("ChatCohere", "model", None),
    ("Cohere", "model", None),
    ("HuggingFaceHub", "model", None),
    ("ChatAnyscale", "model_name", None),
    ("TextGen", "model", "text-gen"),
    ("Ollama", "model", None),
    ("OllamaLLM", "model", None),
    ("ChatOllama", "model", None),
    ("ChatFireworks", "model", None),
    ("ChatPerplexity", "model", None),
    ("VLLM", "model", None),
    ("Xinference", "model_uid", None),
    ("ChatOCIGenAI", "model_id", None),
    ("DeepInfra", "model_id", None),
]

# Fallback paths to try
FALLBACK_PATHS = [
    (["kwargs", "model_name"], "serialized"),
    (["kwargs", "model"], "serialized"),
    (["kwargs", "model_id"], "serialized"),
    (["invocation_params", "model_name"], "kwargs"),
    (["invocation_params", "model"], "kwargs"),
    (["invocation_params", "model_id"], "kwargs"),
]


def _extract_by_path(
    serialized: Optional[Dict[str, Any]],
    kwargs: Dict[str, Any],
    keys: List[str],
    select_from: Literal["serialized", "kwargs"],
) -> Optional[str]:
    """Extract value from nested dict by path."""
    current_obj = kwargs if select_from == "kwargs" else serialized
    if current_obj is None:
        return None

    for key in keys:
        if isinstance(current_obj, dict):
            current_obj = current_obj.get(key)
        else:
            return None
        if current_obj is None:
            return None

    return str(current_obj) if current_obj else None


def _get_class_name(serialized: Optional[Dict[str, Any]]) -> Optional[str]:
    """Get class name from serialized id."""
    if not serialized:
        return None
    serialized_id = serialized.get("id")
    if serialized_id and isinstance(serialized_id, list) and len(serialized_id) > 0:
        return serialized_id[-1]
    return None


def _extract_model_from_repr(
    serialized: Optional[Dict[str, Any]], pattern: str
) -> Optional[str]:
    """Extract model name from repr string using regex."""
    if not serialized:
        return None
    repr_str = serialized.get("repr", "")
    if repr_str:
        match = re.search(rf"{pattern}='(.*?)'", repr_str)
        if match:
            return match.group(1)
    return None


def extract_model_name(
    serialized: Optional[Dict[str, Any]],
    kwargs: Optional[Dict[str, Any]] = None,
) -> str:
    """
    Comprehensive model name extraction (based on Langfuse's approach).

    Tries multiple strategies:
    1. Known model paths for specific providers
    2. Regex patterns from repr strings
    3. Fallback paths
    """
    if kwargs is None:
        kwargs = {}

    class_name = _get_class_name(serialized)

    # 1. Try known model paths
    for model_id, keys, select_from in MODEL_PATHS_BY_ID:
        if class_name == model_id:
            result = _extract_by_path(
                serialized,
                kwargs,
                keys,
                cast(Literal["serialized", "kwargs"], select_from),
            )
            if result:
                return result

    # 2. Try regex patterns
    for model_id, pattern, default in MODEL_PATTERNS:
        if class_name == model_id:
            result = _extract_model_from_repr(serialized, pattern)
            if result:
                return result
            if default:
                return default

    # 3. Try fallback paths
    for keys, select_from in FALLBACK_PATHS:
        result = _extract_by_path(
            serialized, kwargs, keys, cast(Literal["serialized", "kwargs"], select_from)
        )
        if result:
            return result

    # 4. Return class name or unknown
    return class_name or "unknown"


def extract_model_parameters(kwargs: Dict[str, Any]) -> Dict[str, Any]:
    """
    Extract model parameters from invocation_params (based on Langfuse's approach).
    """
    params = {}
    invocation_params = kwargs.get("invocation_params", {})

    param_keys = [
        "temperature",
        "max_tokens",
        "max_completion_tokens",
        "top_p",
        "top_k",
        "frequency_penalty",
        "presence_penalty",
        "request_timeout",
        "stop_sequences",
        "seed",
    ]

    for key in param_keys:
        value = invocation_params.get(key)
        if value is not None:
            params[key] = value

    return params


# =============================================================================
# Provider Detection
# =============================================================================

PROVIDER_MAP = {
    "anthropic": "anthropic",
    "azure": "azure",
    "bedrock": "aws.bedrock",
    "bedrock_converse": "aws.bedrock",
    "cohere": "cohere",
    "google": "google",
    "google_genai": "google",
    "google_vertexai": "google",
    "groq": "groq",
    "mistralai": "mistral_ai",
    "ollama": "ollama",
    "openai": "openai",
    "together": "together",
    "vertexai": "google",
    "fireworks": "fireworks",
    "perplexity": "perplexity",
    "huggingface": "huggingface",
    "deepinfra": "deepinfra",
    "anyscale": "anyscale",
}


def detect_provider(serialized: Optional[Dict[str, Any]]) -> str:
    """Detect the LLM provider from serialized class info."""
    from openlit.semcov import SemanticConvention

    if not serialized:
        return SemanticConvention.GEN_AI_SYSTEM_LANGCHAIN

    class_id = serialized.get("id", [])
    if isinstance(class_id, list):
        class_path = ".".join(class_id).lower()
        for provider, system in PROVIDER_MAP.items():
            if provider in class_path:
                return system

    return SemanticConvention.GEN_AI_SYSTEM_LANGCHAIN


def detect_observation_type(
    serialized: Optional[Dict[str, Any]], callback_type: str, name: str = ""
) -> str:
    """
    Detect observation type (based on Langfuse's approach).
    Distinguishes agents from regular chains.
    """
    if callback_type == "tool":
        return "tool"
    elif callback_type == "retriever":
        return "retriever"
    elif callback_type == "llm":
        return "generation"
    elif callback_type == "chain":
        # Detect if it's an agent
        if serialized and "id" in serialized:
            class_path = serialized["id"]
            if any("agent" in str(part).lower() for part in class_path):
                return "agent"

        # Check name for agent keywords
        if name and "agent" in name.lower():
            return "agent"

        return "chain"

    return "span"


# =============================================================================
# Callback Handler
# =============================================================================


def _create_callback_handler_class(
    tracer,
    version,
    environment,
    application_name,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Create and return an OpenLITCallbackHandler class configured with the given parameters.
    """
    try:
        from langchain_core.callbacks import BaseCallbackHandler
        from langchain_core.messages import BaseMessage
        from langchain_core.outputs import LLMResult
    except ImportError:
        logger.debug("langchain_core not available")
        return None

    from openlit.__helpers import (
        general_tokens,
        get_chat_model_cost,
        record_completion_metrics,
        record_framework_metrics,
        calculate_ttft,
        calculate_tbt,
    )
    from openlit.semcov import SemanticConvention

    class OpenLITCallbackHandler(BaseCallbackHandler):
        """
        OpenLIT LangChain Callback Handler with comprehensive instrumentation.
        """

        def __init__(self) -> None:
            super().__init__()
            self._tracer = tracer
            self._version = version
            self._environment = environment
            self._application_name = application_name
            self._pricing_info = pricing_info
            self._capture_message_content = capture_message_content
            self._metrics = metrics
            self._disable_metrics = disable_metrics
            self.spans: Dict[UUID, SpanHolder] = {}
            self.run_inline = True

        def _get_name_from_callback(
            self,
            serialized: Optional[Dict[str, Any]],
            **kwargs: Any,
        ) -> str:
            """Get the name to be used for the span."""
            if kwargs.get("name"):
                return kwargs["name"]
            if serialized:
                if "kwargs" in serialized and serialized["kwargs"].get("name"):
                    return serialized["kwargs"]["name"]
                if serialized.get("name"):
                    return serialized["name"]
                if "id" in serialized:
                    class_id = serialized["id"]
                    if isinstance(class_id, list) and len(class_id) > 0:
                        return class_id[-1]
            return "unknown"

        def _create_span(
            self,
            run_id: UUID,
            parent_run_id: Optional[UUID],
            span_name: str,
            kind: SpanKind = SpanKind.INTERNAL,
        ) -> Any:
            """Create a new span with proper parent relationship."""
            # For callback-based instrumentation, we manage parent-child relationships
            # via the parent_context parameter, not via context attachment.
            # This avoids "Failed to detach context" errors in streaming scenarios
            # where callbacks may execute in different execution contexts.
            parent_context = None
            if parent_run_id is not None and parent_run_id in self.spans:
                parent_span = self.spans[parent_run_id].span
                parent_context = set_span_in_context(parent_span)

            span = self._tracer.start_span(
                span_name,
                context=parent_context,
                kind=kind,
            )

            # Don't attach to context - callbacks can execute in different contexts
            holder = SpanHolder(span, token=None)
            self.spans[run_id] = holder

            if parent_run_id is not None and parent_run_id in self.spans:
                self.spans[parent_run_id].children.append(run_id)

            return span

        def _end_span(self, run_id: UUID, error: Optional[str] = None) -> None:
            """End a span and clean up."""
            if run_id not in self.spans:
                return

            holder = self.spans[run_id]
            span = holder.span

            # End any child spans that are still open
            for child_id in holder.children:
                if child_id in self.spans:
                    child_holder = self.spans[child_id]
                    if child_holder.span.end_time is None:
                        child_holder.span.end()

            if error:
                span.set_status(Status(StatusCode.ERROR, error))
            else:
                span.set_status(Status(StatusCode.OK))

            span.end()

            del self.spans[run_id]

        def _set_common_attributes(self, span, operation_type: str) -> None:
            """Set common attributes on a span."""
            span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_type)
            span.set_attribute(SemanticConvention.GEN_AI_ENVIRONMENT, self._environment)
            span.set_attribute(
                SemanticConvention.GEN_AI_APPLICATION_NAME, self._application_name
            )
            span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, self._version)

        def _extract_from_generations(
            self,
            response,
            input_tokens: int,
            output_tokens: int,
            completion_content: str,
        ) -> tuple:
            """
            Extract token usage and content from response.generations.
            Based on Langfuse's _parse_usage approach for comprehensive extraction.
            """
            if not response.generations:
                return input_tokens, output_tokens, completion_content

            for gen_list in response.generations:
                for gen in gen_list:
                    # Extract content
                    gen_content = self._extract_generation_content(gen)
                    if gen_content and (
                        not completion_content
                        or len(gen_content) > len(completion_content)
                    ):
                        completion_content = gen_content

                    # Extract usage from message
                    tokens = self._extract_message_usage(
                        gen, input_tokens, output_tokens
                    )
                    input_tokens, output_tokens = tokens

                    # Check generation_info as fallback
                    if hasattr(gen, "generation_info") and gen.generation_info:
                        gen_info = gen.generation_info
                        if "usage" in gen_info:
                            usage = gen_info["usage"]
                            input_tokens = usage.get("input_tokens", input_tokens)
                            output_tokens = usage.get("output_tokens", output_tokens)

            return input_tokens, output_tokens, completion_content

        def _extract_generation_content(self, gen) -> Optional[str]:
            """Extract content from a generation object."""
            if hasattr(gen, "text") and gen.text:
                return gen.text
            if hasattr(gen, "message") and gen.message:
                msg = gen.message
                if hasattr(msg, "content") and msg.content:
                    return (
                        msg.content
                        if isinstance(msg.content, str)
                        else str(msg.content)
                    )
            return None

        def _join_streaming_content(self, streaming_content: List[Any]) -> str:
            """
            Join streaming content into a single string.
            Handles nested lists and ensures all items are properly converted.
            """
            if not streaming_content:
                return ""

            str_parts = []
            for item in streaming_content:
                if isinstance(item, str) and item:
                    str_parts.append(item)
                elif isinstance(item, list):
                    str_parts.extend(self._flatten_list_item(item))
                elif item:
                    str_parts.append(str(item))

            return "".join(str_parts) if str_parts else ""

        def _flatten_list_item(self, items: List[Any]) -> List[str]:
            """Flatten a list of items into strings."""
            result = []
            for sub in items:
                if isinstance(sub, str) and sub:
                    result.append(sub)
                elif sub:
                    result.append(str(sub))
            return result

        def _extract_message_usage(
            self, gen, input_tokens: int, output_tokens: int
        ) -> tuple:
            """
            Extract token usage from message metadata.
            Handles multiple formats based on Langfuse's comprehensive approach:
            - usage_metadata (standard LangChain)
            - response_metadata.usage (Bedrock-Anthropic)
            - response_metadata.amazon-bedrock-invocationMetrics (Bedrock-Titan)
            """
            if not hasattr(gen, "message") or not gen.message:
                return input_tokens, output_tokens

            msg = gen.message

            # Check usage_metadata (standard LangChain, Ollama)
            if hasattr(msg, "usage_metadata") and msg.usage_metadata:
                usage = msg.usage_metadata
                input_tokens = (
                    usage.get("input_tokens")
                    or usage.get("prompt_tokens")
                    or input_tokens
                )
                output_tokens = (
                    usage.get("output_tokens")
                    or usage.get("completion_tokens")
                    or output_tokens
                )

            # Check response_metadata (Bedrock, etc.)
            if hasattr(msg, "response_metadata") and msg.response_metadata:
                metadata = msg.response_metadata
                if isinstance(metadata, dict):
                    # Bedrock-Anthropic style
                    if "usage" in metadata:
                        usage = metadata["usage"]
                        input_tokens = usage.get(
                            "inputTokens", usage.get("input_tokens", input_tokens)
                        )
                        output_tokens = usage.get(
                            "outputTokens", usage.get("output_tokens", output_tokens)
                        )

                    # Bedrock-Titan style (amazon-bedrock-invocationMetrics)
                    if "amazon-bedrock-invocationMetrics" in metadata:
                        metrics = metadata["amazon-bedrock-invocationMetrics"]
                        input_tokens = metrics.get("inputTokenCount", input_tokens)
                        output_tokens = metrics.get("outputTokenCount", output_tokens)

            return input_tokens, output_tokens

        def _set_model_parameters(self, span, params: Dict[str, Any]) -> None:
            """Set model parameters as span attributes."""
            if params.get("temperature") is not None:
                span.set_attribute(
                    SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, params["temperature"]
                )
            if params.get("max_tokens") is not None:
                span.set_attribute(
                    SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, params["max_tokens"]
                )
            if params.get("top_p") is not None:
                span.set_attribute(
                    SemanticConvention.GEN_AI_REQUEST_TOP_P, params["top_p"]
                )
            if params.get("top_k") is not None:
                span.set_attribute(
                    SemanticConvention.GEN_AI_REQUEST_TOP_K, params["top_k"]
                )
            if params.get("frequency_penalty") is not None:
                span.set_attribute(
                    SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY,
                    params["frequency_penalty"],
                )
            if params.get("presence_penalty") is not None:
                span.set_attribute(
                    SemanticConvention.GEN_AI_REQUEST_PRESENCE_PENALTY,
                    params["presence_penalty"],
                )
            if params.get("seed") is not None:
                span.set_attribute(
                    SemanticConvention.GEN_AI_REQUEST_SEED, params["seed"]
                )

        # =================================================================
        # Chain Callbacks
        # =================================================================

        def on_chain_start(
            self,
            serialized: Dict[str, Any],
            inputs: Dict[str, Any],
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            tags: Optional[List[str]] = None,
            metadata: Optional[Dict[str, Any]] = None,
            **kwargs: Any,
        ) -> None:
            """Run when chain starts."""
            try:
                name = self._get_name_from_callback(serialized, **kwargs)
                obs_type = detect_observation_type(serialized, "chain", name=name)

                # Use different operation type for agents vs chains
                if obs_type == "agent":
                    operation_type = SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT
                    span_name = f"agent {name}"
                else:
                    operation_type = SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK
                    span_name = f"workflow {name}"

                span = self._create_span(run_id, parent_run_id, span_name)

                span.set_attribute(
                    SemanticConvention.GEN_AI_SYSTEM,
                    SemanticConvention.GEN_AI_SYSTEM_LANGCHAIN,
                )
                self._set_common_attributes(span, operation_type)
                span.set_attribute(SemanticConvention.GEN_AI_WORKFLOW_TYPE, name)

                if self._capture_message_content and inputs:
                    try:
                        input_str = json.dumps(inputs, default=str)[:5000]
                    except Exception:
                        input_str = str(inputs)[:5000]
                    span.set_attribute(
                        SemanticConvention.GEN_AI_WORKFLOW_INPUT, input_str
                    )

            except Exception as e:
                logger.debug("Error in on_chain_start: %s", e)

        def on_chain_end(
            self,
            outputs: Dict[str, Any],
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            **kwargs: Any,
        ) -> None:
            """Run when chain ends."""
            try:
                if run_id not in self.spans:
                    return

                holder = self.spans[run_id]
                span = holder.span
                duration = time.time() - holder.start_time

                span.set_attribute(
                    SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, duration
                )

                if self._capture_message_content and outputs:
                    try:
                        output_str = json.dumps(outputs, default=str)[:5000]
                    except Exception:
                        output_str = str(outputs)[:5000]
                    span.set_attribute(
                        SemanticConvention.GEN_AI_WORKFLOW_OUTPUT, output_str
                    )

                # Record metrics
                if not self._disable_metrics and self._metrics:
                    try:
                        record_framework_metrics(
                            metrics=self._metrics,
                            gen_ai_operation=SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
                            gen_ai_system=SemanticConvention.GEN_AI_SYSTEM_LANGCHAIN,
                            server_address="localhost",
                            server_port=8080,
                            environment=self._environment,
                            application_name=self._application_name,
                            start_time=holder.start_time,
                            end_time=time.time(),
                        )
                    except Exception as e:
                        logger.debug("Error recording chain metrics: %s", e)

                self._end_span(run_id)

            except Exception as e:
                logger.debug("Error in on_chain_end: %s", e)

        def on_chain_error(
            self,
            error: BaseException,
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            **kwargs: Any,
        ) -> None:
            """Run when chain errors."""
            try:
                if run_id in self.spans:
                    span = self.spans[run_id].span
                    span.set_attribute(
                        SemanticConvention.GEN_AI_FRAMEWORK_ERROR_MESSAGE,
                        str(error)[:1000],
                    )
                self._end_span(run_id, str(error))
            except Exception as e:
                logger.debug("Error in on_chain_error: %s", e)

        # =================================================================
        # LLM Callbacks
        # =================================================================

        def on_llm_start(
            self,
            serialized: Dict[str, Any],
            prompts: List[str],
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            tags: Optional[List[str]] = None,
            metadata: Optional[Dict[str, Any]] = None,
            **kwargs: Any,
        ) -> None:
            """Run when LLM starts."""
            try:
                model_name = extract_model_name(serialized, kwargs)
                model_params = extract_model_parameters(kwargs)
                provider = detect_provider(serialized)
                span_name = f"chat {model_name}"

                span = self._create_span(
                    run_id, parent_run_id, span_name, SpanKind.CLIENT
                )

                span.set_attribute(SemanticConvention.GEN_AI_SYSTEM, provider)
                self._set_common_attributes(
                    span, SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT
                )
                span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, model_name)
                self._set_model_parameters(span, model_params)

                # Store model info for later
                self.spans[run_id].model_name = model_name
                self.spans[run_id].model_parameters = model_params

                if self._capture_message_content and prompts:
                    prompt_str = "\n".join(prompts)[:5000]
                    span.set_attribute(
                        SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt_str
                    )
                    self.spans[run_id].input_tokens = general_tokens(prompt_str)

            except Exception as e:
                logger.debug("Error in on_llm_start: %s", e)

        def on_chat_model_start(
            self,
            serialized: Dict[str, Any],
            messages: List[List[BaseMessage]],
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            tags: Optional[List[str]] = None,
            metadata: Optional[Dict[str, Any]] = None,
            **kwargs: Any,
        ) -> None:
            """Run when Chat Model starts."""
            try:
                model_name = extract_model_name(serialized, kwargs)
                model_params = extract_model_parameters(kwargs)
                provider = detect_provider(serialized)
                span_name = f"chat {model_name}"

                span = self._create_span(
                    run_id, parent_run_id, span_name, SpanKind.CLIENT
                )

                span.set_attribute(SemanticConvention.GEN_AI_SYSTEM, provider)
                self._set_common_attributes(
                    span, SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT
                )
                span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, model_name)
                self._set_model_parameters(span, model_params)

                # Store model info
                self.spans[run_id].model_name = model_name
                self.spans[run_id].model_parameters = model_params

                # Always calculate prompt content for token estimation
                if messages:
                    formatted = []
                    for msg_list in messages:
                        for msg in msg_list:
                            role = getattr(msg, "type", "unknown")
                            content = getattr(msg, "content", str(msg))
                            formatted.append(f"{role}: {content}")
                    prompt_str = "\n".join(formatted)[:5000]

                    # Store prompt for token estimation
                    self.spans[run_id].prompt_content = prompt_str
                    self.spans[run_id].input_tokens = general_tokens(prompt_str)

                    # Set prompt attribute if capturing content
                    if self._capture_message_content:
                        span.set_attribute(
                            SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt_str
                        )

            except Exception as e:
                logger.debug("Error in on_chat_model_start: %s", e)

        def on_llm_new_token(
            self,
            token: str,
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            chunk: Any = None,
            **kwargs: Any,
        ) -> None:
            """Run on new LLM token (streaming) - tracks TTFT."""
            try:
                if run_id in self.spans:
                    holder = self.spans[run_id]
                    current_time = time.time()

                    # Track first token time for TTFT
                    if holder.first_token_time is None:
                        holder.first_token_time = current_time

                    # Track timestamps for TBT calculation
                    holder.token_timestamps.append(current_time)

                    # Helper to extract string from content (handles list, str, etc.)
                    def extract_str(content):
                        if content is None:
                            return None
                        if isinstance(content, str):
                            return content if content else None
                        if isinstance(content, list):
                            # Flatten list of content items
                            parts = []
                            for item in content:
                                if isinstance(item, str):
                                    parts.append(item)
                                elif isinstance(item, dict) and "text" in item:
                                    parts.append(str(item["text"]))
                                elif hasattr(item, "text"):
                                    parts.append(str(item.text))
                                else:
                                    parts.append(str(item))
                            return "".join(parts) if parts else None
                        return str(content)

                    # Append the token content
                    token_str = extract_str(token)
                    if token_str:
                        holder.streaming_content.append(token_str)

                    # Also try to get content from chunk if available
                    # Some providers (Bedrock, etc.) pass content in chunk
                    if not token_str and chunk is not None:
                        chunk_content = None
                        if hasattr(chunk, "content") and chunk.content:
                            chunk_content = extract_str(chunk.content)
                        elif hasattr(chunk, "text") and chunk.text:
                            chunk_content = extract_str(chunk.text)
                        elif hasattr(chunk, "message"):
                            msg = chunk.message
                            if hasattr(msg, "content") and msg.content:
                                chunk_content = extract_str(msg.content)

                        if chunk_content:
                            holder.streaming_content.append(chunk_content)

            except Exception as e:
                logger.debug("Error in on_llm_new_token: %s", e)

        def on_llm_end(
            self,
            response: LLMResult,
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            **kwargs: Any,
        ) -> None:
            """Run when LLM ends."""
            try:
                if run_id not in self.spans:
                    return

                holder = self.spans[run_id]
                span = holder.span
                end_time = time.time()
                duration = end_time - holder.start_time

                span.set_attribute(
                    SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, duration
                )

                # Calculate TTFT and TBT
                ttft = 0.0
                tbt = 0.0
                is_streaming = len(holder.streaming_content) > 0

                if is_streaming:
                    ttft = calculate_ttft(holder.token_timestamps, holder.start_time)
                    tbt = calculate_tbt(holder.token_timestamps)
                    span.set_attribute(SemanticConvention.GEN_AI_SERVER_TTFT, ttft)
                    span.set_attribute(SemanticConvention.GEN_AI_SERVER_TBT, tbt)
                    span.set_attribute(
                        SemanticConvention.GEN_AI_REQUEST_IS_STREAM, True
                    )
                else:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_REQUEST_IS_STREAM, False
                    )

                # Extract response content
                input_tokens = holder.input_tokens
                output_tokens = 0
                completion_content = ""
                model_name = holder.model_name

                # Get completion from streaming content first
                completion_content = self._join_streaming_content(
                    holder.streaming_content
                )

                # Extract from response.generations (works for streaming and non-streaming)
                input_tokens, output_tokens, completion_content = (
                    self._extract_from_generations(
                        response, input_tokens, output_tokens, completion_content
                    )
                )

                # Try to get token usage from llm_output as another fallback
                if response.llm_output:
                    token_usage = response.llm_output.get(
                        "token_usage"
                    ) or response.llm_output.get("usage", {})
                    if token_usage:
                        input_tokens = (
                            token_usage.get("prompt_tokens")
                            or token_usage.get("input_tokens")
                            or input_tokens
                        )
                        output_tokens = (
                            token_usage.get("completion_tokens")
                            or token_usage.get("output_tokens")
                            or output_tokens
                        )

                # Calculate tokens from content if not available from metadata
                # (Similar to OpenAI instrumentation approach)
                try:
                    if output_tokens == 0 and completion_content:
                        output_tokens = general_tokens(completion_content)
                except Exception as token_err:
                    logger.debug("Error calculating output tokens: %s", token_err)

                # If we still don't have input tokens, estimate from prompt
                try:
                    if (
                        input_tokens == 0
                        and hasattr(holder, "prompt_content")
                        and holder.prompt_content
                    ):
                        input_tokens = general_tokens(holder.prompt_content)
                except Exception as input_err:
                    logger.debug("Error calculating input tokens: %s", input_err)

                # Set completion content attribute
                if completion_content and self._capture_message_content:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_CONTENT_COMPLETION,
                        completion_content[:5000],
                    )

                # Always set token attributes
                span.set_attribute(
                    SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, input_tokens
                )
                span.set_attribute(
                    SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, output_tokens
                )
                span.set_attribute(
                    SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS,
                    input_tokens + output_tokens,
                )

                # Calculate cost
                cost = get_chat_model_cost(
                    model_name, self._pricing_info, input_tokens, output_tokens
                )
                span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)

                # Set response model
                response_model = model_name
                if response.llm_output:
                    response_model = response.llm_output.get(
                        "model_name"
                    ) or response.llm_output.get("model", model_name)
                    span.set_attribute(
                        SemanticConvention.GEN_AI_RESPONSE_MODEL, response_model
                    )

                # Record metrics (like OpenAI instrumentation)
                if not self._disable_metrics and self._metrics:
                    try:
                        record_completion_metrics(
                            metrics=self._metrics,
                            gen_ai_operation=SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
                            gen_ai_system=SemanticConvention.GEN_AI_SYSTEM_LANGCHAIN,
                            server_address="localhost",
                            server_port=8080,
                            request_model=model_name,
                            response_model=response_model,
                            environment=self._environment,
                            application_name=self._application_name,
                            start_time=holder.start_time,
                            end_time=end_time,
                            input_tokens=input_tokens,
                            output_tokens=output_tokens,
                            cost=cost,
                            tbt=tbt,
                            ttft=ttft,
                        )
                    except Exception as e:
                        logger.debug("Error recording LLM metrics: %s", e)

                self._end_span(run_id)

            except Exception as e:
                logger.debug("Error in on_llm_end: %s", e)

        def on_llm_error(
            self,
            error: BaseException,
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            **kwargs: Any,
        ) -> None:
            """Run when LLM errors."""
            try:
                if run_id in self.spans:
                    span = self.spans[run_id].span
                    span.set_attribute(
                        SemanticConvention.GEN_AI_FRAMEWORK_ERROR_MESSAGE,
                        str(error)[:1000],
                    )
                self._end_span(run_id, str(error))
            except Exception as e:
                logger.debug("Error in on_llm_error: %s", e)

        # =================================================================
        # Tool Callbacks
        # =================================================================

        def on_tool_start(
            self,
            serialized: Dict[str, Any],
            input_str: str,
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            tags: Optional[List[str]] = None,
            metadata: Optional[Dict[str, Any]] = None,
            **kwargs: Any,
        ) -> None:
            """Run when tool starts."""
            try:
                name = self._get_name_from_callback(serialized, **kwargs)
                span_name = f"tool {name}"

                span = self._create_span(run_id, parent_run_id, span_name)

                span.set_attribute(
                    SemanticConvention.GEN_AI_SYSTEM,
                    SemanticConvention.GEN_AI_SYSTEM_LANGCHAIN,
                )
                self._set_common_attributes(
                    span, SemanticConvention.GEN_AI_OPERATION_TYPE_TOOLS
                )
                span.set_attribute(SemanticConvention.GEN_AI_TOOL_NAME, name)

                if self._capture_message_content and input_str:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_TOOL_INPUT, str(input_str)[:5000]
                    )

            except Exception as e:
                logger.debug("Error in on_tool_start: %s", e)

        def on_tool_end(
            self,
            output: str,
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            **kwargs: Any,
        ) -> None:
            """Run when tool ends."""
            try:
                if run_id not in self.spans:
                    return

                holder = self.spans[run_id]
                span = holder.span
                duration = time.time() - holder.start_time

                span.set_attribute(
                    SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, duration
                )

                if self._capture_message_content and output:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_TOOL_OUTPUT, str(output)[:5000]
                    )

                # Record metrics
                if not self._disable_metrics and self._metrics:
                    try:
                        record_framework_metrics(
                            metrics=self._metrics,
                            gen_ai_operation=SemanticConvention.GEN_AI_OPERATION_TYPE_TOOLS,
                            gen_ai_system=SemanticConvention.GEN_AI_SYSTEM_LANGCHAIN,
                            server_address="localhost",
                            server_port=8080,
                            environment=self._environment,
                            application_name=self._application_name,
                            start_time=holder.start_time,
                            end_time=time.time(),
                        )
                    except Exception as e:
                        logger.debug("Error recording tool metrics: %s", e)

                self._end_span(run_id)

            except Exception as e:
                logger.debug("Error in on_tool_end: %s", e)

        def on_tool_error(
            self,
            error: BaseException,
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            **kwargs: Any,
        ) -> None:
            """Run when tool errors."""
            try:
                if run_id in self.spans:
                    span = self.spans[run_id].span
                    span.set_attribute(
                        SemanticConvention.GEN_AI_FRAMEWORK_ERROR_MESSAGE,
                        str(error)[:1000],
                    )
                self._end_span(run_id, str(error))
            except Exception as e:
                logger.debug("Error in on_tool_error: %s", e)

        # =================================================================
        # Retriever Callbacks
        # =================================================================

        def on_retriever_start(
            self,
            serialized: Dict[str, Any],
            query: str,
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            tags: Optional[List[str]] = None,
            metadata: Optional[Dict[str, Any]] = None,
            **kwargs: Any,
        ) -> None:
            """Run when retriever starts."""
            try:
                name = self._get_name_from_callback(serialized, **kwargs)
                span_name = f"retrieval {name}"

                span = self._create_span(run_id, parent_run_id, span_name)

                span.set_attribute(
                    SemanticConvention.GEN_AI_SYSTEM,
                    SemanticConvention.GEN_AI_SYSTEM_LANGCHAIN,
                )
                self._set_common_attributes(
                    span, SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE
                )

                if self._capture_message_content and query:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_RETRIEVAL_QUERY, str(query)[:5000]
                    )

            except Exception as e:
                logger.debug("Error in on_retriever_start: %s", e)

        def on_retriever_end(
            self,
            documents: List[Any],
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            **kwargs: Any,
        ) -> None:
            """Run when retriever ends."""
            try:
                if run_id not in self.spans:
                    return

                holder = self.spans[run_id]
                span = holder.span
                duration = time.time() - holder.start_time

                span.set_attribute(
                    SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, duration
                )

                span.set_attribute(
                    SemanticConvention.GEN_AI_RETRIEVAL_DOCUMENT_COUNT, len(documents)
                )

                if self._capture_message_content and documents:
                    sample_docs = []
                    for doc in documents[:3]:
                        content = getattr(doc, "page_content", str(doc))
                        sample_docs.append(content[:500])
                    span.set_attribute(
                        SemanticConvention.GEN_AI_RETRIEVAL_DOCUMENTS,
                        "; ".join(sample_docs),
                    )

                # Record metrics
                if not self._disable_metrics and self._metrics:
                    try:
                        record_framework_metrics(
                            metrics=self._metrics,
                            gen_ai_operation=SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
                            gen_ai_system=SemanticConvention.GEN_AI_SYSTEM_LANGCHAIN,
                            server_address="localhost",
                            server_port=8080,
                            environment=self._environment,
                            application_name=self._application_name,
                            start_time=holder.start_time,
                            end_time=time.time(),
                        )
                    except Exception as e:
                        logger.debug("Error recording retriever metrics: %s", e)

                self._end_span(run_id)

            except Exception as e:
                logger.debug("Error in on_retriever_end: %s", e)

        def on_retriever_error(
            self,
            error: BaseException,
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            **kwargs: Any,
        ) -> None:
            """Run when retriever errors."""
            try:
                if run_id in self.spans:
                    span = self.spans[run_id].span
                    span.set_attribute(
                        SemanticConvention.GEN_AI_FRAMEWORK_ERROR_MESSAGE,
                        str(error)[:1000],
                    )
                self._end_span(run_id, str(error))
            except Exception as e:
                logger.debug("Error in on_retriever_error: %s", e)

        # =================================================================
        # Agent Callbacks
        # =================================================================

        def on_agent_action(
            self,
            action: Any,
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            **kwargs: Any,
        ) -> None:
            """Run on agent action."""
            try:
                if run_id in self.spans:
                    span = self.spans[run_id].span
                    span.set_attribute(
                        SemanticConvention.GEN_AI_OPERATION,
                        SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT,
                    )
                    if self._capture_message_content:
                        # Extract tool and input from action
                        tool = getattr(action, "tool", str(action))
                        tool_input = getattr(action, "tool_input", "")
                        log = getattr(action, "log", "")
                        span.set_attribute(
                            SemanticConvention.GEN_AI_AGENT_ACTION_TOOL,
                            str(tool)[:1000],
                        )
                        span.set_attribute(
                            SemanticConvention.GEN_AI_AGENT_ACTION_TOOL_INPUT,
                            str(tool_input)[:5000],
                        )
                        if log:
                            span.set_attribute(
                                SemanticConvention.GEN_AI_AGENT_ACTION_LOG,
                                str(log)[:5000],
                            )
            except Exception as e:
                logger.debug("Error in on_agent_action: %s", e)

        def on_agent_finish(
            self,
            finish: Any,
            *,
            run_id: UUID,
            parent_run_id: Optional[UUID] = None,
            **kwargs: Any,
        ) -> None:
            """Run on agent finish."""
            try:
                if run_id in self.spans:
                    span = self.spans[run_id].span
                    if self._capture_message_content:
                        # Extract output and log from finish
                        output = getattr(finish, "return_values", str(finish))
                        log = getattr(finish, "log", "")
                        span.set_attribute(
                            SemanticConvention.GEN_AI_AGENT_FINISH_OUTPUT,
                            str(output)[:5000],
                        )
                        if log:
                            span.set_attribute(
                                SemanticConvention.GEN_AI_AGENT_FINISH_LOG,
                                str(log)[:5000],
                            )
            except Exception as e:
                logger.debug("Error in on_agent_finish: %s", e)

    return OpenLITCallbackHandler


# =============================================================================
# Callback Manager Wrapper (OpenLLMetry approach)
# =============================================================================


class _BaseCallbackManagerInitWrapper:
    """
    Wrapper to inject OpenLIT callback handler via BaseCallbackManager.__init__.

    This is the same approach used by OpenLLMetry (Traceloop).
    """

    def __init__(self, callback_handler):
        self._callback_handler = callback_handler

    def get_handler(self):
        """Return the callback handler instance."""
        return self._callback_handler

    def is_handler_present(self, handlers) -> bool:
        """Check if OpenLIT handler is already present in handlers list."""
        return any(isinstance(h, type(self._callback_handler)) for h in handlers)

    def __call__(self, wrapped, instance, args, kwargs) -> None:
        wrapped(*args, **kwargs)

        try:
            if self.is_handler_present(instance.inheritable_handlers):
                return

            instance.add_handler(self._callback_handler, True)
        except Exception as e:
            logger.debug("Error adding callback handler: %s", e)


# =============================================================================
# Instrumentor
# =============================================================================


class LangChainInstrumentor(BaseInstrumentor):
    """
    OpenLIT LangChain instrumentor with comprehensive features.

    Features from:
    - OpenLLMetry: Auto-injection via BaseCallbackManager.__init__
    - Langfuse: Model name extraction, parameter tracking, agent detection
    - OpenInference: Thread-safe span tracking
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        """Instrument LangChain."""
        try:
            version = importlib.metadata.version("langchain-core")
        except importlib.metadata.PackageNotFoundError:
            version = "unknown"

        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics", False)

        handler_class = _create_callback_handler_class(
            tracer,
            version,
            environment,
            application_name,
            pricing_info,
            capture_message_content,
            metrics,
            disable_metrics,
        )

        if handler_class is None:
            logger.debug("Could not create LangChain callback handler class")
            return

        handler_instance = handler_class()

        try:
            wrap_function_wrapper(
                module="langchain_core.callbacks.manager",
                name="BaseCallbackManager.__init__",
                wrapper=_BaseCallbackManagerInitWrapper(handler_instance),
            )
            logger.debug("Successfully wrapped BaseCallbackManager.__init__")
        except Exception as e:
            logger.debug("Failed to wrap BaseCallbackManager.__init__: %s", e)

    def _uninstrument(self, **kwargs):
        """Remove instrumentation."""
        try:
            from opentelemetry.instrumentation.utils import unwrap

            unwrap("langchain_core.callbacks.manager", "BaseCallbackManager.__init__")
        except Exception as e:
            logger.debug("Error during uninstrumentation: %s", e)
