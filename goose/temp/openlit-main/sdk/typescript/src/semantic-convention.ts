export default class SemanticConvention {
  // Unstable SemConv
  static ATTR_DEPLOYMENT_ENVIRONMENT = 'deployment.environment'
  // GenAI General
  static GEN_AI_ENDPOINT = 'gen_ai.endpoint';
  static GEN_AI_SYSTEM = 'gen_ai.system';
  static GEN_AI_ENVIRONMENT = 'gen_ai.environment';
  static GEN_AI_APPLICATION_NAME = 'gen_ai.application_name';
  static GEN_AI_OPERATION = 'gen_ai.operation.name';
  static GEN_AI_HUB_OWNER = 'gen_ai.hub.owner';
  static GEN_AI_HUB_REPO = 'gen_ai.hub.repo';
  static GEN_AI_RETRIEVAL_SOURCE = 'gen_ai.retrieval.source';
  static GEN_AI_REQUESTS = 'gen_ai.total.requests';
  static GEN_AI_SDK_VERSION = 'gen_ai.sdk.version';
  static GEN_AI_OUTPUT_TYPE = 'gen_ai.output.type';
  

  // GenAI Request
  static GEN_AI_REQUEST_MODEL = 'gen_ai.request.model';
  static GEN_AI_REQUEST_TEMPERATURE = 'gen_ai.request.temperature';
  static GEN_AI_REQUEST_TOP_P = 'gen_ai.request.top_p';
  static GEN_AI_REQUEST_TOP_K = 'gen_ai.request.top_k';
  static GEN_AI_REQUEST_MAX_TOKENS = 'gen_ai.request.max_tokens';
  static GEN_AI_REQUEST_IS_STREAM = 'gen_ai.request.is_stream';
  static GEN_AI_REQUEST_USER = 'gen_ai.request.user';
  static GEN_AI_REQUEST_SEED = 'gen_ai.request.seed';
  static GEN_AI_REQUEST_FREQUENCY_PENALTY = 'gen_ai.request.frequency_penalty';
  static GEN_AI_REQUEST_PRESENCE_PENALTY = 'gen_ai.request.presence_penalty';
  static GEN_AI_REQUEST_STOP_SEQUENCES = 'gen_ai.request.stop_sequences';
  static GEN_AI_REQUEST_ENCODING_FORMATS = 'gen_ai.request.encoding_formats';
  static GEN_AI_REQUEST_EMBEDDING_DIMENSION = 'gen_ai.request.embedding_dimension';
  static GEN_AI_REQUEST_TOOL_CHOICE = 'gen_ai.request.tool_choice';
  static GEN_AI_REQUEST_AUDIO_VOICE = 'gen_ai.request.audio_voice';
  static GEN_AI_REQUEST_AUDIO_RESPONSE_FORMAT = 'gen_ai.request.audio_response_format';
  static GEN_AI_REQUEST_AUDIO_SPEED = 'gen_ai.request.audio_speed';
  static GEN_AI_REQUEST_FINETUNE_STATUS = 'gen_ai.request.fine_tune_status';
  static GEN_AI_REQUEST_FINETUNE_MODEL_SUFFIX = 'gen_ai.request.fine_tune_model_suffix';
  static GEN_AI_REQUEST_FINETUNE_MODEL_EPOCHS = 'gen_ai.request.fine_tune_n_epochs';
  static GEN_AI_REQUEST_FINETUNE_MODEL_LRM = 'gen_ai.request.learning_rate_multiplier';
  static GEN_AI_REQUEST_FINETUNE_BATCH_SIZE = 'gen_ai.request.fine_tune_batch_size';
  static GEN_AI_REQUEST_VALIDATION_FILE = 'gen_ai.request.validation_file';
  static GEN_AI_REQUEST_TRAINING_FILE = 'gen_ai.request.training_file';

  static GEN_AI_REQUEST_IMAGE_SIZE = 'gen_ai.request.image_size';
  static GEN_AI_REQUEST_IMAGE_QUALITY = 'gen_ai.request.image_quality';
  static GEN_AI_REQUEST_IMAGE_STYLE = 'gen_ai.request.image_style';

  // GenAI Usage
  static GEN_AI_USAGE_INPUT_TOKENS = 'gen_ai.usage.input_tokens';
  static GEN_AI_USAGE_OUTPUT_TOKENS = 'gen_ai.usage.output_tokens';
  static GEN_AI_USAGE_TOTAL_TOKENS = 'gen_ai.usage.total_tokens';
  static GEN_AI_USAGE_COST = 'gen_ai.usage.cost';
  static GEN_AI_USAGE_REASONING_TOKENS = 'gen_ai.usage.reasoning_tokens';
  
  // Enhanced token details (for prompt caching, audio tokens, etc.)
  static GEN_AI_USAGE_COMPLETION_TOKENS_DETAILS_AUDIO = 'gen_ai.usage.completion_tokens_details.audio';
  static GEN_AI_USAGE_COMPLETION_TOKENS_DETAILS_REASONING = 'gen_ai.usage.completion_tokens_details.reasoning';
  static GEN_AI_USAGE_PROMPT_TOKENS_DETAILS_CACHE_READ = 'gen_ai.usage.prompt_tokens_details.cache_read';
  static GEN_AI_USAGE_PROMPT_TOKENS_DETAILS_CACHE_WRITE = 'gen_ai.usage.prompt_tokens_details.cache_write';

  // GenAI Response
  static GEN_AI_RESPONSE_ID = 'gen_ai.response.id';
  static GEN_AI_RESPONSE_MODEL = 'gen_ai.response.model';
  static GEN_AI_RESPONSE_FINISH_REASON = 'gen_ai.response.finish_reasons';
  static GEN_AI_RESPONSE_IMAGE = 'gen_ai.response.image';
  static GEN_AI_RESPONSE_IMAGE_SIZE = 'gen_ai.request.image_size';
  static GEN_AI_RESPONSE_IMAGE_QUALITY = 'gen_ai.request.image_quality';
  static GEN_AI_RESPONSE_IMAGE_STYLE = 'gen_ai.request.image_style';
  
  // OpenAI-specific attributes
  static GEN_AI_REQUEST_SERVICE_TIER = 'gen_ai.request.service_tier';
  static GEN_AI_RESPONSE_SERVICE_TIER = 'gen_ai.response.service_tier';
  static GEN_AI_RESPONSE_SYSTEM_FINGERPRINT = 'gen_ai.response.system_fingerprint';

  // GenAI Content
  static GEN_AI_CONTENT_PROMPT = 'gen_ai.prompt';
  static GEN_AI_CONTENT_PROMPT_EVENT = 'gen_ai.content.prompt';
  static GEN_AI_CONTENT_COMPLETION = 'gen_ai.completion';
  static GEN_AI_CONTENT_COMPLETION_EVENT = 'gen_ai.content.completion';
  static GEN_AI_CONTENT_REVISED_PROMPT = 'gen_ai.content.revised_prompt';
  
  // Tool attributes
  static GEN_AI_TOOL_NAME = 'gen_ai.tool.name';
  static GEN_AI_TOOL_TYPE = 'gen_ai.tool.call.type';
  static GEN_AI_TOOL_DESCRIPTION = 'gen_ai.tool.description';
  static GEN_AI_TOOL_DEFINITION = 'gen_ai.tool.definition';
  static GEN_AI_TOOL_CALL_ID = 'gen_ai.tool.call.id';
  static GEN_AI_TOOL_CALL_ARGUMENTS = 'gen_ai.tool.call.arguments';
  static GEN_AI_TOOL_CALL_RESULT = 'gen_ai.tool.call.result';


  static GEN_AI_TOKEN_TYPE = 'gen_ai.token.type';

  static GEN_AI_OPERATION_TYPE_TEXT_COMPLETION = 'text_completion';
  static GEN_AI_OPERATION_TYPE_CHAT = 'chat';
  static GEN_AI_OPERATION_TYPE_EMBEDDING = 'embeddings';
  static GEN_AI_OPERATION_TYPE_IMAGE = 'image';
  static GEN_AI_OPERATION_TYPE_AUDIO = 'audio';
  static GEN_AI_OPERATION_TYPE_FINETUNING = 'fine_tuning';
  static GEN_AI_OPERATION_TYPE_VECTORDB = 'vectordb';
  static GEN_AI_OPERATION_TYPE_FRAMEWORK = 'workflow';
  
  // GenAI Output Types
  static GEN_AI_OUTPUT_TYPE_TEXT = 'text';
  static GEN_AI_OUTPUT_TYPE_JSON = 'json';
  static GEN_AI_OUTPUT_TYPE_IMAGE = 'image';
  static GEN_AI_OUTPUT_TYPE_SPEECH = 'speech';

  static GEN_AI_SYSTEM_HUGGING_FACE = 'huggingface';
  static GEN_AI_SYSTEM_OPENAI = 'openai';
  static GEN_AI_SYSTEM_AZURE_OPENAI = 'az.ai.openai';
  static GEN_AI_SYSTEM_ANTHROPIC = 'anthropic';
  static GEN_AI_SYSTEM_COHERE = 'cohere';
  static GEN_AI_SYSTEM_MISTRAL = 'mistral_ai';
  static GEN_AI_SYSTEM_AWS_BEDROCK = 'aws.bedrock';
  static GEN_AI_SYSTEM_VERTEXAI = 'vertex_ai';
  static GEN_AI_SYSTEM_LANGCHAIN = 'langchain';

  // Vector DB
  static DB_REQUESTS = 'db.total.requests';
  static DB_SYSTEM = 'db.system';
  static DB_SYSTEM_CHROMA = 'chroma';
  static DB_SYSTEM_PINECONE = 'pinecone';
  static DB_COLLECTION_NAME = 'db.collection.name';
  static DB_OPERATION = 'db.operation';
  static DB_OPERATION_CREATE_INDEX = 'create_index';
  static DB_OPERATION_QUERY = 'query';
  static DB_OPERATION_DELETE = 'delete';
  static DB_OPERATION_UPDATE = 'update';
  static DB_OPERATION_UPSERT = 'upsert';
  static DB_OPERATION_GET = 'get';
  static DB_OPERATION_ADD = 'add';
  static DB_OPERATION_PEEK = 'peek';
  static DB_ID_COUNT = 'db.ids_count';
  static DB_VECTOR_COUNT = 'db.vector_count';
  static DB_METADATA_COUNT = 'db.metadatas_count';
  static DB_DOCUMENTS_COUNT = 'db.documents_count';
  static DB_QUERY_LIMIT = 'db.limit';
  static DB_OFFSET = 'db.offset';
  static DB_WHERE_DOCUMENT = 'db.where_document';
  static DB_FILTER = 'db.filter';
  static DB_STATEMENT = 'db.statement';
  static DB_N_RESULTS = 'db.n_results';
  static DB_DELETE_ALL = 'db.delete_all';
  static DB_INDEX_NAME = 'db.create_index.name';
  static DB_INDEX_DIMENSION = 'db.create_index.dimensions';
  static DB_INDEX_METRIC = 'db.create_index.metric';
  static DB_INDEX_SPEC = 'db.create_index.spec';
  static DB_NAMESPACE = 'db.query.namespace';
  static DB_UPDATE_METADATA = 'db.update.metadata';
  static DB_UPDATE_VALUES = 'db.update.values';
  static DB_UPDATE_ID = 'db.update.id';
  static GEN_AI_CLIENT_OPERATION_DURATION = 'gen_ai.client.operation.duration';
  static GEN_AI_CLIENT_TOKEN_USAGE = 'gen_ai.client.token.usage';
  static GEN_AI_SERVER_TBT = 'gen_ai.server.time_per_output_token';
  static GEN_AI_SERVER_TTFT = 'gen_ai.server.time_to_first_token';
  static DB_CLIENT_OPERATION_DURATION = 'db.client.operation.duration';
  
  // Server attributes
  static SERVER_ADDRESS = 'server.address';
  static SERVER_PORT = 'server.port';
  static ERROR_TYPE = 'error.type';
}
