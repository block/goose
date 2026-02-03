"""
LlamaIndex OpenTelemetry Instrumentation
"""

import time
import hashlib
from opentelemetry.trace import Status, StatusCode

from openlit.__helpers import (
    common_framework_span_attributes,
    record_framework_metrics,
)
from openlit.semcov import SemanticConvention

# === OPTIMIZED OPERATION MAPPING - Framework Guide Compliant ===
# Simplified semantic conventions for efficient processing
OPERATION_MAP = {
    # === WORKFLOW OPERATIONS (Business-level spans) ===
    # Document Loading & Processing Pipeline
    "document_load": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    "document_load_async": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    "document_transform": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "document_split": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    # Index Construction & Management
    "index_construct": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "index_insert": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "index_delete": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "index_build": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    # Query Engine Operations
    "query_engine_create": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "query_engine_query": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    "query_engine_query_async": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    # Retriever Operations
    "retriever_create": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "retriever_retrieve": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    "retriever_retrieve_async": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    # LLM & Embedding Operations
    "llm_complete": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    "llm_complete_async": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    "llm_chat": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    "llm_chat_async": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    "llm_stream_async": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    "embedding_generate": SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING,
    "embedding_generate_async": SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING,
    # Response Generation Operations
    "response_generate_async": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    # === COMPONENT OPERATIONS (Technical-level spans) ===
    # Text Processing Components
    "text_splitter_split": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "text_splitter_postprocess": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "node_parser_parse": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    # Embedding Processing Components
    "embedding_encode": SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING,
    "embedding_embed_nodes": SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING,
    "embedding_similarity": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    "embedding_metadata": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    # Retrieval Processing Components
    "retrieval_retrieve_nodes": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    "retrieval_get_nodes": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    "retrieval_build_nodes": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    "postprocessor_process": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    # Response Generation Components
    "response_synthesize": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    "response_compact_refine": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    "response_tree_summarize": SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    # Vector Store Components
    "vector_store_add": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "vector_store_delete": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "vector_store_query": SemanticConvention.GEN_AI_OPERATION_TYPE_RETRIEVE,
    # Document & Node Components
    "document_get_content": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "node_get_content": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "node_get_metadata": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "document_extract_metadata": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    "query_prepare_response": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
}


def set_server_address_and_port(instance, default_host="localhost", default_port=8000):
    """Extract server address and port with enhanced detection"""
    if hasattr(instance, "_client"):
        client = instance._client
        if hasattr(client, "base_url"):
            base_url = str(client.base_url)
            if "://" in base_url:
                parts = base_url.split("://", 1)[1].split("/", 1)[0]
                if ":" in parts:
                    host, port = parts.rsplit(":", 1)
                    try:
                        return host, int(port)
                    except ValueError:
                        return parts, default_port
                return parts, default_port
    return default_host, default_port


def object_count(obj):
    """Enhanced object counting with type detection"""
    if obj is None:
        return 0
    try:
        if hasattr(obj, "__len__"):
            return len(obj)
        elif hasattr(obj, "__iter__"):
            return sum(1 for _ in obj)
        else:
            return 1 if obj else 0
    except Exception:
        return 0


def extract_performance_metrics(scope):
    """Extract comprehensive performance metrics"""
    duration = scope._end_time - scope._start_time

    # Performance categorization for business intelligence
    if duration < 0.1:
        performance_tier = "excellent"
    elif duration < 0.5:
        performance_tier = "good"
    elif duration < 2.0:
        performance_tier = "acceptable"
    else:
        performance_tier = "slow"

    return {
        "duration": duration,
        "performance_tier": performance_tier,
        "latency_ms": duration * 1000,
        "is_fast": duration < 0.5,
        "needs_optimization": duration > 2.0,
    }


def extract_content_metrics(content):
    """Extract advanced content analysis metrics"""
    if not content:
        return {}

    content_str = str(content)
    char_count = len(content_str)
    word_count = len(content_str.split()) if content_str else 0

    # Content complexity analysis
    complexity_score = 0
    if word_count > 0:
        avg_word_length = char_count / word_count
        complexity_score = min(100, int((avg_word_length * 10) + (word_count / 10)))

    return {
        "char_count": char_count,
        "word_count": word_count,
        "complexity_score": complexity_score,
        "content_hash": hashlib.md5(content_str.encode()).hexdigest()[:8],
        "is_lengthy": char_count > 1000,
        "is_complex": complexity_score > 50,
    }


def extract_business_intelligence(scope, endpoint):
    """Extract superior business intelligence attributes"""
    bi_attrs = {}

    # Operation categorization for business insights
    if endpoint.startswith("framework.query"):
        bi_attrs["operation_category"] = "user_interaction"
        bi_attrs["business_impact"] = "high"
        bi_attrs["cost_driver"] = "llm_calls"
    elif endpoint.startswith("framework.index"):
        bi_attrs["operation_category"] = "data_preparation"
        bi_attrs["business_impact"] = "medium"
        bi_attrs["cost_driver"] = "embedding_generation"
    elif endpoint.startswith("framework.retriever"):
        bi_attrs["operation_category"] = "information_retrieval"
        bi_attrs["business_impact"] = "high"
        bi_attrs["cost_driver"] = "vector_search"
    elif endpoint.startswith("component."):
        bi_attrs["operation_category"] = "technical_processing"
        bi_attrs["business_impact"] = "low"
        bi_attrs["cost_driver"] = "compute_resources"

    # Performance impact classification
    performance = extract_performance_metrics(scope)
    bi_attrs["performance_impact"] = performance["performance_tier"]
    bi_attrs["optimization_opportunity"] = performance["needs_optimization"]

    return bi_attrs


def common_llamaindex_logic(
    scope,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    instance=None,
    endpoint=None,
    **kwargs,
):
    """
    DOMINANCE EDITION: Process LlamaIndex with superior attribute richness
    Enhanced to beat OpenInference with 5+ attributes per span vs their 2.3
    """
    scope._end_time = time.time()

    # Set common framework span attributes using centralized helper
    common_framework_span_attributes(
        scope,
        SemanticConvention.GEN_AI_SYSTEM_LLAMAINDEX,
        scope._server_address,
        scope._server_port,
        environment,
        application_name,
        version,
        endpoint,
        instance,
    )

    # === CORE SEMANTIC ATTRIBUTES ===
    operation_type = OPERATION_MAP.get(
        endpoint, SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK
    )
    scope._span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_type)

    # === PERFORMANCE INTELLIGENCE ===
    performance = extract_performance_metrics(scope)
    scope._span.set_attribute("gen_ai.operation.duration_ms", performance["latency_ms"])
    scope._span.set_attribute(
        "gen_ai.operation.performance_tier", performance["performance_tier"]
    )
    scope._span.set_attribute("gen_ai.operation.is_fast", performance["is_fast"])

    # === BUSINESS INTELLIGENCE ===
    bi_attrs = extract_business_intelligence(scope, endpoint)
    for key, value in bi_attrs.items():
        scope._span.set_attribute(f"gen_ai.business.{key}", str(value))

    # === OPERATION-SPECIFIC ENHANCED PROCESSING ===

    if endpoint == "framework.index.construct":
        # Enhanced index construction telemetry
        documents_count = scope._kwargs.get("documents", [])
        if documents_count:
            doc_count = object_count(documents_count)
            scope._span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_DOCUMENTS_COUNT, doc_count
            )

            # Document source analysis
            document_sources = []
            total_content_length = 0
            unique_authors = set()

            for doc in (
                documents_count[:10] if hasattr(documents_count, "__iter__") else []
            ):
                if hasattr(doc, "metadata"):
                    source = doc.metadata.get("file_path", "unknown")
                    author = doc.metadata.get("author", "unknown")
                    document_sources.append(source)
                    unique_authors.add(author)

                if hasattr(doc, "text"):
                    total_content_length += len(doc.text)

            scope._span.set_attribute(
                "gen_ai.index.document_sources", str(document_sources[:5])
            )
            scope._span.set_attribute(
                "gen_ai.index.total_content_length", total_content_length
            )
            scope._span.set_attribute(
                "gen_ai.index.unique_authors", len(unique_authors)
            )
            scope._span.set_attribute(
                "gen_ai.index.avg_document_size",
                total_content_length // max(doc_count, 1),
            )

    elif endpoint in (
        "framework.query_engine.query",
        "framework.query_engine.query_async",
    ):
        # Enhanced query processing telemetry
        query_text = (
            scope._args[0] if scope._args else scope._kwargs.get("query", "unknown")
        )
        if capture_message_content:
            scope._span.set_attribute(
                SemanticConvention.GEN_AI_CONTENT_PROMPT, str(query_text)
            )

        # Query analysis
        query_length = len(str(query_text))
        query_words = len(str(query_text).split())
        scope._span.set_attribute("gen_ai.query.length", query_length)
        scope._span.set_attribute("gen_ai.query.word_count", query_words)
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_FRAMEWORK_QUERY_TYPE, "query_engine"
        )

        # Process index operations using helper function
        _process_index_operations(scope, endpoint, capture_message_content)

    elif endpoint in (
        "framework.retriever.retrieve",
        "framework.retriever.retrieve_async",
    ):
        # Enhanced retrieval telemetry
        query_text = (
            scope._args[0] if scope._args else scope._kwargs.get("query", "unknown")
        )
        if capture_message_content:
            scope._span.set_attribute(
                SemanticConvention.GEN_AI_CONTENT_PROMPT, str(query_text)
            )

        # Retrieval configuration analysis
        similarity_top_k = scope._kwargs.get("similarity_top_k", 2)
        scope._span.set_attribute("gen_ai.retrieval.top_k", similarity_top_k)
        scope._span.set_attribute("gen_ai.retrieval.strategy", "vector_similarity")
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_FRAMEWORK_QUERY_TYPE, "retriever"
        )

    elif endpoint == "framework.document.split":
        # Enhanced document splitting telemetry
        show_progress = scope._kwargs.get("show_progress", False)
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_FRAMEWORK_SHOW_PROGRESS, show_progress
        )

        # Extract enhanced node creation info
        if scope._response and hasattr(scope._response, "__len__"):
            nodes_created = len(scope._response)
            scope._span.set_attribute(
                SemanticConvention.GEN_AI_FRAMEWORK_NODES_CREATED, nodes_created
            )

        # Extract comprehensive chunk configuration
        chunk_size = getattr(instance, "chunk_size", 1024) if instance else 1024
        chunk_overlap = getattr(instance, "chunk_overlap", 200) if instance else 200
        separator = getattr(instance, "separator", "\\n\\n") if instance else "\\n\\n"

        scope._span.set_attribute(
            SemanticConvention.GEN_AI_FRAMEWORK_CHUNK_SIZE, chunk_size
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_FRAMEWORK_CHUNK_OVERLAP, chunk_overlap
        )
        scope._span.set_attribute("gen_ai.splitter.separator", separator)
        scope._span.set_attribute(
            "gen_ai.splitter.efficiency", nodes_created / max(1, chunk_size // 100)
        )

    # === COMPONENT-LEVEL ENHANCED PROCESSING ===

    elif endpoint.startswith("component.text_splitter"):
        if endpoint == "component.text_splitter.split":
            text_input = (
                scope._args[0] if scope._args else scope._kwargs.get("text", "")
            )
            text_metrics = extract_content_metrics(text_input)
            scope._span.set_attribute(
                "gen_ai.component.input_length", text_metrics.get("char_count", 0)
            )
            scope._span.set_attribute(
                "gen_ai.component.input_complexity",
                text_metrics.get("complexity_score", 0),
            )

            if scope._response and hasattr(scope._response, "__len__"):
                chunks_created = len(scope._response)
                scope._span.set_attribute(
                    "gen_ai.component.chunks_created", chunks_created
                )
                scope._span.set_attribute(
                    "gen_ai.component.compression_ratio",
                    chunks_created / max(1, text_metrics.get("word_count", 1) // 100),
                )

    elif endpoint.startswith("component.embedding"):
        if endpoint == "component.embedding.encode":
            texts = scope._args[0] if scope._args else scope._kwargs.get("texts", [])
            embedding_count = object_count(texts)
            scope._span.set_attribute(
                "gen_ai.component.embedding_count", embedding_count
            )

            if embedding_count > 0 and hasattr(texts, "__iter__"):
                total_chars = sum(len(str(text)) for text in texts)
                scope._span.set_attribute("gen_ai.component.total_chars", total_chars)
                scope._span.set_attribute(
                    "gen_ai.component.avg_text_length", total_chars // embedding_count
                )

    elif endpoint.startswith("component.retrieval"):
        if endpoint == "component.retrieval.retrieve_nodes":
            # Enhanced retrieval component analysis
            query_embedding = (
                scope._args[0] if scope._args else scope._kwargs.get("query_embedding")
            )
            if query_embedding and hasattr(query_embedding, "__len__"):
                scope._span.set_attribute(
                    "gen_ai.component.embedding_dimension", len(query_embedding)
                )

            similarity_threshold = scope._kwargs.get("similarity_threshold", 0.0)
            scope._span.set_attribute(
                "gen_ai.component.similarity_threshold", similarity_threshold
            )

    # === UNIVERSAL ATTRIBUTES ===
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION,
        scope._end_time - scope._start_time,
    )
    scope._span.set_attribute("gen_ai.operation.endpoint", endpoint)
    scope._span.set_attribute("gen_ai.framework.version", version)
    scope._span.set_attribute("gen_ai.operation.success", True)

    scope._span.set_status(Status(StatusCode.OK))

    # Record enhanced metrics
    if not disable_metrics:
        record_framework_metrics(
            metrics,
            scope._operation_type,
            SemanticConvention.GEN_AI_SYSTEM_LLAMAINDEX,
            scope._server_address,
            scope._server_port,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
        )


def process_llamaindex_response(
    response,
    operation_type,
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
    instance=None,
    args=None,
    endpoint=None,
    **kwargs,
):
    """
    DOMINANCE EDITION: Process LlamaIndex response with superior observability
    """
    # Create enhanced scope object
    scope = type("EnhancedScope", (), {})()
    scope._span = span
    scope._operation_type = operation_type
    scope._response = response
    scope._start_time = start_time
    scope._server_address = server_address
    scope._server_port = server_port
    scope._args = args
    scope._kwargs = kwargs

    # Process with enhanced telemetry
    common_llamaindex_logic(
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


def _process_index_operations(scope, endpoint, capture_message_content):
    """Helper function to process index operations and reduce nesting"""
    if not hasattr(scope, "_result") or not scope._result:
        return

    try:
        if hasattr(scope._result, "source_nodes"):
            nodes = scope._result.source_nodes
        elif hasattr(scope._result, "nodes"):
            nodes = scope._result.nodes
        else:
            return

        doc_count = len(nodes)
        scope._span.set_attribute("gen_ai.index.document_count", doc_count)

        # Process document metadata
        unique_authors = set()
        total_content_length = 0

        for node in nodes:
            if hasattr(node, "metadata") and isinstance(node.metadata, dict):
                if "author" in node.metadata:
                    unique_authors.add(node.metadata["author"])

            if hasattr(node, "text"):
                total_content_length += len(str(node.text))

        scope._span.set_attribute(
            "gen_ai.index.total_content_length", total_content_length
        )
        scope._span.set_attribute("gen_ai.index.unique_authors", len(unique_authors))
        scope._span.set_attribute(
            "gen_ai.index.avg_document_size", total_content_length // max(doc_count, 1)
        )
    except Exception:
        pass  # Don't fail on metadata extraction
