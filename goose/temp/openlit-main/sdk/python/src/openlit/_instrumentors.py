"""
OpenLIT Instrumentors
"""

import importlib

# Mapping of instrumentor names to their required Python packages
MODULE_NAME_MAP = {
    # OpenLIT AI/ML instrumentations
    "openai": "openai",
    "anthropic": "anthropic",
    "cohere": "cohere",
    "mistral": "mistralai",
    "bedrock": "boto3",
    "vertexai": "vertexai",
    "groq": "groq",
    "ollama": "ollama",
    "gpt4all": "gpt4all",
    "elevenlabs": "elevenlabs",
    "vllm": "vllm",
    "google-ai-studio": "google.genai",
    "azure-ai-inference": "azure.ai.inference",
    "langchain": "langchain_core",
    "langgraph": "langgraph",
    "llama_index": "llama_index",
    "haystack": "haystack",
    "mem0": "mem0",
    "chroma": "chromadb",
    "pinecone": "pinecone",
    "qdrant": "qdrant_client",
    "milvus": "pymilvus",
    "transformers": "transformers",
    "litellm": "litellm",
    "crewai": "crewai",
    "ag2": "ag2",
    "autogen": "autogen",
    "pyautogen": "pyautogen",
    "multion": "multion",
    "dynamiq": "dynamiq",
    "agno": "agno",
    "reka-api": "reka",
    "premai": "premai",
    "julep": "julep",
    "astra": "astrapy",
    "ai21": "ai21",
    "controlflow": "controlflow",
    "assemblyai": "assemblyai",
    "crawl4ai": "crawl4ai",
    "firecrawl": "firecrawl",
    "letta": "letta_client",
    "together": "together",
    "openai-agents": "agents",
    "pydantic_ai": "pydantic_ai",
    "sarvam": "sarvamai",
    "browser-use": "browser_use",
    "mcp": "mcp",
    # Database instrumentations
    "psycopg": "psycopg",
    "psycopg-pool": "psycopg_pool",
    # Official OpenTelemetry HTTP Framework instrumentations
    "asgi": "asgiref",
    "django": "django",
    "fastapi": "fastapi",
    "flask": "flask",
    "pyramid": "pyramid",
    "starlette": "starlette",
    "falcon": "falcon",
    "tornado": "tornado",
    # Official OpenTelemetry HTTP Client instrumentations
    "aiohttp-client": "aiohttp",
    "httpx": "httpx",
    "requests": "requests",
    "urllib": "urllib",
    "urllib3": "urllib3",
}

# Dictionary mapping instrumentor names to their full module paths
INSTRUMENTOR_MAP = {
    # OpenLIT AI/ML instrumentations
    "openai": "openlit.instrumentation.openai.OpenAIInstrumentor",
    "anthropic": "openlit.instrumentation.anthropic.AnthropicInstrumentor",
    "cohere": "openlit.instrumentation.cohere.CohereInstrumentor",
    "mistral": "openlit.instrumentation.mistral.MistralInstrumentor",
    "bedrock": "openlit.instrumentation.bedrock.BedrockInstrumentor",
    "vertexai": "openlit.instrumentation.vertexai.VertexAIInstrumentor",
    "groq": "openlit.instrumentation.groq.GroqInstrumentor",
    "ollama": "openlit.instrumentation.ollama.OllamaInstrumentor",
    "gpt4all": "openlit.instrumentation.gpt4all.GPT4AllInstrumentor",
    "elevenlabs": "openlit.instrumentation.elevenlabs.ElevenLabsInstrumentor",
    "vllm": "openlit.instrumentation.vllm.VLLMInstrumentor",
    "google-ai-studio": "openlit.instrumentation.google_ai_studio.GoogleAIStudioInstrumentor",
    "azure-ai-inference": "openlit.instrumentation.azure_ai_inference.AzureAIInferenceInstrumentor",
    "langchain": "openlit.instrumentation.langchain.LangChainInstrumentor",
    "langgraph": "openlit.instrumentation.langgraph.LangGraphInstrumentor",
    "llama_index": "openlit.instrumentation.llamaindex.LlamaIndexInstrumentor",
    "haystack": "openlit.instrumentation.haystack.HaystackInstrumentor",
    "mem0": "openlit.instrumentation.mem0.Mem0Instrumentor",
    "chroma": "openlit.instrumentation.chroma.ChromaInstrumentor",
    "pinecone": "openlit.instrumentation.pinecone.PineconeInstrumentor",
    "qdrant": "openlit.instrumentation.qdrant.QdrantInstrumentor",
    "milvus": "openlit.instrumentation.milvus.MilvusInstrumentor",
    "transformers": "openlit.instrumentation.transformers.TransformersInstrumentor",
    "litellm": "openlit.instrumentation.litellm.LiteLLMInstrumentor",
    "crewai": "openlit.instrumentation.crewai.CrewAIInstrumentor",
    "ag2": "openlit.instrumentation.ag2.AG2Instrumentor",
    "multion": "openlit.instrumentation.multion.MultiOnInstrumentor",
    "autogen": "openlit.instrumentation.ag2.AG2Instrumentor",
    "pyautogen": "openlit.instrumentation.ag2.AG2Instrumentor",
    "dynamiq": "openlit.instrumentation.dynamiq.DynamiqInstrumentor",
    "agno": "openlit.instrumentation.agno.AgnoInstrumentor",
    "reka-api": "openlit.instrumentation.reka.RekaInstrumentor",
    "premai": "openlit.instrumentation.premai.PremAIInstrumentor",
    "julep": "openlit.instrumentation.julep.JulepInstrumentor",
    "astra": "openlit.instrumentation.astra.AstraInstrumentor",
    "ai21": "openlit.instrumentation.ai21.AI21Instrumentor",
    "controlflow": "openlit.instrumentation.controlflow.ControlFlowInstrumentor",
    "assemblyai": "openlit.instrumentation.assemblyai.AssemblyAIInstrumentor",
    "crawl4ai": "openlit.instrumentation.crawl4ai.Crawl4AIInstrumentor",
    "firecrawl": "openlit.instrumentation.firecrawl.FireCrawlInstrumentor",
    "letta": "openlit.instrumentation.letta.LettaInstrumentor",
    "together": "openlit.instrumentation.together.TogetherInstrumentor",
    "openai-agents": "openlit.instrumentation.openai_agents.OpenAIAgentsInstrumentor",
    "pydantic_ai": "openlit.instrumentation.pydantic_ai.PydanticAIInstrumentor",
    "sarvam": "openlit.instrumentation.sarvam.SarvamInstrumentor",
    "browser-use": "openlit.instrumentation.browser_use.BrowserUseInstrumentor",
    "mcp": "openlit.instrumentation.mcp.MCPInstrumentor",
    # Database instrumentations
    "psycopg": "openlit.instrumentation.psycopg.PsycopgInstrumentor",
    "psycopg-pool": "openlit.instrumentation.psycopg.PsycopgInstrumentor",
    # Official OpenTelemetry HTTP Framework instrumentations
    "asgi": "opentelemetry.instrumentation.asgi.AsgiInstrumentor",
    "django": "opentelemetry.instrumentation.django.DjangoInstrumentor",
    "fastapi": "opentelemetry.instrumentation.fastapi.FastAPIInstrumentor",
    "flask": "opentelemetry.instrumentation.flask.FlaskInstrumentor",
    "pyramid": "opentelemetry.instrumentation.pyramid.PyramidInstrumentor",
    "starlette": "opentelemetry.instrumentation.starlette.StarletteInstrumentor",
    "falcon": "opentelemetry.instrumentation.falcon.FalconInstrumentor",
    "tornado": "opentelemetry.instrumentation.tornado.TornadoInstrumentor",
    # Official OpenTelemetry HTTP Client instrumentations
    "aiohttp-client": "opentelemetry.instrumentation.aiohttp_client.AioHttpClientInstrumentor",
    "httpx": "opentelemetry.instrumentation.httpx.HTTPXClientInstrumentor",
    "requests": "opentelemetry.instrumentation.requests.RequestsInstrumentor",
    "urllib": "opentelemetry.instrumentation.urllib.URLLibInstrumentor",
    "urllib3": "opentelemetry.instrumentation.urllib3.URLLib3Instrumentor",
}


def get_instrumentor_class(name):
    """
    Get instrumentor class by name.

    Args:
        name (str): Name of the instrumentor

    Returns:
        class: Instrumentor class or None if not found
    """
    if name not in INSTRUMENTOR_MAP:
        return None

    module_path, class_name = INSTRUMENTOR_MAP[name].rsplit(".", 1)

    try:
        module = importlib.import_module(module_path)
        return getattr(module, class_name)
    except (ImportError, AttributeError):
        return None


def get_all_instrumentors():
    """
    Get all available instrumentor instances.

    Returns:
        dict: Dictionary of instrumentor instances
    """
    instances = {}

    for name in INSTRUMENTOR_MAP:
        instrumentor_class = get_instrumentor_class(name)
        if instrumentor_class:
            try:
                instances[name] = instrumentor_class()
            except Exception:
                pass  # Skip instrumentors that fail to instantiate

    return instances
