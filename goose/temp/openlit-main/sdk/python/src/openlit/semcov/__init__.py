# pylint: disable=too-few-public-methods
"""
This module defines the `SemanticConvention` class which encapsulates various constants used
for semantic tagging within a generalized AI application context. These constants are
intended for use across different components of AI applications, including request handling,
response processing, usage metrics, and interaction with vector databases and AI systems.
The purpose is to standardize the semantics for easier integration, analytics, and maintenance.
"""


class SemanticConvention:
    """
    The SemanticConvention class provides a centralized repository of constant values that
    represent the keys for various semantic conventions within AI applications. These
    conventions cover a broad range of areas including general AI configurations, request
    parameters, usage metrics, response attributes, and integrations with external AI and
    database systems. It is designed to facilitate consistency and understandability across
    the application's data logging and processing functionalities.
    """

    # General Attributes (OTel Semconv)
    SERVER_PORT = "server.port"
    SERVER_ADDRESS = "server.address"
    ERROR_TYPE = "error.type"

    # GenAI Metric Names (OTel Semconv)
    GEN_AI_CLIENT_TOKEN_USAGE = "gen_ai.client.token.usage"
    DB_CLIENT_TOKEN_USAGE = "db.client.token.usage"
    GEN_AI_CLIENT_OPERATION_DURATION = "gen_ai.client.operation.duration"
    GEN_AI_SERVER_REQUEST_DURATION = "gen_ai.server.request.duration"
    GEN_AI_SERVER_TBT = "gen_ai.server.time_per_output_token"
    GEN_AI_SERVER_TTFT = "gen_ai.server.time_to_first_token"

    # GenAI Event Names (OTel Semconv)
    GEN_AI_USER_MESSAGE = "gen_ai.user.message"
    GEN_AI_SYSTEM_MESSAGE = "gen_ai.system.message"
    GEN_AI_ASSISTANT_MESSAGE = "gen_ai.assistant.message"
    GEN_AI_TOOL_MESSAGE = "gen_ai.tools.message"
    GEN_AI_CHOICE = "gen_ai.choice"

    # GenAI Request Attributes (OTel Semconv)
    GEN_AI_OPERATION = "gen_ai.operation.name"
    GEN_AI_SYSTEM = "gen_ai.system"
    GEN_AI_OUTPUT_TYPE = "gen_ai.output.type"
    GEN_AI_ENDPOINT = "gen_ai.endpoint"
    GEN_AI_REQUEST_MODEL = "gen_ai.request.model"
    GEN_AI_REQUEST_SEED = "gen_ai.request.seed"
    GEN_AI_REQUEST_ENCODING_FORMATS = "gen_ai.request.encoding_formats"
    GEN_AI_REQUEST_FREQUENCY_PENALTY = "gen_ai.request.frequency_penalty"
    GEN_AI_REQUEST_MAX_TOKENS = "gen_ai.request.max_tokens"
    GEN_AI_REQUEST_PRESENCE_PENALTY = "gen_ai.request.presence_penalty"
    GEN_AI_REQUEST_STOP_SEQUENCES = "gen_ai.request.stop_sequences"
    GEN_AI_REQUEST_TEMPERATURE = "gen_ai.request.temperature"
    GEN_AI_REQUEST_TOP_K = "gen_ai.request.top_k"
    GEN_AI_REQUEST_TOP_P = "gen_ai.request.top_p"

    # GenAI Response Attributes (OTel Semconv)
    GEN_AI_TOKEN_TYPE = "gen_ai.token.type"
    GEN_AI_RESPONSE_FINISH_REASON = "gen_ai.response.finish_reasons"
    GEN_AI_RESPONSE_ID = "gen_ai.response.id"
    GEN_AI_RESPONSE_MODEL = "gen_ai.response.model"

    GEN_AI_USAGE_INPUT_TOKENS = "gen_ai.usage.input_tokens"
    GEN_AI_USAGE_OUTPUT_TOKENS = "gen_ai.usage.output_tokens"
    GEN_AI_USAGE_REASONING_TOKENS = "gen_ai.usage.reasoning_tokens"
    GEN_AI_USAGE_READ_UNITS = "gen_ai.usage.read_units"
    GEN_AI_USAGE_RERANK_UNITS = "gen_ai.usage.rerank_units"
    GEN_AI_TOOL_CALL_ID = "gen_ai.tool.call.id"
    GEN_AI_TOOL_NAME = "gen_ai.tool.name"
    GEN_AI_TOOL_ARGS = "gen_ai.tool.args"

    # GenAI Operation Types (OTel Semconv)
    GEN_AI_OPERATION_TYPE_TEXT_COMPLETION = "text_completion"
    GEN_AI_OPERATION_TYPE_CHAT = "chat"
    GEN_AI_OPERATION_TYPE_TOOLS = "execute_tool"
    GEN_AI_OPERATION_TYPE_EMBEDDING = "embeddings"
    GEN_AI_OPERATION_TYPE_IMAGE = "image"
    GEN_AI_OPERATION_TYPE_AUDIO = "audio"
    GEN_AI_OPERATION_TYPE_TRANSLATE = "translate"
    GEN_AI_OPERATION_TYPE_SPEECH_TO_TEXT = "speech_to_text"
    GEN_AI_OPERATION_TYPE_TEXT_TO_SPEECH = "text_to_speech"
    GEN_AI_OPERATION_TYPE_TRANSLITERATE = "transliterate"
    GEN_AI_OPERATION_TYPE_LANGUAGE_IDENTIFICATION = "language_identification"
    GEN_AI_OPERATION_TYPE_SPEECH_TO_TEXT_TRANSLATE = "speech_to_text_translate"
    GEN_AI_OPERATION_TYPE_VECTORDB = "vectordb"
    GEN_AI_OPERATION_TYPE_FRAMEWORK = "workflow"
    GEN_AI_OPERATION_TYPE_AGENT = "invoke_agent"
    GEN_AI_OPERATION_TYPE_TEAM = "team"
    GEN_AI_OPERATION_TYPE_CREATE_AGENT = "create_agent"
    GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK = "execute_task"
    GEN_AI_OPERATION_TYPE_GRAPH_EXECUTION = "graph_execution"
    GEN_AI_OPERATION_TYPE_USER_PROMPT_PROCESSING = "user_prompt_processing"
    GEN_AI_OPERATION_TYPE_MODEL_REQUEST = "model_request"
    GEN_AI_OPERATION_TYPE_TOOL_COORDINATION = "tool_coordination"

    # Julep-specific Operation Types
    GEN_AI_OPERATION_TYPE_AGENT_CREATE = "agent"
    GEN_AI_OPERATION_TYPE_TASK_CREATE = "task"
    GEN_AI_OPERATION_TYPE_EXECUTION_CREATE = "execution"

    # Model Request Types
    GEN_AI_MODEL_REQUEST_TYPE_INITIAL = "initial"
    GEN_AI_MODEL_REQUEST_TYPE_TOOL_RESPONSE = "tool_response"
    GEN_AI_MODEL_REQUEST_TYPE_CHAT = "chat"

    # Tool Processing Types
    GEN_AI_TOOL_PROCESSING_TYPE_EXECUTION = "execution"
    GEN_AI_TOOL_PROCESSING_TYPE_COORDINATION = "coordination"
    GEN_AI_OPERATION_TYPE_RETRIEVE = "retrieve"

    # GenAI Output Types (OTel Semconv)
    GEN_AI_OUTPUT_TYPE_IMAGE = "image"
    GEN_AI_OUTPUT_TYPE_JSON = "json"
    GEN_AI_OUTPUT_TYPE_SPEECH = "speech"
    GEN_AI_OUTPUT_TYPE_TEXT = "text"

    # GenAI System Names (OTel Semconv)
    GEN_AI_SYSTEM_ANTHROPIC = "anthropic"
    GEN_AI_SYSTEM_AWS_BEDROCK = "aws.bedrock"
    GEN_AI_SYSTEM_AZURE_AI_INFERENCE = "az.ai.inference"
    GEN_AI_SYSTEM_AZURE_OPENAI = "az.ai.openai"
    GEN_AI_SYSTEM_COHERE = "cohere"
    GEN_AI_SYSTEM_DEEPSEEK = "deepseek"
    GEN_AI_SYSTEM_GEMINI = "gemini"
    GEN_AI_SYSTEM_GROQ = "groq"
    GEN_AI_SYSTEM_IBM_WATSON = "ibm.watson.ai"
    GEN_AI_SYSTEM_MISTRAL = "mistral_ai"
    GEN_AI_SYSTEM_OPENAI = "openai"
    GEN_AI_SYSTEM_PERPLEXITY = "perplexity"
    GEN_AI_SYSTEM_VERTEXAI = "vertex_ai"
    GEN_AI_SYSTEM_XAI = "xai"

    # GenAI OpenAI Attributes (OTel Semconv)
    GEN_AI_REQUEST_SERVICE_TIER = "gen_ai.request.service_tier"
    GEN_AI_RESPONSE_SERVICE_TIER = "gen_ai.response.service_tier"
    GEN_AI_RESPONSE_SYSTEM_FINGERPRINT = "gen_ai.response.system_fingerprint"

    # GenAI System Names (Extra)
    GEN_AI_SYSTEM_HUGGING_FACE = "huggingface"
    GEN_AI_SYSTEM_OLLAMA = "ollama"
    GEN_AI_SYSTEM_GPT4ALL = "gpt4all"
    GEN_AI_SYSTEM_ELEVENLABS = "elevenlabs"
    GEN_AI_SYSTEM_VLLM = "vLLM"
    GEN_AI_SYSTEM_GOOGLE_AI_STUDIO = "google.ai.studio"
    GEN_AI_SYSTEM_REKAAI = "rekaai"
    GEN_AI_SYSTEM_PREMAI = "premai"
    GEN_AI_SYSTEM_LANGCHAIN = "langchain"
    GEN_AI_SYSTEM_LLAMAINDEX = "llama_index"
    GEN_AI_SYSTEM_HAYSTACK = "haystack"
    GEN_AI_SYSTEM_MEM0 = "mem0"
    GEN_AI_SYSTEM_LITELLM = "litellm"
    GEN_AI_SYSTEM_CREWAI = "crewai"
    GEN_AI_SYSTEM_AG2 = "ag2"
    GEN_AI_SYSTEM_MULTION = "multion"
    GEN_AI_SYSTEM_DYNAMIQ = "dynamiq"
    GEN_AI_SYSTEM_AGNO = "agno"
    GEN_AI_SYSTEM_JULEP = "julep"
    GEN_AI_SYSTEM_AI21 = "ai21"
    GEN_AI_SYSTEM_CONTROLFLOW = "controlflow"
    GEN_AI_SYSTEM_ASSEMBLYAI = "assemblyai"
    GEN_AI_SYSTEM_CRAWL4AI = "crawl4ai"
    GEN_AI_SYSTEM_FIRECRAWL = "firecrawl"
    GEN_AI_SYSTEM_LETTA = "letta"
    GEN_AI_SYSTEM_TOGETHER = "together"
    GEN_AI_SYSTEM_OPENAI_AGENTS = "openai_agents"
    GEN_AI_SYSTEM_PYDANTIC_AI = "pydantic_ai"
    GEN_AI_SYSTEM_SARVAM = "sarvam"
    GEN_AI_SYSTEM_BROWSER_USE = "browser_use"
    GEN_AI_SYSTEM_LANGGRAPH = "langgraph"

    # LangGraph-specific Attributes
    LANGGRAPH_GRAPH_NODES = "langgraph.graph.nodes"
    LANGGRAPH_GRAPH_NODE_COUNT = "langgraph.graph.node_count"
    LANGGRAPH_GRAPH_EDGES = "langgraph.graph.edges"
    LANGGRAPH_GRAPH_EDGE_COUNT = "langgraph.graph.edge_count"
    LANGGRAPH_EXECUTION_MODE = "langgraph.execution.mode"
    LANGGRAPH_EXECUTED_NODES = "langgraph.graph.executed_nodes"
    LANGGRAPH_NODE_EXECUTION_COUNT = "langgraph.graph.node_execution_count"
    LANGGRAPH_MESSAGE_COUNT = "langgraph.graph.message_count"
    LANGGRAPH_CHUNK_COUNT = "langgraph.graph.total_chunks"
    LANGGRAPH_FINAL_RESPONSE = "langgraph.graph.final_response"
    LANGGRAPH_NODE_NAME = "langgraph.node.name"
    LANGGRAPH_GRAPH_STATUS = "langgraph.graph.status"
    LANGGRAPH_THREAD_ID = "langgraph.thread.id"
    LANGGRAPH_CHECKPOINT_ID = "langgraph.checkpoint.id"
    LANGGRAPH_STREAM_MODE = "langgraph.stream.mode"

    # GenAI Framework Component Attributes (Standard)
    GEN_AI_FRAMEWORK_COMPONENT_NAME = "gen_ai.framework.component.name"
    GEN_AI_FRAMEWORK_COMPONENT_TYPE = "gen_ai.framework.component.type"
    GEN_AI_FRAMEWORK_COMPONENT_CLASS_NAME = "gen_ai.component.class_name"
    GEN_AI_FRAMEWORK_COMPONENT_INPUT_TYPES = "gen_ai.framework.component.input_types"
    GEN_AI_FRAMEWORK_COMPONENT_OUTPUT_TYPES = "gen_ai.framework.component.output_types"
    GEN_AI_FRAMEWORK_COMPONENT_INPUT_SPEC = "gen_ai.framework.component.input_spec"
    GEN_AI_FRAMEWORK_COMPONENT_OUTPUT_SPEC = "gen_ai.framework.component.output_spec"
    GEN_AI_FRAMEWORK_COMPONENT_VISITS = "gen_ai.framework.component.visits"
    GEN_AI_FRAMEWORK_COMPONENT_SENDERS = "gen_ai.framework.component.senders"
    GEN_AI_FRAMEWORK_COMPONENT_RECEIVERS = "gen_ai.framework.component.receivers"
    GEN_AI_FRAMEWORK_COMPONENT_CONNECTIONS = "gen_ai.framework.component.connections"

    # GenAI Framework Pipeline Attributes (Standard)
    GEN_AI_FRAMEWORK_PIPELINE_INPUT_DATA = "gen_ai.framework.pipeline.input_data"
    GEN_AI_FRAMEWORK_PIPELINE_OUTPUT_DATA = "gen_ai.framework.pipeline.output_data"
    GEN_AI_FRAMEWORK_PIPELINE_METADATA = "gen_ai.framework.pipeline.metadata"
    GEN_AI_FRAMEWORK_PIPELINE_MAX_RUNS = (
        "gen_ai.framework.pipeline.max_runs_per_component"
    )
    GEN_AI_FRAMEWORK_PIPELINE_COMPONENT_COUNT = (
        "gen_ai.framework.pipeline.component_count"
    )
    GEN_AI_FRAMEWORK_PIPELINE_EXECUTION_TIME = (
        "gen_ai.framework.pipeline.execution_time"
    )

    # GenAI Request Attributes (Extra)
    GEN_AI_REQUEST_IS_STREAM = "gen_ai.request.is_stream"
    GEN_AI_REQUEST_USER = "gen_ai.request.user"
    GEN_AI_REQUEST_EMBEDDING_DIMENSION = "gen_ai.request.embedding_dimension"
    GEN_AI_REQUEST_TOOL_CHOICE = "gen_ai.request.tool_choice"
    GEN_AI_REQUEST_AUDIO_VOICE = "gen_ai.request.audio_voice"
    GEN_AI_REQUEST_AUDIO_RESPONSE_FORMAT = "gen_ai.request.audio_response_format"
    GEN_AI_REQUEST_AUDIO_SPEED = "gen_ai.request.audio_speed"
    GEN_AI_REQUEST_AUDIO_SETTINGS = "gen_ai.request.audio_settings"
    GEN_AI_REQUEST_AUDIO_DURATION = "gen_ai.request.audio_duration"

    # Translation request attributes
    GEN_AI_REQUEST_TRANSLATE_SOURCE_LANGUAGE = (
        "gen_ai.request.translate.source_language"
    )
    GEN_AI_REQUEST_TRANSLATE_TARGET_LANGUAGE = (
        "gen_ai.request.translate.target_language"
    )
    GEN_AI_REQUEST_TRANSLATE_SPEAKER_GENDER = "gen_ai.request.translate.speaker_gender"
    GEN_AI_REQUEST_TRANSLATE_MODE = "gen_ai.request.translate.mode"
    GEN_AI_REQUEST_TRANSLATE_ENABLE_PREPROCESSING = (
        "gen_ai.request.translate.enable_preprocessing"
    )
    GEN_AI_REQUEST_TRANSLATE_OUTPUT_SCRIPT = "gen_ai.request.translate.output_script"
    GEN_AI_REQUEST_TRANSLATE_NUMERALS_FORMAT = (
        "gen_ai.request.translate.numerals_format"
    )

    # Translation response attributes
    GEN_AI_RESPONSE_TRANSLATE_SOURCE_LANGUAGE = (
        "gen_ai.response.translate.source_language"
    )

    # Transliteration request attributes
    GEN_AI_REQUEST_TRANSLITERATE_SOURCE_LANGUAGE = (
        "gen_ai.request.transliterate.source_language"
    )
    GEN_AI_REQUEST_TRANSLITERATE_TARGET_LANGUAGE = (
        "gen_ai.request.transliterate.target_language"
    )
    GEN_AI_REQUEST_TRANSLITERATE_NUMERALS_FORMAT = (
        "gen_ai.request.transliterate.numerals_format"
    )
    GEN_AI_REQUEST_TRANSLITERATE_SPOKEN_FORM = (
        "gen_ai.request.transliterate.spoken_form"
    )
    GEN_AI_REQUEST_TRANSLITERATE_SPOKEN_FORM_NUMERALS_LANGUAGE = (
        "gen_ai.request.transliterate.spoken_form_numerals_language"
    )

    # Transliteration response attributes
    GEN_AI_RESPONSE_TRANSLITERATE_SOURCE_LANGUAGE = (
        "gen_ai.response.transliterate.source_language"
    )

    # Language identification response attributes
    GEN_AI_RESPONSE_LANGUAGE_CODE = "gen_ai.response.language_code"
    GEN_AI_RESPONSE_SCRIPT_CODE = "gen_ai.response.script_code"

    # Speech-to-text request attributes
    GEN_AI_REQUEST_SPEECH_LANGUAGE_CODE = "gen_ai.request.speech.language_code"
    GEN_AI_REQUEST_SPEECH_PROMPT = "gen_ai.request.speech.prompt"
    GEN_AI_REQUEST_SPEECH_WITH_TIMESTAMPS = "gen_ai.request.speech.with_timestamps"

    # Speech-to-text response attributes
    GEN_AI_RESPONSE_SPEECH_TIMESTAMPS = "gen_ai.response.speech.timestamps"
    GEN_AI_RESPONSE_SPEECH_DIARIZED_TRANSCRIPT = (
        "gen_ai.response.speech.diarized_transcript"
    )
    GEN_AI_RESPONSE_SPEECH_DETECTED_LANGUAGE = (
        "gen_ai.response.speech.detected_language"
    )

    # Text-to-speech request attributes
    GEN_AI_REQUEST_TTS_TARGET_LANGUAGE_CODE = "gen_ai.request.tts.target_language_code"
    GEN_AI_REQUEST_TTS_SPEAKER = "gen_ai.request.tts.speaker"
    GEN_AI_REQUEST_TTS_PITCH = "gen_ai.request.tts.pitch"
    GEN_AI_REQUEST_TTS_PACE = "gen_ai.request.tts.pace"
    GEN_AI_REQUEST_TTS_LOUDNESS = "gen_ai.request.tts.loudness"
    GEN_AI_REQUEST_TTS_SPEECH_SAMPLE_RATE = "gen_ai.request.tts.speech_sample_rate"
    GEN_AI_REQUEST_TTS_ENABLE_PREPROCESSING = "gen_ai.request.tts.enable_preprocessing"
    GEN_AI_REQUEST_TTS_OUTPUT_AUDIO_CODEC = "gen_ai.request.tts.output_audio_codec"

    # Provider-specific request attributes
    GEN_AI_REQUEST_WIKI_GROUNDING = "gen_ai.request.wiki_grounding"
    GEN_AI_REQUEST_N = "gen_ai.request.n"
    GEN_AI_REQUEST_IMAGE_SIZE = "gen_ai.request.image_size"
    GEN_AI_REQUEST_IMAGE_QUALITY = "gen_ai.request.image_quality"
    GEN_AI_REQUEST_IMAGE_STYLE = "gen_ai.request.image_style"
    GEN_AI_HUB_OWNER = "gen_ai.hub.owner"
    GEN_AI_HUB_REPO = "gen_ai.hub.repo"
    GEN_AI_RETRIEVAL_SOURCE = "gen_ai.retrieval.source"
    GEN_AI_REQUESTS = "gen_ai.total.requests"
    GEN_AI_DATA_SOURCES = "gen_ai.data_source_count"
    GEN_AI_ENVIRONMENT = "gen_ai.environment"
    GEN_AI_APPLICATION_NAME = "gen_ai.application_name"
    GEN_AI_SDK_VERSION = "gen_ai.sdk.version"

    # GenAI Response Attributes (Extra)
    GEN_AI_USAGE_TOTAL_TOKENS = "gen_ai.usage.total_tokens"
    GEN_AI_USAGE_COST = "gen_ai.usage.cost"
    GEN_AI_RESPONSE_IMAGE = "gen_ai.response.image"
    GEN_AI_TOOL_CALLS = "gen_ai.response.tool_calls"

    # GenAI Content
    GEN_AI_CONTENT_PROMPT_EVENT = "gen_ai.content.prompt"
    GEN_AI_CONTENT_PROMPT = "gen_ai.prompt"
    GEN_AI_CONTENT_COMPLETION_EVENT = "gen_ai.content.completion"
    GEN_AI_CONTENT_COMPLETION = "gen_ai.completion"
    GEN_AI_CONTENT_REVISED_PROMPT = "gen_ai.content.revised_prompt"
    GEN_AI_CONTENT_REASONING = "gen_ai.content.reasoning"

    # Tool Attributes (LangChain/Framework Support)
    GEN_AI_TOOL_INPUT = "gen_ai.tool.input"
    GEN_AI_TOOL_OUTPUT = "gen_ai.tool.output"

    # Retrieval Attributes (LangChain Retriever Support)
    GEN_AI_RETRIEVAL_QUERY = "gen_ai.retrieval.query"
    GEN_AI_RETRIEVAL_DOCUMENTS = "gen_ai.retrieval.documents"
    GEN_AI_RETRIEVAL_DOCUMENT_COUNT = "gen_ai.retrieval.document_count"

    # Workflow Attributes (LangChain Chain Support)
    GEN_AI_WORKFLOW_INPUT = "gen_ai.workflow.input"
    GEN_AI_WORKFLOW_OUTPUT = "gen_ai.workflow.output"
    GEN_AI_WORKFLOW_TYPE = "gen_ai.workflow.type"

    # Framework Attributes (Enhanced Observability)
    GEN_AI_FRAMEWORK_TAGS = "gen_ai.framework.tags"
    GEN_AI_FRAMEWORK_ERROR_CLASS = "gen_ai.framework.error.class"
    GEN_AI_FRAMEWORK_ERROR_TYPE = "gen_ai.framework.error.type"
    GEN_AI_FRAMEWORK_ERROR_MESSAGE = "gen_ai.framework.error.message"
    GEN_AI_FRAMEWORK_PERFORMANCE_VS_BASELINE = (
        "gen_ai.framework.performance.vs_baseline"
    )
    GEN_AI_FRAMEWORK_PERFORMANCE_BASELINE_AVG = (
        "gen_ai.framework.performance.baseline_avg"
    )
    GEN_AI_FRAMEWORK_PERFORMANCE_BASELINE_PERCENTILE = (
        "gen_ai.framework.performance.baseline_percentile"
    )

    # Serialized Function Capture (LangSmith-style)
    GEN_AI_SERIALIZED_NAME = "gen_ai.serialized.name"
    GEN_AI_SERIALIZED_SIGNATURE = "gen_ai.serialized.signature"
    GEN_AI_SERIALIZED_DOC = "gen_ai.serialized.doc"
    GEN_AI_SERIALIZED_MODULE = "gen_ai.serialized.module"

    # Provider Mapping
    GEN_AI_REQUEST_PROVIDER = "gen_ai.request.provider"

    # Enhanced Token Usage Details (OpenAI/Anthropic Cache Support)
    GEN_AI_USAGE_COMPLETION_TOKENS_DETAILS_AUDIO = (
        "gen_ai.usage.completion_tokens_details.audio_tokens"
    )
    GEN_AI_USAGE_COMPLETION_TOKENS_DETAILS_REASONING = (
        "gen_ai.usage.completion_tokens_details.reasoning_tokens"
    )
    GEN_AI_USAGE_PROMPT_TOKENS_DETAILS_CACHE_READ = (
        "gen_ai.usage.prompt_tokens_details.cached_tokens"
    )
    GEN_AI_USAGE_PROMPT_TOKENS_DETAILS_CACHE_WRITE = (
        "gen_ai.usage.prompt_tokens_details.cache_creation_tokens"
    )

    # GenAI Rag
    GEN_AI_RAG_MAX_SEGMENTS = "gen_ai.rag.max_segments"
    GEN_AI_RAG_STRATEGY = "gen_ai.rag.strategy"
    GEN_AI_RAG_SIMILARITY_THRESHOLD = "gen_ai.rag.similarity_threshold"
    GEN_AI_RAG_MAX_NEIGHBORS = "gen_ai.rag.max_neighbors"
    GEN_AI_RAG_DOCUMENTS_PATH = "gen_ai.rag.documents_path"
    GEN_AI_RAG_FILE_IDS = "gen_ai.rag.file_ids"

    # GenAI Evaluation Metrics
    GEN_AI_EVAL_CONTEXT_RELEVANCY = "gen_ai.eval.context_relevancy"
    GEN_AI_EVAL_ANSWER_RELEVANCY = "gen_ai.eval.answer_relevancy"
    GEN_AI_EVAL_GROUNDEDNESS = "gen_ai.eval.groundedness"

    # VectorDB Metrics (OTel Semconv)
    DB_CLIENT_OPERATION_DURATION = "db.client.operation.duration"

    # Vector DB Attributes (OTel Semconv)
    DB_SYSTEM_NAME = "db.system.name"
    DB_COLLECTION_NAME = "db.collection.name"
    DB_NAMESPACE = "db.namespace"
    DB_OPERATION_NAME = "db.operation.name"
    DB_QUERY_TEXT = "db.query.text"
    DB_QUERY_SUMMARY = "db.query.summary"
    DB_QUERY_PARAMETER = "db.query.parameter"
    DB_RESPONSE_RETURNED_ROWS = "db.response.returned_rows"

    # Vector DB Attributes (Extras)
    DB_SDK_VERSION = "db.sdk.version"
    DB_OPERATION_API_ENDPOINT = "db.operation.api_endpoint"
    DB_REQUESTS = "db.total.requests"
    DB_OPERATION_ID = "db.operation.id"
    DB_OPERATION_STATUS = "db.operation.status"
    DB_OPERATION_COST = "db.operation.cost"
    DB_OPERATION_CREATE_INDEX = "create_index"
    DB_OPERATION_GET_COLLECTION = "get_collection"
    DB_OPERATION_CREATE_COLLECTION = "create_collection"
    DB_OPERATION_UPDATE_COLLECTION = "update_collection"
    DB_OPERATION_DELETE_COLLECTION = "delete_collection"
    DB_OPERATION_INSERT = "INSERT"
    DB_OPERATION_SELECT = "SELECT"
    DB_OPERATION_QUERY = "QUERY"
    DB_OPERATION_SEARCH = "SEARCH"
    DB_OPERATION_FETCH = "FETCH"
    DB_OPERATION_REPLACE = "findAndModify"
    DB_OPERATION_FIND_AND_DELETE = "findAndDelete"
    DB_OPERATION_DELETE = "DELETE"
    DB_OPERATION_UPDATE = "UPDATE"
    DB_OPERATION_UPSERT = "UPSERT"
    DB_OPERATION_GET = "GET"
    DB_OPERATION_ADD = "ADD"
    DB_OPERATION_PEEK = "PEEK"
    DB_ID_COUNT = "db.ids_count"
    DB_VECTOR_COUNT = "db.vector.count"
    DB_METADATA = "db.metadata"
    DB_METADATA_COUNT = "db.metadatas_count"
    DB_DOCUMENTS_COUNT = "db.documents_count"
    DB_PAYLOAD_COUNT = "db.payload_count"
    DB_QUERY_LIMIT = "db.limit"
    DB_OFFSET = "db.offset"
    DB_WHERE_DOCUMENT = "db.where_document"
    DB_FILTER = "db.filter"
    DB_STATEMENT = "db.statement"
    DB_N_RESULTS = "db.n_results"
    DB_DELETE_ALL = "db.delete_all"
    DB_INDEX_NAME = "db.index.name"
    DB_COLLECTION_DIMENSION = "db.collection.dimension"
    DB_INDEX_DIMENSION = "db.index.dimension"
    DB_COLLECTION_DIMENSION = "db.collection.dimension"
    DB_SEARCH_SIMILARITY_METRIC = "db.search.similarity_metric"
    DB_INDEX_METRIC = "db.create_index.metric"
    DB_COLLECTION_SPEC = "db.collection.spec"
    DB_INDEX_SPEC = "db.create_index.spec"
    DB_NAMESPACE = "db.query.namespace"
    DB_UPDATE_METADATA = "db.update.metadata"
    DB_UPDATE_VALUES = "db.update.values"
    DB_UPDATE_ID = "db.update.id"
    DB_DELETE_ID = "db.delete.id"
    DB_VECTOR_QUERY_TOP_K = "db.vector.query.top_k"
    DB_VECTOR_QUERY_FILTER = "db.vector.query.filter"

    DB_SYSTEM_CHROMA = "chroma"
    DB_SYSTEM_PINECONE = "pinecone"
    DB_SYSTEM_QDRANT = "qdrant"
    DB_SYSTEM_MILVUS = "milvus"
    DB_SYSTEM_ASTRA = "astra"
    DB_SYSTEM_POSTGRESQL = "postgresql"

    # SQL Transaction Operations
    DB_OPERATION_COMMIT = "COMMIT"
    DB_OPERATION_ROLLBACK = "ROLLBACK"
    DB_OPERATION_COPY = "COPY"
    DB_OPERATION_CREATE = "CREATE"
    DB_OPERATION_ALTER = "ALTER"
    DB_OPERATION_DROP = "DROP"
    DB_OPERATION_TRUNCATE = "TRUNCATE"
    DB_OPERATION_CALL = "CALL"

    # PostgreSQL-specific attributes
    DB_POSTGRESQL_ROWS_AFFECTED = "db.postgresql.rows_affected"
    DB_POSTGRESQL_PLAN = "db.postgresql.plan"

    # GenAI Request Attributes (OTel Semconv)
    GEN_AI_AGENT_ID = "gen_ai.agent.id"
    GEN_AI_AGENT_NAME = "gen_ai.agent.name"
    GEN_AI_AGENT_DESCRIPTION = "gen_ai.agent.description"

    GEN_AI_AGENT_TYPE = "gen_ai.agent.type"
    GEN_AI_AGENT_TASK_ID = "gen_ai.agent.task.id"
    GEN_AI_AGENT_ROLE = "gen_ai.agent.role"
    GEN_AI_AGENT_GOAL = "gen_ai.agent.goal"
    GEN_AI_AGENT_CONTEXT = "gen_ai.agent.context"
    GEN_AI_AGENT_ENABLE_CACHE = "gen_ai.agent.enable_cache"
    GEN_AI_AGENT_ENABLE_HISTORY = "gen_ai.agent.enable_history"
    GEN_AI_AGENT_ALLOW_DELEGATION = "gen_ai.agent.allow_delegation"
    GEN_AI_AGENT_ALLOW_CODE_EXECUTION = "gen_ai.agent.allow_code_execution"
    GEN_AI_AGENT_MAX_RETRY_LIMIT = "gen_ai.agent.max_retry_limit"
    GEN_AI_AGENT_TOOLS = "gen_ai.agent.tools"
    GEN_AI_AGENT_TOOL_RESULTS = "gen_ai.agent.tool_results"
    GEN_AI_AGENT_TASK = "gen_ai.agent.task"
    GEN_AI_AGENT_PARAMS = "gen_ai.agent.params"
    GEN_AI_AGENT_INSTRUCTIONS = "gen_ai.agent.instructions"
    GEN_AI_AGENT_STORAGE = "gen_ai.agent.storage"
    GEN_AI_AGENT_EXPECTED_OUTPUT = "gen_ai.agent.expected_output"
    GEN_AI_AGENT_ACTUAL_OUTPUT = "gen_ai.agent.actual_output"
    GEN_AI_AGENT_HUMAN_INPUT = "gen_ai.agent.human_input"
    GEN_AI_AGENT_SCHEMA = "gen_ai.agent.schema"
    GEN_AI_AGENT_TASK_ASSOCIATION = "gen_ai.agent.task_associations"
    GEN_AI_AGENT_BROWSE_URL = "gen_ai.agent.browse_url"
    GEN_AI_AGENT_STEP_COUNT = "gen_ai.agent.step_count"
    GEN_AI_AGENT_RESPONSE_TIME = "gen_ai.agent.response_time"
    GEN_AI_AGENT_STRATEGY = "gen_ai.agent.strategy"

    # Agent Action/Finish Attributes (LangChain Agent Support)
    GEN_AI_AGENT_ACTION_TOOL = "gen_ai.agent.action.tool"
    GEN_AI_AGENT_ACTION_TOOL_INPUT = "gen_ai.agent.action.tool_input"
    GEN_AI_AGENT_ACTION_LOG = "gen_ai.agent.action.log"
    GEN_AI_AGENT_FINISH_OUTPUT = "gen_ai.agent.finish.output"
    GEN_AI_AGENT_FINISH_LOG = "gen_ai.agent.finish.log"

    GEN_AI_AGENT_TYPE_BROWSER = "browser"

    # Crawl4AI Specific Attributes (0.7.x Support)
    GEN_AI_CRAWL_URL_COUNT = "gen_ai.crawl.url_count"
    GEN_AI_CRAWL_DEPTH = "gen_ai.crawl.depth"
    GEN_AI_CRAWL_SESSION_ID = "gen_ai.crawl.session_id"
    GEN_AI_CRAWL_CACHE_MODE = "gen_ai.crawl.cache_mode"
    GEN_AI_CRAWL_EXTRACTION_STRATEGY = "gen_ai.crawl.extraction_strategy"
    GEN_AI_CRAWL_SCRAPING_STRATEGY = "gen_ai.crawl.scraping_strategy"
    GEN_AI_CRAWL_BROWSER_TYPE = "gen_ai.crawl.browser_type"
    GEN_AI_CRAWL_HEADLESS = "gen_ai.crawl.headless"
    GEN_AI_CRAWL_VIEWPORT_WIDTH = "gen_ai.crawl.viewport_width"
    GEN_AI_CRAWL_VIEWPORT_HEIGHT = "gen_ai.crawl.viewport_height"
    GEN_AI_CRAWL_USER_AGENT = "gen_ai.crawl.user_agent"
    GEN_AI_CRAWL_WORD_COUNT_THRESHOLD = "gen_ai.crawl.word_count_threshold"
    GEN_AI_CRAWL_CSS_SELECTOR = "gen_ai.crawl.css_selector"
    GEN_AI_CRAWL_EXCLUDED_TAGS = "gen_ai.crawl.excluded_tags"
    GEN_AI_CRAWL_SCREENSHOT = "gen_ai.crawl.screenshot"
    GEN_AI_CRAWL_PDF = "gen_ai.crawl.pdf"
    GEN_AI_CRAWL_WAIT_FOR = "gen_ai.crawl.wait_for"
    GEN_AI_CRAWL_PAGE_TIMEOUT = "gen_ai.crawl.page_timeout"
    GEN_AI_CRAWL_JS_CODE = "gen_ai.crawl.js_code"
    GEN_AI_CRAWL_RESULT_SUCCESS = "gen_ai.crawl.result.success"
    GEN_AI_CRAWL_RESULT_STATUS_CODE = "gen_ai.crawl.result.status_code"
    GEN_AI_CRAWL_RESULT_HTML_LENGTH = "gen_ai.crawl.result.html_length"
    GEN_AI_CRAWL_RESULT_MARKDOWN_LENGTH = "gen_ai.crawl.result.markdown_length"
    GEN_AI_CRAWL_RESULT_LINKS_COUNT = "gen_ai.crawl.result.links_count"
    GEN_AI_CRAWL_RESULT_IMAGES_COUNT = "gen_ai.crawl.result.images_count"
    GEN_AI_CRAWL_RESULT_REDIRECTED_URL = "gen_ai.crawl.result.redirected_url"
    GEN_AI_CRAWL_DEEP_STRATEGY = "gen_ai.crawl.deep_strategy"
    GEN_AI_CRAWL_PROXY_CONFIG = "gen_ai.crawl.proxy_config"

    # Crawl4AI LLM Extraction Attributes (0.7.x)
    GEN_AI_EXTRACTION_STRATEGY_TYPE = "gen_ai.extraction.strategy.type"
    GEN_AI_EXTRACTION_TYPE = "gen_ai.extraction.type"
    GEN_AI_EXTRACTION_INSTRUCTION = "gen_ai.extraction.instruction"
    GEN_AI_EXTRACTION_SCHEMA = "gen_ai.extraction.schema"
    GEN_AI_EXTRACTION_INPUT_FORMAT = "gen_ai.extraction.input_format"
    GEN_AI_EXTRACTION_CHUNK_COUNT = "gen_ai.extraction.chunk_count"
    GEN_AI_EXTRACTION_CHUNK_TOKEN_THRESHOLD = "gen_ai.extraction.chunk_token_threshold"
    GEN_AI_EXTRACTION_OVERLAP_RATE = "gen_ai.extraction.overlap_rate"
    GEN_AI_EXTRACTION_APPLY_CHUNKING = "gen_ai.extraction.apply_chunking"
    GEN_AI_EXTRACTION_SUCCESS = "gen_ai.extraction.success"
    GEN_AI_EXTRACTION_ERROR = "gen_ai.extraction.error"

    # LLM Provider and Model Information
    GEN_AI_LLM_PROVIDER = "gen_ai.llm.provider"
    GEN_AI_LLM_MODEL = "gen_ai.llm.model"
    GEN_AI_LLM_BASE_URL = "gen_ai.llm.base_url"
    GEN_AI_LLM_TEMPERATURE = "gen_ai.llm.temperature"
    GEN_AI_LLM_MAX_TOKENS = "gen_ai.llm.max_tokens"
    GEN_AI_LLM_TOP_P = "gen_ai.llm.top_p"

    # Token Usage and Cost Tracking (Enhanced Business Intelligence)
    GEN_AI_TOKEN_USAGE_INPUT = "gen_ai.token.usage.input"
    GEN_AI_TOKEN_USAGE_OUTPUT = "gen_ai.token.usage.output"
    GEN_AI_TOKEN_USAGE_TOTAL = "gen_ai.token.usage.total"
    GEN_AI_TOKEN_COST_INPUT = "gen_ai.token.cost.input"
    GEN_AI_TOKEN_COST_OUTPUT = "gen_ai.token.cost.output"
    GEN_AI_TOKEN_COST_TOTAL = "gen_ai.token.cost.total"
    GEN_AI_TOKEN_CHUNK_USAGE = "gen_ai.token.chunk_usage"
    GEN_AI_TOKEN_CHUNK_COUNT = "gen_ai.token.chunk_count"

    # CrawlerMonitor Integration Attributes
    GEN_AI_MONITOR_TASK_ID = "gen_ai.monitor.task_id"
    GEN_AI_MONITOR_TASK_STATUS = "gen_ai.monitor.task_status"
    GEN_AI_MONITOR_MEMORY_USAGE = "gen_ai.monitor.memory_usage"
    GEN_AI_MONITOR_PEAK_MEMORY = "gen_ai.monitor.peak_memory"
    GEN_AI_MONITOR_RETRY_COUNT = "gen_ai.monitor.retry_count"
    GEN_AI_MONITOR_WAIT_TIME = "gen_ai.monitor.wait_time"
    GEN_AI_MONITOR_QUEUE_SIZE = "gen_ai.monitor.queue_size"
    GEN_AI_MONITOR_COMPLETION_RATE = "gen_ai.monitor.completion_rate"

    # Extraction Strategy Types
    GEN_AI_EXTRACTION_STRATEGY_LLM = "llm"
    GEN_AI_EXTRACTION_STRATEGY_CSS = "css"
    GEN_AI_EXTRACTION_STRATEGY_XPATH = "xpath"
    GEN_AI_EXTRACTION_STRATEGY_COSINE = "cosine"
    GEN_AI_EXTRACTION_STRATEGY_REGEX = "regex"
    GEN_AI_EXTRACTION_STRATEGY_LXML = "lxml"

    # LLM Extraction Types
    GEN_AI_EXTRACTION_TYPE_SCHEMA = "schema"
    GEN_AI_EXTRACTION_TYPE_BLOCK = "block"

    # Monitor Task Status Values
    GEN_AI_MONITOR_STATUS_PENDING = "pending"
    GEN_AI_MONITOR_STATUS_RUNNING = "running"
    GEN_AI_MONITOR_STATUS_COMPLETED = "completed"
    GEN_AI_MONITOR_STATUS_FAILED = "failed"
    GEN_AI_MONITOR_STATUS_RETRYING = "retrying"

    # Crawl4AI Operation Types (reusing existing pattern)
    GEN_AI_OPERATION_TYPE_CRAWL = "crawl"
    GEN_AI_OPERATION_TYPE_CRAWL_DEEP = "crawl_deep"
    GEN_AI_OPERATION_TYPE_EXTRACT = "extract"
    GEN_AI_OPERATION_TYPE_EXTRACT_LLM = "extract_llm"
    GEN_AI_OPERATION_TYPE_EXTRACT_CSS = "extract_css"
    GEN_AI_OPERATION_TYPE_EXTRACT_XPATH = "extract_xpath"
    GEN_AI_OPERATION_TYPE_EXTRACT_COSINE = "extract_cosine"
    GEN_AI_OPERATION_TYPE_EXTRACT_REGEX = "extract_regex"
    GEN_AI_OPERATION_TYPE_SCRAPE = "scrape"

    # Firecrawl Operation Types
    GEN_AI_OPERATION_TYPE_MAP = "map"
    GEN_AI_OPERATION_TYPE_SEARCH = "search"
    GEN_AI_OPERATION_TYPE_CRAWL_STATUS = "crawl_status"
    GEN_AI_OPERATION_TYPE_SCRAPE_STATUS = "scrape_status"
    GEN_AI_OPERATION_TYPE_EXTRACT_STATUS = "extract_status"
    GEN_AI_OPERATION_TYPE_CANCEL = "cancel"

    # GPU
    GPU_INDEX = "gpu.index"
    GPU_UUID = "gpu.uuid"
    GPU_NAME = "gpu.name"

    GPU_UTILIZATION = "gpu.utilization"
    GPU_UTILIZATION_ENC = "gpu.enc.utilization"
    GPU_UTILIZATION_DEC = "gpu.dec.utilization"
    GPU_TEMPERATURE = "gpu.temperature"
    GPU_FAN_SPEED = "gpu.fan_speed"
    GPU_MEMORY_AVAILABLE = "gpu.memory.available"
    GPU_MEMORY_TOTAL = "gpu.memory.total"
    GPU_MEMORY_USED = "gpu.memory.used"
    GPU_MEMORY_FREE = "gpu.memory.free"
    GPU_POWER_DRAW = "gpu.power.draw"
    GPU_POWER_LIMIT = "gpu.power.limit"

    # Browser-Use Specific Attributes
    GEN_AI_BROWSER_ACTION_TYPE = "gen_ai.browser.action.type"
    GEN_AI_BROWSER_ACTION_TARGET = "gen_ai.browser.action.target"
    GEN_AI_BROWSER_ACTION_VALUE = "gen_ai.browser.action.value"
    GEN_AI_BROWSER_ACTION_SELECTOR = "gen_ai.browser.action.selector"
    GEN_AI_BROWSER_PAGE_URL = "gen_ai.browser.page.url"
    GEN_AI_BROWSER_PAGE_TITLE = "gen_ai.browser.page.title"
    GEN_AI_BROWSER_VIEWPORT_WIDTH = "gen_ai.browser.viewport.width"
    GEN_AI_BROWSER_VIEWPORT_HEIGHT = "gen_ai.browser.viewport.height"
    GEN_AI_BROWSER_USER_AGENT = "gen_ai.browser.user_agent"
    GEN_AI_BROWSER_TASK_DESCRIPTION = "gen_ai.browser.task.description"
    GEN_AI_BROWSER_STEP_NUMBER = "gen_ai.browser.step.number"
    GEN_AI_BROWSER_MAX_STEPS = "gen_ai.browser.max_steps"
    GEN_AI_BROWSER_DOM_ELEMENTS_COUNT = "gen_ai.browser.dom.elements_count"
    GEN_AI_BROWSER_SCREENSHOT_TAKEN = "gen_ai.browser.screenshot.taken"
    GEN_AI_BROWSER_SESSION_ID = "gen_ai.browser.session.id"
    GEN_AI_BROWSER_CONTROLLER_TYPE = "gen_ai.browser.controller.type"

    # Browser-Use Operation Types
    GEN_AI_OPERATION_TYPE_BROWSER_RUN = "browser_run"
    GEN_AI_OPERATION_TYPE_BROWSER_STEP = "browser_step"
    GEN_AI_OPERATION_TYPE_BROWSER_ACTION = "browser_action"
    GEN_AI_OPERATION_TYPE_BROWSER_PAUSE = "browser_pause"
    GEN_AI_OPERATION_TYPE_BROWSER_RESUME = "browser_resume"
    GEN_AI_OPERATION_TYPE_BROWSER_STOP = "browser_stop"

    # Browser-Use Agent Specific Attributes
    GEN_AI_AGENT_MAX_STEPS = "gen_ai.agent.max_steps"
    GEN_AI_AGENT_ID = "gen_ai.agent.id"
    GEN_AI_AGENT_TASK_ID = "gen_ai.agent.task_id"
    GEN_AI_AGENT_SESSION_ID = "gen_ai.agent.session_id"
    GEN_AI_AGENT_USE_VISION = "gen_ai.agent.use_vision"
    GEN_AI_AGENT_MAX_FAILURES = "gen_ai.agent.max_failures"
    GEN_AI_AGENT_MAX_ACTIONS_PER_STEP = "gen_ai.agent.max_actions_per_step"
    GEN_AI_AGENT_HEADLESS = "gen_ai.agent.headless"
    GEN_AI_AGENT_ALLOWED_DOMAINS = "gen_ai.agent.allowed_domains"
    GEN_AI_AGENT_VISION_DETAIL_LEVEL = "gen_ai.agent.vision_detail_level"
    GEN_AI_AGENT_RETRY_DELAY = "gen_ai.agent.retry_delay"
    GEN_AI_AGENT_VALIDATE_OUTPUT = "gen_ai.agent.validate_output"
    GEN_AI_AGENT_LLM_TIMEOUT = "gen_ai.agent.llm_timeout"

    # Browser-Use Action Specific Attributes
    GEN_AI_ACTION_SUCCESS = "gen_ai.action.success"
    GEN_AI_ACTION_ERROR = "gen_ai.action.error"
    GEN_AI_ACTION_TYPE = "gen_ai.action.type"
    GEN_AI_ACTION_INDEX = "gen_ai.action.index"
    GEN_AI_ACTION_HAS_SENSITIVE_DATA = "gen_ai.action.has_sensitive_data"
    GEN_AI_ACTION_FILE_PATH = "gen_ai.action.file_path"
    GEN_AI_ACTION_EXTRACTED_CONTENT_LENGTH = "gen_ai.action.extracted_content_length"

    # Browser-Use Browser Specific Attributes
    GEN_AI_BROWSER_PAGE_TITLE = "gen_ai.browser.page_title"
    GEN_AI_BROWSER_TABS_COUNT = "gen_ai.browser.tabs_count"

    # Browser-Use Agent Execution Attributes
    GEN_AI_AGENT_THINKING = "gen_ai.agent.thinking"
    GEN_AI_AGENT_MEMORY = "gen_ai.agent.memory"
    GEN_AI_AGENT_NEXT_GOAL = "gen_ai.agent.next_goal"
    GEN_AI_AGENT_EVALUATION = "gen_ai.agent.evaluation"
    GEN_AI_AGENT_ACTIONS = "gen_ai.agent.actions"
    GEN_AI_AGENT_ACTIONS_COUNT = "gen_ai.agent.actions_count"
    GEN_AI_AGENT_PAGE_TITLE = "gen_ai.agent.page_title"
    GEN_AI_AGENT_TABS_COUNT = "gen_ai.agent.tabs_count"
    GEN_AI_AGENT_INTERACTED_ELEMENTS_COUNT = "gen_ai.agent.interacted_elements_count"
    GEN_AI_AGENT_ACTIONS_SUCCESS_COUNT = "gen_ai.agent.actions_success_count"
    GEN_AI_AGENT_ACTIONS_ERROR_COUNT = "gen_ai.agent.actions_error_count"
    GEN_AI_AGENT_ACTION_ERRORS = "gen_ai.agent.action_errors"
    GEN_AI_AGENT_STEP_DURATION = "gen_ai.agent.step_duration"
    GEN_AI_AGENT_TOTAL_ACTIONS = "gen_ai.agent.total_actions"
    GEN_AI_AGENT_SUCCESSFUL_STEPS = "gen_ai.agent.successful_steps"
    GEN_AI_AGENT_FAILED_STEPS = "gen_ai.agent.failed_steps"
    GEN_AI_AGENT_SUCCESS_RATE = "gen_ai.agent.success_rate"
    GEN_AI_AGENT_FINAL_RESULT = "gen_ai.agent.final_result"

    # Browser-Use Operation Attributes
    GEN_AI_OPERATION_TYPE = "gen_ai.operation.type"
    GEN_AI_CLIENT_OPERATION_DURATION = "gen_ai.client.operation.duration"

    # Browser-Use Span Name Components
    GEN_AI_SPAN_INVOKE_MODEL = "invoke_model"

    GEN_AI_BROWSER_SESSION_DURATION = "gen_ai.browser.session.duration"
    GEN_AI_BROWSER_SESSION_ACTIONS_COUNT = "gen_ai.browser.session.actions_count"
    GEN_AI_BROWSER_SESSION_SUCCESS_RATE = "gen_ai.browser.session.success_rate"
    GEN_AI_BROWSER_PAGE_LOAD_TIME = "gen_ai.browser.page.load_time"
    GEN_AI_BROWSER_ACTION_SUCCESS = "gen_ai.browser.action.success"
    GEN_AI_BROWSER_ACTION_RETRY_COUNT = "gen_ai.browser.action.retry_count"
    GEN_AI_BROWSER_ERROR_TYPE = "gen_ai.browser.error.type"
    GEN_AI_BROWSER_ERROR_MESSAGE = "gen_ai.browser.error.message"
    GEN_AI_BROWSER_NETWORK_REQUESTS_COUNT = "gen_ai.browser.network.requests_count"
    GEN_AI_BROWSER_NETWORK_FAILED_REQUESTS = "gen_ai.browser.network.failed_requests"
    GEN_AI_BROWSER_MEMORY_USAGE = "gen_ai.browser.memory.usage"
    GEN_AI_BROWSER_CPU_USAGE = "gen_ai.browser.cpu.usage"
    GEN_AI_BROWSER_SCREENSHOT_CAPTURED = "gen_ai.browser.screenshot.captured"
    GEN_AI_BROWSER_VIDEO_RECORDING = "gen_ai.browser.video.recording"
    GEN_AI_BROWSER_DOM_CHANGES_COUNT = "gen_ai.browser.dom.changes_count"
    GEN_AI_BROWSER_JAVASCRIPT_ERRORS = "gen_ai.browser.javascript.errors"
    GEN_AI_BROWSER_CONSOLE_LOGS = "gen_ai.browser.console.logs"
    GEN_AI_BROWSER_NETWORK_TIMING = "gen_ai.browser.network.timing"

    # Guard
    GUARD_REQUESTS = "guard.requests"
    GUARD_VERDICT = "guard.verdict"
    GUARD_SCORE = "guard.score"
    GUARD_CLASSIFICATION = "guard.classification"
    GUARD_VALIDATOR = "guard.validator"
    GUARD_EXPLANATION = "guard.explanation"

    # Evals
    EVAL_REQUESTS = "evals.requests"
    EVAL_VERDICT = "evals.verdict"
    EVAL_SCORE = "evals.score"
    EVAL_CLASSIFICATION = "evals.classification"
    EVAL_VALIDATOR = "evals.validator"
    EVAL_EXPLANATION = "evals.explanation"

    # === FRAMEWORK OPERATIONS (Generic attributes for all RAG/AI frameworks) ===

    # Document Processing
    GEN_AI_FRAMEWORK_DOCUMENTS_COUNT = "gen_ai.framework.documents.count"
    GEN_AI_FRAMEWORK_DOCUMENT_SOURCES = "gen_ai.framework.document.sources"
    GEN_AI_FRAMEWORK_DOCUMENT_SIZE = "gen_ai.framework.document.size"
    GEN_AI_FRAMEWORK_DOCUMENT_TYPE = "gen_ai.framework.document.type"

    # Text Processing & Chunking
    GEN_AI_FRAMEWORK_CHUNK_SIZE = "gen_ai.framework.chunk.size"
    GEN_AI_FRAMEWORK_CHUNK_OVERLAP = "gen_ai.framework.chunk.overlap"
    GEN_AI_FRAMEWORK_CHUNK_COUNT = "gen_ai.framework.chunk.count"
    GEN_AI_FRAMEWORK_TEXT_LENGTH = "gen_ai.framework.text.length"
    GEN_AI_FRAMEWORK_TEXT_PROCESSED = "gen_ai.framework.text.processed"

    # Node/Data Processing
    GEN_AI_FRAMEWORK_NODES_COUNT = "gen_ai.framework.nodes.count"
    GEN_AI_FRAMEWORK_NODES_PROCESSED = "gen_ai.framework.nodes.processed"
    GEN_AI_FRAMEWORK_NODES_CREATED = "gen_ai.framework.nodes.created"
    GEN_AI_FRAMEWORK_NODES_ADDED = "gen_ai.framework.nodes.added"
    GEN_AI_FRAMEWORK_NODES_INSERTED = "gen_ai.framework.nodes.inserted"
    GEN_AI_FRAMEWORK_NODES_DELETED = "gen_ai.framework.nodes.deleted"
    GEN_AI_FRAMEWORK_NODE_ID = "gen_ai.framework.node.id"

    # Embedding Operations
    GEN_AI_FRAMEWORK_EMBEDDING_DIMENSION = "gen_ai.framework.embedding.dimension"
    GEN_AI_FRAMEWORK_EMBEDDING_COUNT = "gen_ai.framework.embedding.count"
    GEN_AI_FRAMEWORK_EMBEDDING_BATCH_SIZE = "gen_ai.framework.embedding.batch_size"
    GEN_AI_FRAMEWORK_EMBEDDING_MODEL = "gen_ai.framework.embedding.model"
    GEN_AI_FRAMEWORK_EMBEDDING_PROCESSED = "gen_ai.framework.embedding.processed"

    # Query Operations
    GEN_AI_FRAMEWORK_QUERY_TYPE = "gen_ai.framework.query.type"
    GEN_AI_FRAMEWORK_QUERY_TEXT = "gen_ai.framework.query.text"
    GEN_AI_FRAMEWORK_QUERY_LENGTH = "gen_ai.framework.query.length"
    GEN_AI_FRAMEWORK_SIMILARITY_TOP_K = "gen_ai.framework.similarity.top_k"
    GEN_AI_FRAMEWORK_SIMILARITY_THRESHOLD = "gen_ai.framework.similarity.threshold"

    # Retrieval Operations
    GEN_AI_FRAMEWORK_RETRIEVAL_SOURCE = "gen_ai.framework.retrieval.source"
    GEN_AI_FRAMEWORK_RETRIEVAL_COUNT = "gen_ai.framework.retrieval.count"
    GEN_AI_FRAMEWORK_RETRIEVAL_METHOD = "gen_ai.framework.retrieval.method"

    # Response Generation
    GEN_AI_FRAMEWORK_RESPONSE_LENGTH = "gen_ai.framework.response.length"
    GEN_AI_FRAMEWORK_TEMPLATE_TYPE = "gen_ai.framework.template.type"
    GEN_AI_FRAMEWORK_CONTEXT_SIZE = "gen_ai.framework.context.size"
    GEN_AI_FRAMEWORK_CONTEXT_COUNT = "gen_ai.framework.context.count"

    # Processing Flags & Configuration
    GEN_AI_FRAMEWORK_SHOW_PROGRESS = "gen_ai.framework.show_progress"

    # Vector Store Operations (reuse DB attributes where appropriate)
    GEN_AI_FRAMEWORK_VECTOR_DIMENSION = "gen_ai.framework.vector.dimension"
    GEN_AI_FRAMEWORK_INDEX_NAME = "gen_ai.framework.index.name"
    GEN_AI_FRAMEWORK_INDEX_TYPE = "gen_ai.framework.index.type"

    # === GENERAL FRAMEWORK SEMANTIC CONVENTIONS (reusable across frameworks) ===

    # Framework tracing attributes (general, reusable across frameworks)
    GEN_AI_FRAMEWORK_TAGS = "gen_ai.framework.tags"

    # Framework performance tracking (general)
    GEN_AI_FRAMEWORK_PERFORMANCE_VS_BASELINE = (
        "gen_ai.framework.performance.vs_baseline"
    )
    GEN_AI_FRAMEWORK_PERFORMANCE_BASELINE_AVG = (
        "gen_ai.framework.performance.baseline_avg"
    )
    GEN_AI_FRAMEWORK_PERFORMANCE_BASELINE_PERCENTILE = (
        "gen_ai.framework.performance.baseline_percentile"
    )

    # Framework error classification (general)
    GEN_AI_FRAMEWORK_ERROR_CLASS = "gen_ai.framework.error.class"
    GEN_AI_FRAMEWORK_ERROR_TYPE = "gen_ai.framework.error.type"
    GEN_AI_FRAMEWORK_ERROR_MESSAGE = "gen_ai.framework.error.message"

    # Workflow attributes (general, reusable)
    GEN_AI_WORKFLOW_TYPE = "gen_ai.workflow.type"
    GEN_AI_WORKFLOW_INPUT = "gen_ai.workflow.input"
    GEN_AI_WORKFLOW_OUTPUT = "gen_ai.workflow.output"

    # Serialized function information (general, reusable)
    GEN_AI_SERIALIZED_NAME = "gen_ai.serialized.name"
    GEN_AI_SERIALIZED_SIGNATURE = "gen_ai.serialized.signature"
    GEN_AI_SERIALIZED_DOC = "gen_ai.serialized.doc"
    GEN_AI_SERIALIZED_MODULE = "gen_ai.serialized.module"

    # Tool operation attributes (general, reusable)
    GEN_AI_TOOL_INPUT = "gen_ai.tool.input"
    GEN_AI_TOOL_OUTPUT = "gen_ai.tool.output"

    # Retrieval operation attributes (general, reusable)
    GEN_AI_RETRIEVAL_QUERY = "gen_ai.retrieval.query"
    GEN_AI_RETRIEVAL_DOCUMENT_COUNT = "gen_ai.retrieval.document_count"
    GEN_AI_RETRIEVAL_DOCUMENTS = "gen_ai.retrieval.documents"

    # Provider information (general, reusable)
    GEN_AI_REQUEST_PROVIDER = "gen_ai.request.provider"

    # Enhanced token details (general, reusable across providers)
    GEN_AI_USAGE_COMPLETION_TOKENS_DETAILS_AUDIO = (
        "gen_ai.usage.completion_tokens_details.audio"
    )
    GEN_AI_USAGE_COMPLETION_TOKENS_DETAILS_REASONING = (
        "gen_ai.usage.completion_tokens_details.reasoning"
    )
    GEN_AI_USAGE_PROMPT_TOKENS_DETAILS_CACHE_READ = (
        "gen_ai.usage.prompt_tokens_details.cache_read"
    )
    GEN_AI_USAGE_PROMPT_TOKENS_DETAILS_CACHE_WRITE = (
        "gen_ai.usage.prompt_tokens_details.cache_write"
    )

    # === STANDARD OPENTELEMETRY SEMANTIC CONVENTIONS ===
    # These are framework-agnostic and reusable across all agent frameworks

    # OpenAI Agent-specific Attributes (for any framework using OpenAI models)
    GEN_AI_CONVERSATION_ID = "gen_ai.conversation.id"
    GEN_AI_OPENAI_ASSISTANT_ID = "gen_ai.openai.assistant.id"
    GEN_AI_OPENAI_THREAD_ID = "gen_ai.openai.thread.id"
    GEN_AI_OPENAI_RUN_ID = "gen_ai.openai.run.id"
    GEN_AI_OPENAI_REQUEST_SERVICE_TIER = "gen_ai.openai.request.service_tier"
    GEN_AI_OPENAI_RESPONSE_SERVICE_TIER = "gen_ai.openai.response.service_tier"
    GEN_AI_OPENAI_RESPONSE_SYSTEM_FINGERPRINT = (
        "gen_ai.openai.response.system_fingerprint"
    )

    # Data Source Attributes (for RAG and knowledge retrieval)
    GEN_AI_DATA_SOURCE_ID = "gen_ai.data_source.id"
    GEN_AI_DATA_SOURCE_TYPE = "gen_ai.data_source.type"

    # Standard Tool Attributes (framework-agnostic)
    GEN_AI_TOOL_TYPE = "gen_ai.tool.type"

    # Standard Workflow Attributes (framework-agnostic)
    GEN_AI_WORKFLOW_AGENT_COUNT = "gen_ai.workflow.agent_count"
    GEN_AI_WORKFLOW_TASK_COUNT = "gen_ai.workflow.task_count"
    GEN_AI_WORKFLOW_EXECUTION_TYPE = "gen_ai.workflow.execution_type"

    # Standard Task Attributes (framework-agnostic)
    GEN_AI_TASK_DESCRIPTION = "gen_ai.task.description"
    GEN_AI_TASK_EXPECTED_OUTPUT = "gen_ai.task.expected_output"

    # Julep-specific Task and Execution Attributes
    GEN_AI_TASK_ID = "gen_ai.task.id"
    GEN_AI_TASK_NAME = "gen_ai.task.name"
    GEN_AI_TASK_TYPE = "gen_ai.task.type"
    GEN_AI_TASK_TOOLS = "gen_ai.task.tools"
    GEN_AI_TASK_AGENT_ID = "gen_ai.task.agent_id"
    GEN_AI_EXECUTION_ID = "gen_ai.execution.id"
    GEN_AI_EXECUTION_STATUS = "gen_ai.execution.status"
    GEN_AI_EXECUTION_INPUT = "gen_ai.execution.input"
    GEN_AI_EXECUTION_OUTPUT = "gen_ai.execution.output"
    GEN_AI_EXECUTION_ERROR = "gen_ai.execution.error"

    GEN_AI_GROUPCHAT_PARTICIPANTS = "gen_ai.groupchat.participants"
    GEN_AI_GROUPCHAT_SPEAKER_SELECTION = "gen_ai.groupchat.speaker_selection"
    GEN_AI_GROUPCHAT_MESSAGE_COUNT = "gen_ai.groupchat.message_count"
    GEN_AI_GROUPCHAT_TURN_COUNT = "gen_ai.groupchat.turn_count"

    GEN_AI_AGENT_RECIPIENT = "gen_ai.agent.recipient"
    GEN_AI_AGENT_SENDER = "gen_ai.agent.sender"
    GEN_AI_AGENT_MESSAGE_TYPE = "gen_ai.agent.message_type"
    GEN_AI_AGENT_REPLY_MODE = "gen_ai.agent.reply_mode"

    # === ENHANCED SEMANTIC CONVENTIONS FOR COMPREHENSIVE INSTRUMENTATION ===

    # Message structure attributes (reuse existing prompt for input, add output messages)
    # Note: For input messages, we reuse GEN_AI_CONTENT_PROMPT for consistency
    GEN_AI_OUTPUT_MESSAGES = "gen_ai.output_messages"
    GEN_AI_MESSAGE_ROLE = "gen_ai.message.role"
    GEN_AI_MESSAGE_CONTENT = "gen_ai.message.content"

    # Tool result tracking (extending existing tool attributes)
    GEN_AI_TOOL_RESULT = "gen_ai.tool.result"
    GEN_AI_TOOL_SCHEMA = "gen_ai.tool.schema"

    # Model invocation parameters (for comprehensive model tracking)
    GEN_AI_REQUEST_PARAMETERS = "gen_ai.request.parameters"

    # Session and conversation tracking
    GEN_AI_SESSION_ID = "gen_ai.session.id"
    GEN_AI_USER_ID = "gen_ai.user.id"
    GEN_AI_RUN_ID = "gen_ai.run.id"

    # Memory operation types
    GEN_AI_OPERATION_TYPE_MEMORY = "memory"
    GEN_AI_OPERATION_TYPE_MEMORY_ADD = "memory_add"
    GEN_AI_OPERATION_TYPE_MEMORY_SEARCH = "memory_search"
    GEN_AI_OPERATION_TYPE_MEMORY_GET = "memory_get"
    GEN_AI_OPERATION_TYPE_MEMORY_UPDATE = "memory_update"
    GEN_AI_OPERATION_TYPE_MEMORY_DELETE = "memory_delete"

    # Memory-specific attributes
    GEN_AI_MEMORY_TYPE = "gen_ai.memory.type"
    GEN_AI_MEMORY_METADATA = "gen_ai.memory.metadata"
    GEN_AI_MEMORY_INFER = "gen_ai.memory.infer"
    GEN_AI_MEMORY_COUNT = "gen_ai.memory.count"
    GEN_AI_MEMORY_SEARCH_QUERY = "gen_ai.memory.search.query"
    GEN_AI_MEMORY_SEARCH_LIMIT = "gen_ai.memory.search.limit"
    GEN_AI_MEMORY_SEARCH_THRESHOLD = "gen_ai.memory.search.threshold"
    GEN_AI_MEMORY_OPERATION_RESULT_COUNT = "gen_ai.memory.operation.result_count"

    # Agent lifecycle phases
    GEN_AI_AGENT_LIFECYCLE_PHASE = "gen_ai.agent.lifecycle.phase"
    GEN_AI_AGENT_LIFECYCLE_PHASE_CREATE = "create"
    GEN_AI_AGENT_LIFECYCLE_PHASE_EXECUTE = "execute"
    GEN_AI_AGENT_LIFECYCLE_PHASE_GRAPH_EXECUTION = "graph_execution"
    GEN_AI_AGENT_LIFECYCLE_PHASE_USER_PROMPT_PROCESSING = "user_prompt_processing"
    GEN_AI_AGENT_LIFECYCLE_PHASE_MODEL_REQUEST = "model_request"
    GEN_AI_AGENT_LIFECYCLE_PHASE_TOOL_EXECUTION = "tool_execution"

    # Performance metrics (extending existing cost tracking)
    GEN_AI_PERFORMANCE_TOKENS_PER_SECOND = "gen_ai.performance.tokens_per_second"
    # Note: For latency/duration, we reuse existing GEN_AI_CLIENT_OPERATION_DURATION

    # Tool execution metadata
    GEN_AI_TOOL_EXECUTION_DURATION = "gen_ai.tool.execution.duration"
    GEN_AI_TOOL_EXECUTION_SUCCESS = "gen_ai.tool.execution.success"

    # Additional request parameters (general, reusable across integrations)
    GEN_AI_REQUEST_CONTEXT_WINDOW = "gen_ai.request.context_window"
    GEN_AI_REQUEST_ENABLE_REASONER = "gen_ai.request.enable_reasoner"
    GEN_AI_REQUEST_REASONING_EFFORT = "gen_ai.request.reasoning_effort"
    GEN_AI_REQUEST_ASYNC = "gen_ai.request.async"
    GEN_AI_REQUEST_RETURN_SEQUENCE_NO = "gen_ai.request.return_sequence_no"
    GEN_AI_REQUEST_INCLUDE_FINAL_MESSAGE = "gen_ai.request.include_final_message"
    GEN_AI_REQUEST_MESSAGE_COUNT = "gen_ai.request.message_count"

    # Model configuration attributes (general, reusable)
    GEN_AI_MODEL_HANDLE = "gen_ai.model.handle"

    # Agent attributes (general, reusable)
    GEN_AI_AGENT_SLUG = "gen_ai.agent.slug"
    GEN_AI_AGENT_STEP_MESSAGES = "gen_ai.agent.step_messages"

    # Streaming attributes (general, reusable)
    GEN_AI_STREAMING_CHUNK_COUNT = "gen_ai.streaming.chunk_count"
    GEN_AI_STREAMING_RESPONSE_COUNT = "gen_ai.streaming.response_count"

    # Agent execution and performance attributes (only truly new ones)
    GEN_AI_AGENT_INTRODUCTION = "gen_ai.agent.introduction"
    GEN_AI_AGENT_MEMORY_ENABLED = "gen_ai.agent.memory_enabled"
    GEN_AI_AGENT_EXECUTION_TIME = "gen_ai.agent.execution_time"
    GEN_AI_AGENT_SHOW_REASONING = "gen_ai.agent.show_reasoning"
    GEN_AI_AGENT_STREAM_INTERMEDIATE_STEPS = "gen_ai.agent.stream_intermediate_steps"
    # Reuse existing: GEN_AI_REQUEST_USER, GEN_AI_SESSION_ID, GEN_AI_REQUEST_IS_STREAM

    # Tool execution attributes (only truly new ones)
    GEN_AI_TOOL_DESCRIPTION = "gen_ai.tool.description"
    GEN_AI_TOOL_PARAMETERS = "gen_ai.tool.parameters"
    GEN_AI_TOOL_INPUT_KWARGS = "gen_ai.tool.input_kwargs"
    GEN_AI_TOOL_OUTPUT_TYPE = "gen_ai.tool.output_type"
    GEN_AI_TOOL_ERROR = "gen_ai.tool.error"

    # Toolkit attributes (new)
    GEN_AI_TOOLKIT_NAME = "gen_ai.toolkit.name"
    GEN_AI_TOOLKIT_FUNCTIONS = "gen_ai.toolkit.functions"
    GEN_AI_TOOLKIT_FUNCTION_COUNT = "gen_ai.toolkit.function_count"
    GEN_AI_TOOLKIT_EXECUTION_DURATION = "gen_ai.toolkit.execution.duration"

    # Memory operation attributes (only truly new ones)
    GEN_AI_MEMORY_OPERATION = "gen_ai.memory.operation"
    GEN_AI_MEMORY_SESSION_ID = "gen_ai.memory.session_id"
    GEN_AI_MEMORY_USER_ID = "gen_ai.memory.user_id"
    GEN_AI_MEMORY_RESULTS_COUNT = "gen_ai.memory.results_count"
    GEN_AI_MEMORY_DB_TYPE = "gen_ai.memory.db_type"
    GEN_AI_MEMORY_TABLE_NAME = "gen_ai.memory.table_name"
    GEN_AI_MEMORY_INPUT = "gen_ai.memory.input"
    GEN_AI_MEMORY_AGENT_ID = "gen_ai.memory.agent_id"
    GEN_AI_MEMORY_OPERATION_DURATION = "gen_ai.memory.operation.duration"
    GEN_AI_MEMORY_OPERATION_SUCCESS = "gen_ai.memory.operation.success"
    GEN_AI_MEMORY_RESULT_ID = "gen_ai.memory.result_id"
    GEN_AI_MEMORY_SEARCH_RESULTS_COUNT = "gen_ai.memory.search.results_count"
    GEN_AI_MEMORY_SEARCH_TOP_SCORES = "gen_ai.memory.search.top_scores"
    # Reuse existing: GEN_AI_MEMORY_METADATA, GEN_AI_MEMORY_SEARCH_QUERY, GEN_AI_MEMORY_SEARCH_LIMIT

    # VectorDB operation attributes (only truly new ones)
    GEN_AI_VECTORDB_NAME = "gen_ai.vectordb.name"
    GEN_AI_VECTORDB_DIMENSIONS = "gen_ai.vectordb.dimensions"
    GEN_AI_VECTORDB_SEARCH_QUERY = "gen_ai.vectordb.search.query"
    GEN_AI_VECTORDB_SEARCH_VECTOR_SIZE = "gen_ai.vectordb.search.vector_size"
    GEN_AI_VECTORDB_SEARCH_LIMIT = "gen_ai.vectordb.search.limit"
    GEN_AI_VECTORDB_SEARCH_RESULTS_COUNT = "gen_ai.vectordb.search.results_count"
    GEN_AI_VECTORDB_SEARCH_TOP_SCORES = "gen_ai.vectordb.search.top_scores"
    GEN_AI_VECTORDB_OPERATION_DURATION = "gen_ai.vectordb.operation.duration"
    GEN_AI_VECTORDB_OPERATION_SUCCESS = "gen_ai.vectordb.operation.success"
    GEN_AI_VECTORDB_UPSERT_DOCUMENT_COUNT = "gen_ai.vectordb.upsert.document_count"

    # Knowledge base operation attributes (only truly new ones)
    GEN_AI_KNOWLEDGE_SEARCH_QUERY = "gen_ai.knowledge.search.query"
    GEN_AI_KNOWLEDGE_SEARCH_LIMIT = "gen_ai.knowledge.search.limit"
    GEN_AI_KNOWLEDGE_SEARCH_RESULTS_COUNT = "gen_ai.knowledge.search.results_count"
    GEN_AI_KNOWLEDGE_ADD_DOCUMENT_COUNT = "gen_ai.knowledge.add.document_count"
    GEN_AI_KNOWLEDGE_ADD_CONTENT_LENGTH = "gen_ai.knowledge.add.content_length"
    GEN_AI_KNOWLEDGE_OPERATION_DURATION = "gen_ai.knowledge.operation.duration"
    GEN_AI_KNOWLEDGE_OPERATION_SUCCESS = "gen_ai.knowledge.operation.success"

    # Workflow operation attributes (additional ones not already covered above)
    GEN_AI_WORKFLOW_NAME = "gen_ai.workflow.name"
    GEN_AI_WORKFLOW_DESCRIPTION = "gen_ai.workflow.description"
    GEN_AI_WORKFLOW_EXECUTION_DURATION = "gen_ai.workflow.execution.duration"
    GEN_AI_WORKFLOW_OPERATION_SUCCESS = "gen_ai.workflow.operation.success"

    # Team operation attributes (additional ones not already covered above)
    GEN_AI_TEAM_NAME = "gen_ai.team.name"
    GEN_AI_TEAM_AGENTS = "gen_ai.team.agents"
    GEN_AI_TEAM_AGENT_COUNT = "gen_ai.team.agent_count"
    GEN_AI_TEAM_EXECUTION_DURATION = "gen_ai.team.execution.duration"
    GEN_AI_TEAM_OPERATION_SUCCESS = "gen_ai.team.operation.success"

    # Reasoning operation attributes (additional ones not already covered above)
    GEN_AI_REASONING_MIN_STEPS = "gen_ai.reasoning.min_steps"
    GEN_AI_REASONING_MAX_STEPS = "gen_ai.reasoning.max_steps"
    GEN_AI_REASONING_MODEL = "gen_ai.reasoning.model"
    GEN_AI_REASONING_EXECUTION_DURATION = "gen_ai.reasoning.execution_duration"

    # Note: Most workflow and team attributes already exist above, only add truly new ones if needed

    # === MCP (Model Context Protocol) SEMANTIC CONVENTIONS ===

    # MCP System
    GEN_AI_SYSTEM_MCP = "mcp"

    # MCP Operation Types
    GEN_AI_OPERATION_TYPE_MCP_TOOL_CALL = "mcp_tool_call"
    GEN_AI_OPERATION_TYPE_MCP_TOOL_LIST = "mcp_tool_list"
    GEN_AI_OPERATION_TYPE_MCP_RESOURCE_READ = "mcp_resource_read"
    GEN_AI_OPERATION_TYPE_MCP_RESOURCE_LIST = "mcp_resource_list"
    GEN_AI_OPERATION_TYPE_MCP_REQUEST = "mcp_request"
    GEN_AI_OPERATION_TYPE_MCP_RESPONSE = "mcp_response"
    GEN_AI_OPERATION_TYPE_MCP_SERVER = "mcp_server"
    GEN_AI_OPERATION_TYPE_MCP_CLIENT = "mcp_client"

    # MCP Request/Response Attributes (using mcp.* namespace)
    MCP_METHOD = "mcp.method"
    MCP_MESSAGE_ID = "mcp.message_id"
    MCP_JSONRPC_VERSION = "mcp.jsonrpc_version"
    MCP_PARAMS = "mcp.params"
    MCP_RESULT = "mcp.result"
    MCP_ERROR_CODE = "mcp.error.code"
    MCP_ERROR_MESSAGE = "mcp.error.message"
    MCP_ERROR_DATA = "mcp.error.data"

    # MCP Tool Attributes
    MCP_TOOL_NAME = "mcp.tool.name"
    MCP_TOOL_DESCRIPTION = "mcp.tool.description"
    MCP_TOOL_ARGUMENTS = "mcp.tool.arguments"
    MCP_TOOL_RESULT = "mcp.tool.result"

    # MCP Resource Attributes
    MCP_RESOURCE_URI = "mcp.resource.uri"
    MCP_RESOURCE_NAME = "mcp.resource.name"
    MCP_RESOURCE_DESCRIPTION = "mcp.resource.description"
    MCP_RESOURCE_MIME_TYPE = "mcp.resource.mime_type"
    MCP_RESOURCE_SIZE = "mcp.resource.size"

    # MCP Transport Attributes
    MCP_TRANSPORT_TYPE = "mcp.transport.type"
    MCP_TRANSPORT_STDIO = "stdio"
    MCP_TRANSPORT_SSE = "sse"
    MCP_TRANSPORT_WEBSOCKET = "websocket"

    # MCP Request/Response Payload Attributes
    MCP_REQUEST_PAYLOAD = "mcp.request.payload"
    MCP_RESPONSE_PAYLOAD = "mcp.response.payload"

    # MCP Core Attributes
    MCP_OPERATION = "mcp.operation.name"
    MCP_SYSTEM = "mcp.system"
    MCP_SDK_VERSION = "mcp.sdk.version"
    MCP_CLIENT_OPERATION_DURATION = "mcp.client.operation.duration"

    # MCP Prompt Attributes
    MCP_PROMPT_NAME = "mcp.prompt.name"
    MCP_PROMPT_DESCRIPTION = "mcp.prompt.description"

    # MCP Server/Client Attributes
    MCP_SERVER_NAME = "mcp.server.name"
    MCP_SERVER_VERSION = "mcp.server.version"
    MCP_CLIENT_VERSION = "mcp.client.version"
    MCP_CLIENT_TYPE = "mcp.client.type"
    MCP_RESPONSE_SIZE = "mcp.response.size"

    # MCP Metrics (for business intelligence and operational insights)
    MCP_REQUESTS = "mcp.requests"
    MCP_CLIENT_OPERATION_DURATION_METRIC = "mcp.client.operation.duration"
    MCP_REQUEST_SIZE = "mcp.request.size"
    MCP_RESPONSE_SIZE_METRIC = "mcp.response.size"
    MCP_TOOL_CALLS = "mcp.tool.calls"
    MCP_RESOURCE_READS = "mcp.resource.reads"
    MCP_PROMPT_GETS = "mcp.prompt.gets"
    MCP_TRANSPORT_USAGE = "mcp.transport.usage"
    MCP_ERRORS = "mcp.errors"
    MCP_OPERATION_SUCCESS_RATE = "mcp.operation.success_rate"

    # === NEW FASTMCP FRAMEWORK ATTRIBUTES ===
    MCP_FASTMCP_SERVER_DEBUG_MODE = "mcp.fastmcp.server.debug_mode"
    MCP_FASTMCP_SERVER_LOG_LEVEL = "mcp.fastmcp.server.log_level"
    MCP_FASTMCP_SERVER_HOST = "mcp.fastmcp.server.host"
    MCP_FASTMCP_SERVER_PORT = "mcp.fastmcp.server.port"
    MCP_FASTMCP_SERVER_TRANSPORT = "mcp.fastmcp.server.transport"
    MCP_FASTMCP_TOOL_ANNOTATIONS = "mcp.fastmcp.tool.annotations"
    MCP_FASTMCP_RESOURCE_MIME_TYPE = "mcp.fastmcp.resource.mime_type"
    MCP_FASTMCP_PROMPT_ARGUMENTS = "mcp.fastmcp.prompt.arguments"
    MCP_FASTMCP_TOOL_STRUCTURED_OUTPUT = "mcp.fastmcp.tool.structured_output"
    MCP_FASTMCP_SERVER_INSTRUCTIONS = "mcp.fastmcp.server.instructions"
    MCP_FASTMCP_SERVER_LIFESPAN = "mcp.fastmcp.server.lifespan"
    MCP_FASTMCP_MOUNT_PATH = "mcp.fastmcp.mount_path"
    MCP_FASTMCP_SSE_PATH = "mcp.fastmcp.sse_path"
    MCP_FASTMCP_MESSAGE_PATH = "mcp.fastmcp.message_path"
    MCP_FASTMCP_STREAMABLE_HTTP_PATH = "mcp.fastmcp.streamable_http_path"
    MCP_FASTMCP_JSON_RESPONSE = "mcp.fastmcp.json_response"
    MCP_FASTMCP_STATELESS_HTTP = "mcp.fastmcp.stateless_http"

    # === NEW AUTHENTICATION & SECURITY ATTRIBUTES ===
    MCP_AUTH_CLIENT_ID = "mcp.auth.client_id"
    MCP_AUTH_SCOPES = "mcp.auth.scopes"
    MCP_AUTH_GRANT_TYPE = "mcp.auth.grant_type"
    MCP_AUTH_TOKEN_TYPE = "mcp.auth.token_type"
    MCP_AUTH_EXPIRES_AT = "mcp.auth.expires_at"
    MCP_AUTH_AUTHORIZATION_CODE = "mcp.auth.authorization_code"
    MCP_AUTH_REDIRECT_URI = "mcp.auth.redirect_uri"
    MCP_AUTH_STATE = "mcp.auth.state"
    MCP_AUTH_CODE_CHALLENGE = "mcp.auth.code_challenge"
    MCP_AUTH_RESOURCE_INDICATOR = "mcp.auth.resource_indicator"
    MCP_SECURITY_TRANSPORT_SECURITY = "mcp.security.transport_security"
    MCP_SECURITY_DNS_REBINDING_PROTECTION = "mcp.security.dns_rebinding_protection"
    MCP_AUTH_TOKEN_VERIFICATION_SUCCESS = "mcp.auth.token_verification_success"
    MCP_AUTH_CLIENT_REGISTRATION_TYPE = "mcp.auth.client_registration_type"

    # === NEW ADVANCED SESSION ATTRIBUTES ===
    MCP_SESSION_REQUEST_TIMEOUT = "mcp.session.request_timeout"
    MCP_SESSION_PROGRESS_TOKEN = "mcp.session.progress_token"
    MCP_SESSION_ELICITATION_SUPPORT = "mcp.session.elicitation_support"
    MCP_SESSION_SAMPLING_SUPPORT = "mcp.session.sampling_support"
    MCP_SESSION_ROOTS_SUPPORT = "mcp.session.roots_support"
    MCP_SESSION_READ_TIMEOUT = "mcp.session.read_timeout"
    MCP_SESSION_STATELESS = "mcp.session.stateless"
    MCP_SESSION_RAISE_EXCEPTIONS = "mcp.session.raise_exceptions"
    MCP_SESSION_CLIENT_INFO_NAME = "mcp.session.client_info.name"
    MCP_SESSION_CLIENT_INFO_VERSION = "mcp.session.client_info.version"
    MCP_SESSION_COMPLETION_SUPPORT = "mcp.session.completion_support"
    MCP_SESSION_LOGGING_SUPPORT = "mcp.session.logging_support"

    # === NEW WEBSOCKET SPECIFIC ATTRIBUTES ===
    MCP_WEBSOCKET_SUBPROTOCOL = "mcp.websocket.subprotocol"
    MCP_WEBSOCKET_URL = "mcp.websocket.url"
    MCP_WEBSOCKET_CONNECTION_STATE = "mcp.websocket.connection_state"

    # === NEW PERFORMANCE & RELIABILITY ATTRIBUTES ===
    MCP_TOOL_EXECUTION_TIME = "mcp.tool.execution_time"
    MCP_RESOURCE_READ_TIME = "mcp.resource.read_time"
    MCP_PROMPT_RENDER_TIME = "mcp.prompt.render_time"
    MCP_TRANSPORT_CONNECTION_TIME = "mcp.transport.connection_time"
    MCP_PROGRESS_COMPLETION_PERCENTAGE = "mcp.progress.completion_percentage"
    MCP_PROGRESS_TOTAL = "mcp.progress.total"
    MCP_PROGRESS_MESSAGE = "mcp.progress.message"
    MCP_ELICITATION_ACTION = "mcp.elicitation.action"
    MCP_ELICITATION_SCHEMA = "mcp.elicitation.schema"
    MCP_SAMPLING_MAX_TOKENS = "mcp.sampling.max_tokens"
    MCP_SAMPLING_MESSAGES = "mcp.sampling.messages"

    # === NEW MANAGER-LEVEL ATTRIBUTES ===
    MCP_MANAGER_TYPE = "mcp.manager.type"
    MCP_MANAGER_OPERATION = "mcp.manager.operation"
    MCP_TOOL_MANAGER_TOOL_COUNT = "mcp.tool_manager.tool_count"
    MCP_RESOURCE_MANAGER_RESOURCE_COUNT = "mcp.resource_manager.resource_count"
    MCP_PROMPT_MANAGER_PROMPT_COUNT = "mcp.prompt_manager.prompt_count"
    MCP_TOOL_MANAGER_WARN_DUPLICATES = "mcp.tool_manager.warn_duplicates"
    MCP_RESOURCE_MANAGER_WARN_DUPLICATES = "mcp.resource_manager.warn_duplicates"
    MCP_PROMPT_MANAGER_WARN_DUPLICATES = "mcp.prompt_manager.warn_duplicates"

    # === NEW MEMORY & PROGRESS SPECIFIC ATTRIBUTES ===
    MCP_MEMORY_TRANSPORT_TYPE = "mcp.memory.transport_type"
    MCP_MEMORY_CLIENT_SERVER_SESSION = "mcp.memory.client_server_session"
    MCP_PROGRESS_CONTEXT_CURRENT = "mcp.progress_context.current"
    MCP_PROGRESS_CONTEXT_TOTAL = "mcp.progress_context.total"

    # === NEW COMPLETION ATTRIBUTES ===
    MCP_COMPLETION_REF_TYPE = "mcp.completion.ref.type"
    MCP_COMPLETION_ARGUMENT_NAME = "mcp.completion.argument.name"
    MCP_COMPLETION_ARGUMENT_VALUE = "mcp.completion.argument.value"
    MCP_COMPLETION_CONTEXT_ARGUMENTS = "mcp.completion.context.arguments"
    MCP_COMPLETION_VALUES = "mcp.completion.values"
    MCP_COMPLETION_TOTAL = "mcp.completion.total"
    MCP_COMPLETION_HAS_MORE = "mcp.completion.has_more"

    # === NEW ADVANCED OPERATION ATTRIBUTES ===
    MCP_PING_RESPONSE_TIME = "mcp.ping.response_time"
    MCP_LOGGING_LEVEL_SET = "mcp.logging.level_set"
    MCP_NOTIFICATION_TYPE = "mcp.notification.type"
    MCP_NOTIFICATION_RELATED_REQUEST_ID = "mcp.notification.related_request_id"
