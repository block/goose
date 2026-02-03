# evolving_agents/core/llm_service.py

import logging
import os
import json
import hashlib
import time
import asyncio 
from datetime import datetime, timezone, timedelta  # Added timedelta for clear_cache
from typing import Dict, Any, List, Optional, Union

import pymongo

from beeai_framework.backend.chat import ChatModel
from beeai_framework.backend.message import UserMessage
from beeai_framework.adapters.litellm.chat import LiteLLMChatModel
from beeai_framework.backend.constants import ProviderName

from evolving_agents.core.mongodb_client import MongoDBClient

logger = logging.getLogger(__name__)

# Utility function for handling non-serializable types in JSON operations
def json_serializable(obj):
    """Convert an object to a JSON-serializable format."""
    if isinstance(obj, datetime):
        return obj.isoformat()
    if isinstance(obj, set):
        return list(obj)
    try:
        # Try using the default JSON encoder
        json.dumps(obj)
        return obj
    except (TypeError, OverflowError):
        # For objects with their own serialization method
        if hasattr(obj, 'to_dict') and callable(getattr(obj, 'to_dict')):
            return obj.to_dict()
        # Last resort: convert to string
        return str(obj)

class LLMCache:
    """
    Cache for LLM completions and embeddings using MongoDB with TTL.
    """
    def __init__(
        self,
        mongodb_client: MongoDBClient,
        ttl: int = 86400 * 7, # 7 days default TTL
        collection_name: str = "eat_llm_cache"
    ):
        """
        Initialize the LLM cache with MongoDB.

        Args:
            mongodb_client: Instance of MongoDBClient for database operations.
            ttl: Time-to-live for cache entries in seconds.
            collection_name: Name of the MongoDB collection for caching.
        """
        self.mongodb_client = mongodb_client
        self.ttl = ttl
        self.cache_collection_name = collection_name
        self.cache_collection = self.mongodb_client.get_collection(self.cache_collection_name)

        # Ensure TTL index is created (non-blocking)
        asyncio.create_task(self._ensure_indexes())

        logger.info(f"Initialized LLM cache with MongoDB collection '{self.cache_collection_name}' and TTL {self.ttl}s.")

    async def _ensure_indexes(self):
        """Ensure necessary indexes on the cache collection, especially TTL."""
        try:
            # TTL index on 'created_at' field
            await self.cache_collection.create_index(
                [("created_at", pymongo.ASCENDING)],
                expireAfterSeconds=self.ttl,
                name="ttl_created_at_index"
            )
            logger.info(f"Ensured TTL index on '{self.cache_collection_name}' for 'created_at' field.")
        except Exception as e:
            logger.error(f"Error creating TTL index for {self.cache_collection_name}: {e}", exc_info=True)

    def _generate_key(self, data: Any, model_id: str, cache_type: str) -> str:
        """Generate a unique key for the given input data, model, and type."""
        try:
            # Convert UserMessage to a consistent serializable format
            if isinstance(data, list) and all(isinstance(item, UserMessage) for item in data):
                serializable_data = []
                for msg in data:
                    content = msg.content.text if hasattr(msg.content, 'text') else str(msg.content)
                    serializable_data.append({"content": content, "role": msg.role})
            else:
                # For simple string data like embeddings
                serializable_data = data if isinstance(data, str) else json_serializable(data)
            
            # Create the key structure with proper serialization
            key_structure = {
                "data": serializable_data,
                "model_id": model_id,
                "cache_type": cache_type
            }
            
            # Handle the serialization carefully
            try:
                data_str = json.dumps(key_structure, sort_keys=True, default=json_serializable)
            except (TypeError, OverflowError) as e:
                logger.warning(f"Error serializing cache key data: {e}. Using string representation.")
                # Fallback to string representation
                if isinstance(serializable_data, str):
                    data_str = f"{serializable_data}::{model_id}::{cache_type}"
                else:
                    data_str = f"{str(serializable_data)}::{model_id}::{cache_type}"
            
            return hashlib.md5(data_str.encode()).hexdigest()
        except Exception as e:
            logger.error(f"Error generating cache key: {e}", exc_info=True)
            # Generate a fallback key with timestamp to avoid collisions
            fallback = f"{str(data)[:100]}::{model_id}::{cache_type}::{time.time()}"
            return hashlib.md5(fallback.encode()).hexdigest()

    async def get_completion(self, messages: List[UserMessage], model_id: str) -> Optional[str]:
        """Get a cached completion result from MongoDB."""
        try:
            key = self._generate_key(messages, model_id, "completion")
            # Using motor syntax for async find_one
            cached_doc = await self.cache_collection.find_one({"_id": key})
            if cached_doc:
                logger.info(f"Cache hit for completion (key: {key[:8]}...).")
                return cached_doc.get("cached_data") # 'cached_data' stores the response string
        except Exception as e:
            logger.warning(f"Error reading completion cache from MongoDB: {e}", exc_info=True)
        return None

    async def save_completion(self, messages: List[UserMessage], model_id: str, response: str) -> None:
        """Save a completion result to MongoDB cache."""
        try:
            key = self._generate_key(messages, model_id, "completion")
            cache_entry = {
                "_id": key, # Use the generated key as MongoDB's _id
                "model_id": model_id,
                "cache_type": "completion",
                "cached_data": response,
                "created_at": datetime.now(timezone.utc) # Store as BSON Date for TTL
            }
            
            # Using motor syntax for async replace_one (upsert)
            await self.cache_collection.replace_one({"_id": key}, cache_entry, upsert=True)
            logger.info(f"Cached completion (key: {key[:8]}...).")
        except Exception as e:
            logger.warning(f"Error writing completion cache to MongoDB: {e}", exc_info=True)

    async def get_embedding(self, text: str, model_id: str) -> Optional[List[float]]:
        """Get a cached embedding from MongoDB."""
        try:
            key = self._generate_key(text, model_id, "embedding")
            cached_doc = await self.cache_collection.find_one({"_id": key})
            if cached_doc:
                logger.info(f"Cache hit for embedding (key: {key[:8]}...).")
                cached_data = cached_doc.get("cached_data")
                # Ensure the cached data is a list of floats
                if isinstance(cached_data, list) and all(isinstance(item, (int, float)) for item in cached_data):
                    return cached_data
                else:
                    logger.warning(f"Invalid embedding data format in cache. Expected list of floats, got: {type(cached_data)}")
                    return None
        except Exception as e:
            logger.warning(f"Error reading embedding cache from MongoDB: {e}", exc_info=True)
        return None

    async def save_embedding(self, text: str, model_id: str, embedding: List[float]) -> None:
        """Save an embedding to MongoDB cache."""
        try:
            # Validate embedding is a list of floats
            if not isinstance(embedding, list) or not all(isinstance(item, (int, float)) for item in embedding):
                logger.warning(f"Invalid embedding format. Expected list of floats, got: {type(embedding)}")
                return
                
            key = self._generate_key(text, model_id, "embedding")
            cache_entry = {
                "_id": key,
                "model_id": model_id,
                "cache_type": "embedding",
                "cached_data": [float(x) for x in embedding],  # Ensure all values are float type
                "created_at": datetime.now(timezone.utc)
            }
            
            await self.cache_collection.replace_one({"_id": key}, cache_entry, upsert=True)
            logger.info(f"Cached embedding (key: {key[:8]}...).")
        except Exception as e:
            logger.warning(f"Error writing embedding cache to MongoDB: {e}", exc_info=True)

    async def get_batch_embeddings(self, texts: List[str], model_id: str) -> Optional[List[List[float]]]:
        """Get cached batch embeddings if all are available from MongoDB."""
        results = []
        all_found = True
        for text in texts:
            embedding = await self.get_embedding(text, model_id)
            if embedding is None:
                all_found = False
                break
            results.append(embedding)
        return results if all_found else None

    async def clear_cache(self, older_than_seconds: Optional[int] = None) -> int:
        """
        Clear the MongoDB cache.
        If older_than_seconds is specified, removes entries older than that.
        Otherwise, removes all entries.

        Args:
            older_than_seconds: Clear entries older than this many seconds. If None, clear all.

        Returns:
            Number of entries removed.
        """
        query_filter = {}
        if older_than_seconds is not None:
            cutoff_time = datetime.now(timezone.utc) - timedelta(seconds=older_than_seconds)
            query_filter = {"created_at": {"$lt": cutoff_time}}

        try:
            # Using motor syntax for async delete_many
            result = await self.cache_collection.delete_many(query_filter)
            deleted_count = result.deleted_count
            logger.info(f"Cleared {deleted_count} cache entries from MongoDB based on filter: {query_filter}.")
            return deleted_count
        except Exception as e:
            logger.error(f"Error clearing cache from MongoDB: {e}", exc_info=True)
            return 0


class OpenAIChatModel(LiteLLMChatModel):
    @property
    def provider_id(self) -> ProviderName:
        return "openai"
    
    def __init__(self, model_id: str | None = None, settings: dict | None = None) -> None:
        _settings = settings.copy() if settings is not None else {}
        super().__init__(
            model_id if model_id else os.getenv("OPENAI_CHAT_MODEL", "gpt-4o-mini"),
            provider_id="openai",
            settings=_settings,
        )


class EmbeddingModel:
    def __init__(self, model_id: str, provider_id: str, settings: Optional[Dict[str, Any]] = None):
        self.model_id = model_id
        self.provider_id = provider_id
        self.settings = settings or {}
    
    async def create(self, input_text: str) -> Dict[str, Any]:
        raise NotImplementedError("Subclasses must implement this method")
    
    async def create_batch(self, input_texts: List[str]) -> List[Dict[str, Any]]:
        raise NotImplementedError("Subclasses must implement this method")


class OpenAIEmbeddingModel(EmbeddingModel):
    def __init__(self, model_id: str | None = None, settings: dict | None = None):
        _settings = settings.copy() if settings else {}
        _model_id = model_id if model_id else os.getenv("OPENAI_EMBEDDING_MODEL", "text-embedding-3-small")
        super().__init__(model_id=_model_id, provider_id="openai", settings=_settings)
        import openai
        self.client = openai.OpenAI(
            api_key=_settings.get("api_key") or os.getenv("OPENAI_API_KEY"),
            base_url=_settings.get("base_url") or os.getenv("OPENAI_API_ENDPOINT"),
        )
    
    async def create(self, input_text: str) -> Dict[str, Any]:
        try:
            response = self.client.embeddings.create(model=self.model_id, input=input_text, encoding_format="float")
            # Safely extract model_dump - handle different versions of Pydantic
            usage_data = {}
            if hasattr(response.usage, "model_dump"):
                usage_data = response.usage.model_dump()
            elif hasattr(response.usage, "dict"):
                usage_data = response.usage.dict()
            else:
                usage_data = {"prompt_tokens": getattr(response.usage, "prompt_tokens", 0),
                             "total_tokens": getattr(response.usage, "total_tokens", 0)}
                
            return {
                "data": response.data[0].embedding, 
                "model": response.model, 
                "usage": usage_data
            }
        except Exception as e:
            logger.error(f"Error creating OpenAI embedding: {e}")
            # Return a placeholder embedding with the right dimension
            return {"data": [0.0] * 1536, "error": str(e)}
    
    async def create_batch(self, input_texts: List[str]) -> List[Dict[str, Any]]:
        try:
            response = self.client.embeddings.create(model=self.model_id, input=input_texts, encoding_format="float")
            # Safely extract model_dump - handle different versions of Pydantic
            usage_data = {}
            if hasattr(response.usage, "model_dump"):
                usage_data = response.usage.model_dump()
            elif hasattr(response.usage, "dict"):
                usage_data = response.usage.dict()
            else:
                usage_data = {"prompt_tokens": getattr(response.usage, "prompt_tokens", 0),
                             "total_tokens": getattr(response.usage, "total_tokens", 0)}
                
            return [{"data": item.embedding, "model": response.model, "usage": usage_data} 
                   for item in response.data]
        except Exception as e:
            logger.error(f"Error creating OpenAI batch embeddings: {e}")
            return [{"data": [0.0] * 1536, "error": str(e)} for _ in input_texts]


class OllamaEmbeddingModel(EmbeddingModel):
    def __init__(self, model_id: str | None = None, settings: dict | None = None):
        _settings = settings.copy() if settings else {}
        _model_id = model_id if model_id else "nomic-embed-text"
        super().__init__(model_id=_model_id, provider_id="ollama", settings=_settings)
        import httpx
        self.client = httpx.AsyncClient(base_url=_settings.get("base_url") or os.getenv("OLLAMA_BASE_URL", "http://localhost:11434"))
    
    async def create(self, input_text: str) -> Dict[str, Any]:
        try:
            response = await self.client.post("/api/embeddings", json={"model": self.model_id, "prompt": input_text})
            response.raise_for_status()
            data = response.json()
            return {
                "data": data["embedding"], 
                "model": self.model_id, 
                "usage": {"total_tokens": data.get("total_tokens", 0)}
            }
        except Exception as e:
            logger.error(f"Error creating Ollama embedding: {e}")
            return {"data": [0.0] * 768, "error": str(e)}
    
    async def create_batch(self, input_texts: List[str]) -> List[Dict[str, Any]]:
        results = []
        for text in input_texts:
            results.append(await self.create(text))
        return results


class LLMService:
    """
    LLM service that interfaces with chat and embedding models.
    Now uses MongoDB-backed LLMCache if caching is enabled.
    """
    def __init__(
        self,
        provider: str = "openai",
        api_key: Optional[str] = None,
        model: Optional[str] = None,
        embedding_model: Optional[str] = None,
        use_cache: bool = True,
        mongodb_client: Optional[MongoDBClient] = None,
        container: Optional[Any] = None
    ):
        self.provider = provider
        self.api_key = api_key or os.environ.get("OPENAI_API_KEY")
        self.container = container
        self.chat_model = None
        self.embedding_model = None
        self.cache = None
        self.mongodb_client = None

        # Resolve MongoDBClient
        if mongodb_client:
            self.mongodb_client = mongodb_client
        elif self.container and hasattr(self.container, 'has') and self.container.has('mongodb_client'):
            self.mongodb_client = self.container.get('mongodb_client')
        else:
            # Attempt to create a default MongoDBClient
            try:
                self.mongodb_client = MongoDBClient()
                if self.container and hasattr(self.container, 'register'):
                    self.container.register('mongodb_client', self.mongodb_client)
            except ValueError as e:
                logger.warning(f"MongoDBClient could not be initialized for LLMService cache: {e}. Cache will be disabled.")
                self.mongodb_client = None
                use_cache = False

        # Initialize cache if enabled AND mongodb_client is available
        self.use_cache = use_cache and self.mongodb_client is not None
        if self.use_cache and self.mongodb_client:
            self.cache = LLMCache(mongodb_client=self.mongodb_client)
        else:
            self.cache = None
            if use_cache and not self.mongodb_client:
                logger.warning("LLM Cache was requested but is disabled due to missing MongoDB client.")

        # Save API key to environment if provided (for OpenAI)
        if self.api_key and provider == "openai":
            os.environ["OPENAI_API_KEY"] = self.api_key

        # Initialize models based on provider
        try:
            if provider == "openai":
                self.chat_model = OpenAIChatModel(
                    model_id=model,
                    settings={"api_key": self.api_key} if self.api_key else None
                )
                self.embedding_model = OpenAIEmbeddingModel(
                    model_id=embedding_model,
                    settings={"api_key": self.api_key} if self.api_key else None
                )
            elif provider == "ollama":
                self.chat_model = LiteLLMChatModel(
                    model_id=model or "llama3",
                    provider_id="ollama",
                    settings={}
                )
                self.embedding_model = OllamaEmbeddingModel(
                    model_id=embedding_model,
                    settings={}
                )
            else:
                logger.warning(f"Unknown LLM provider '{provider}'. Defaulting to OpenAI.")
                self.provider = "openai"
                self.chat_model = OpenAIChatModel(
                    model_id=model,
                    settings={"api_key": self.api_key} if self.api_key else None
                )
                self.embedding_model = OpenAIEmbeddingModel(
                    model_id=embedding_model,
                    settings={"api_key": self.api_key} if self.api_key else None
                )
        except Exception as e:
            logger.error(f"Error initializing LLM models: {e}", exc_info=True)
            # We'll continue with None models and handle errors at usage time
            
        cache_status = "enabled (MongoDB)" if self.cache else "disabled"
        model_id = self.chat_model.model_id if self.chat_model else "N/A"
        embedding_id = self.embedding_model.model_id if self.embedding_model else "N/A"
        
        logger.info(f"Initialized LLM service with provider: {self.provider}, "
                    f"chat model: {model_id}, "
                    f"embedding model: {embedding_id}, "
                    f"cache: {cache_status}")

    async def generate(self, prompt: str) -> str:
        """Generate text based on a prompt."""
        if not self.chat_model:
            logger.error("Chat model not initialized. Cannot generate text.")
            return "Error: Chat model not available."

        logger.debug(f"Generating response for prompt: {prompt[:50]}...")
        messages = [UserMessage(prompt)]

        if self.cache:
            try:
                cached_response = await self.cache.get_completion(messages, self.chat_model.model_id)
                if cached_response is not None:
                    return cached_response
            except Exception as e:
                logger.warning(f"Error retrieving from cache: {e}. Proceeding without cache.")
        
        try:
            response = await self.chat_model.create(messages=messages)
            response_text = response.get_text_content()
            
            if self.cache:
                try:
                    await self.cache.save_completion(messages, self.chat_model.model_id, response_text)
                except Exception as e:
                    logger.warning(f"Error saving to cache: {e}")
                    
            return response_text
        except Exception as e:
            logger.error(f"Error generating response: {e}", exc_info=True)
            return f"Error generating response: {str(e)}"

    async def embed(self, text: str) -> List[float]:
        """Generate embeddings for text."""
        if not self.embedding_model:
            logger.error("Embedding model not initialized. Cannot generate embeddings.")
            # Return a list of floats of a default dimension
            return [0.0] * 1536

        logger.debug(f"Generating embedding for text: {text[:50]}...")
        
        if self.cache:
            try:
                cached_embedding = await self.cache.get_embedding(text, self.embedding_model.model_id)
                if cached_embedding is not None:
                    return cached_embedding
            except Exception as e:
                logger.warning(f"Error retrieving embedding from cache: {e}. Proceeding without cache.")
        
        try:
            response = await self.embedding_model.create(text)
            embedding = response.get("data", [])
            
            if not embedding and "error" not in response:
                logger.warning(f"Embedding model returned no data for text: {text[:50]}")
                
            # Ensure embedding is a list of floats
            if embedding and all(isinstance(x, (int, float)) for x in embedding):
                # Normalize to float for consistent storage
                embedding = [float(x) for x in embedding]
                
                if self.cache:
                    try:
                        await self.cache.save_embedding(text, self.embedding_model.model_id, embedding)
                    except Exception as e:
                        logger.warning(f"Error saving embedding to cache: {e}")
                
                return embedding
            else:
                logger.warning(f"Invalid embedding format returned: {type(embedding)}")
                dim = 1536 if self.provider == "openai" else 768
                return [0.0] * dim
                
        except Exception as e:
            logger.error(f"Error generating embedding: {e}", exc_info=True)
            # Fallback vector dimension
            dim = 1536 if self.provider == "openai" else 768
            return [0.0] * dim

    async def embed_batch(self, texts: List[str]) -> List[List[float]]:
        """Generate embeddings for multiple texts."""
        if not self.embedding_model:
            logger.error("Embedding model not initialized. Cannot generate batch embeddings.")
            dim = 1536 if self.provider == "openai" else 768
            return [[0.0] * dim for _ in texts]

        logger.debug(f"Generating batch embeddings for {len(texts)} texts...")
        
        if self.cache:
            try:
                cached_embeddings = await self.cache.get_batch_embeddings(texts, self.embedding_model.model_id)
                if cached_embeddings is not None:
                    return cached_embeddings
            except Exception as e:
                logger.warning(f"Error retrieving batch embeddings from cache: {e}. Proceeding without cache.")
        
        try:
            responses = await self.embedding_model.create_batch(texts)
            embeddings = []
            
            for i, response in enumerate(responses):
                current_embedding = response.get("data", [])
                
                if not current_embedding and "error" not in response:
                    logger.warning(f"Embedding model returned no data for batch text item {i}: {texts[i][:50]}")
                    dim = 1536 if self.provider == "openai" else 768
                    current_embedding = [0.0] * dim
                
                # Ensure embedding is a list of floats
                if current_embedding and all(isinstance(x, (int, float)) for x in current_embedding):
                    # Normalize to float for consistent storage
                    current_embedding = [float(x) for x in current_embedding]
                    
                    if self.cache:
                        try:
                            await self.cache.save_embedding(texts[i], self.embedding_model.model_id, current_embedding)
                        except Exception as e:
                            logger.warning(f"Error saving batch embedding to cache: {e}")
                else:
                    logger.warning(f"Invalid embedding format for batch item {i}")
                    dim = 1536 if self.provider == "openai" else 768
                    current_embedding = [0.0] * dim
                
                embeddings.append(current_embedding)
                
            return embeddings
            
        except Exception as e:
            logger.error(f"Error generating batch embeddings: {e}", exc_info=True)
            dim = 1536 if self.provider == "openai" else 768
            return [[0.0] * dim for _ in range(len(texts))]

    async def clear_cache(self, older_than_seconds: Optional[int] = None) -> int:
        """Clear the LLM cache. Delegates to LLMCache instance."""
        if not self.cache:
            logger.info("Cache is disabled, nothing to clear.")
            return 0
        try:
            return await self.cache.clear_cache(older_than_seconds)
        except Exception as e:
            logger.error(f"Error clearing cache: {e}", exc_info=True)
            return 0

    async def generate_applicability_text(self, text_chunk: str, component_type: str = "", component_name: str = "") -> str:
        """Generate applicability text (T_raz)."""
        prompt = f"""
        Analyze the following text chunk from a component ({component_type or "unknown"}: {component_name or "unknown"}):
        '''
        {text_chunk}
        '''
        Generate a concise description (T_raz) focusing ONLY on its potential applicability and relevance for different tasks. Describe:
        1. **Relevant Tasks:** What specific developer/agent tasks might this chunk be useful for (e.g., 'code generation', 'API documentation', 'testing', 'requirements analysis', 'debugging', 'cost estimation', 'security review')?
        2. **Key Concepts/Implications:** What are the non-obvious technical or functional implications derived from this text? (e.g., 'dependency on X', 'requires async handling', 'critical for user authentication flow').
        3. **Target Audience/Context:** Who would find this most useful or in what situation? (e.g., 'backend developer implementing feature Y', 'project manager estimating effort', 'security auditor reviewing access control').

        Be concise and focus on applicability *beyond* just restating the content. Output ONLY the generated description (T_raz).
        """
        try:
            applicability_text = await self.generate(prompt)
            return applicability_text.strip()
        except Exception as e:
            logger.error(f"Error generating applicability text: {e}", exc_info=True)
            return f"Error generating applicability: {str(e)}"