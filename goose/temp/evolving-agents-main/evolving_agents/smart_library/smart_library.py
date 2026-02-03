# evolving_agents/smart_library/smart_library.py

import os
import json # Still used for logging/debug, not primary storage
import logging
import uuid
import re
import asyncio
from datetime import datetime, timezone
from typing import List, Dict, Any, Optional, Tuple

import numpy as np
import pymongo # For index creation constants
import motor.motor_asyncio # Import Motor for async MongoDB operations

from evolving_agents.core.llm_service import LLMService
from evolving_agents.core.mongodb_client import MongoDBClient # Assuming this is created

# Import DependencyContainer using TYPE_CHECKING to avoid circular import
from typing import TYPE_CHECKING
if TYPE_CHECKING:
    from evolving_agents.core.dependency_container import DependencyContainer

logger = logging.getLogger(__name__)

DEFAULT_EMBEDDING_DIMENSION = 1536


class SmartLibrary:
    """
    Unified library that stores all agents, tools, and firmware as dictionary records
    in MongoDB, with MongoDB Atlas Vector Search for semantic search using dual embeddings.
    """
    def __init__(
        self,
        llm_service: Optional[LLMService] = None,
        container: Optional["DependencyContainer"] = None,
        mongodb_uri: Optional[str] = None,
        mongodb_db_name: Optional[str] = None,
        components_collection_name: str = "eat_components",
        mongodb_client: Optional[MongoDBClient] = None  # Added parameter
    ):
        self.container = container
        self._initialized = False
        self.mongodb_client: Optional[MongoDBClient] = None
        self.components_collection: Optional[motor.motor_asyncio.AsyncIOMotorCollection] = None
        self.components_collection_name = components_collection_name

        # --- Atlas Vector Search Index Names (Assumed to exist in Atlas) ---
        self.atlas_vs_idx_content_embedding = "idx_components_content_embedding"
        self.atlas_vs_idx_applicability_embedding = "idx_components_applicability_embedding" # Corrected assumption

        if container and container.has('llm_service'):
            self.llm_service = llm_service or container.get('llm_service')
        elif llm_service:
            self.llm_service = llm_service
        else:
            self.llm_service = LLMService(container=container)
            if container: container.register('llm_service', self.llm_service)

        if container and container.has('mongodb_client'):
            self.mongodb_client = container.get('mongodb_client')
        elif mongodb_client:  # Check if an instance was passed via the new parameter
            self.mongodb_client = mongodb_client
        elif isinstance(llm_service, MongoDBClient): # Defensive check (was and mongodb_client is None)
            logger.warning("MongoDBClient instance might have been passed as llm_service to SmartLibrary. Correcting.")
            self.mongodb_client = llm_service
            self.llm_service = LLMService(container=container) # Reset llm_service if it was misused
            if container: container.register('llm_service', self.llm_service)
        elif isinstance(mongodb_uri, MongoDBClient): # Defensive check (was and mongodb_client is None)
            logger.warning("MongoDBClient instance might have been passed as mongodb_uri to SmartLibrary. Correcting.")
            self.mongodb_client = mongodb_uri # mongodb_uri was misused for the client instance
        else:  # Default creation using URI and db_name
            try:
                self.mongodb_client = MongoDBClient(uri=mongodb_uri, db_name=mongodb_db_name)
            except ValueError as e:
                logger.error(f"SmartLibrary: Failed to initialize default MongoDBClient: {e}. Operations requiring DB will fail.")
                self.mongodb_client = None
        
        if self.mongodb_client and container and not container.has('mongodb_client'):
            container.register('mongodb_client', self.mongodb_client)
        
        if self.mongodb_client:
            if not isinstance(self.mongodb_client.client, motor.motor_asyncio.AsyncIOMotorClient):
                logger.critical("MongoDBClient is NOT using an AsyncIOMotorClient (Motor). SmartLibrary WILL FAIL with async database operations.")
                self.components_collection = None
            else:
                self.components_collection = self.mongodb_client.get_collection(self.components_collection_name)
                asyncio.create_task(self._ensure_indexes())
        else:
            logger.error("SmartLibrary: MongoDBClient is not available. Database operations will not be possible.")
            self.components_collection = None

        logger.info(f"SmartLibrary initialized. MongoDB collection: '{self.components_collection_name if self.components_collection is not None else 'UNAVAILABLE'}'")
        
        if container and not container.has('smart_library'):
            container.register('smart_library', self)

    async def _ensure_indexes(self):
        if self.components_collection is None:
            logger.error(f"Cannot ensure indexes: components_collection '{self.components_collection_name}' is None.")
            return
        try:
            # Standard indexes
            await self.components_collection.create_index([("id", pymongo.ASCENDING)], unique=True, background=True)
            await self.components_collection.create_index([("name", pymongo.ASCENDING)], background=True)
            await self.components_collection.create_index([("record_type", pymongo.ASCENDING)], background=True)
            await self.components_collection.create_index([("domain", pymongo.ASCENDING)], background=True)
            await self.components_collection.create_index([("tags", pymongo.ASCENDING)], background=True)
            await self.components_collection.create_index([("status", pymongo.ASCENDING)], background=True)
            await self.components_collection.create_index([("last_updated", pymongo.DESCENDING)], background=True)
            
            # Text index for fallback search
            try:
                await self.components_collection.create_index(
                    [
                        ("name", pymongo.TEXT), 
                        ("description", pymongo.TEXT),
                        ("tags", pymongo.TEXT),
                        ("domain", pymongo.TEXT) 
                    ], 
                    name="component_text_search_fallback_idx", 
                    default_language="english",
                    weights={"name": 5, "tags": 3, "description": 2, "domain": 1},
                    background=True
                )
                logger.info("Created/ensured text index for fallback search on components.")
            except pymongo.errors.OperationFailure as op_fail:
                if op_fail.code == 85: # IndexOptionsConflict
                     logger.info(f"Text index 'component_text_search_fallback_idx' or equivalent already exists. Details: {op_fail.details}")
                elif op_fail.code == 86: # IndexKeySpecsConflict
                     logger.warning(f"Text index with different specs already exists. Details: {op_fail.details}")
                else:
                    logger.warning(f"Could not create/ensure text index for fallback search: {op_fail}")
            except Exception as text_idx_err:
                logger.warning(f"Could not create/ensure text index for fallback search (generic error): {text_idx_err}")
                
            logger.info(f"Ensured standard indexes on '{self.components_collection_name}' collection.")
            logger.info(f"Reminder: Atlas Vector Search Indexes ({self.atlas_vs_idx_content_embedding}, {self.atlas_vs_idx_applicability_embedding}) must be configured manually in MongoDB Atlas.")
        except Exception as e:
            logger.error(f"Error creating MongoDB indexes for {self.components_collection_name}: {e}", exc_info=True)

    async def initialize(self):
        if self._initialized: return
        if self.container and self.container.has('agent_bus'):
            agent_bus = self.container.get('agent_bus')
            if hasattr(agent_bus, 'initialize_from_library'):
                await agent_bus.initialize_from_library()
        self._initialized = True
        logger.info("SmartLibrary fully initialized and integrated.")

    def _get_record_vector_text(self, record: Dict[str, Any]) -> str:
        name = record.get("name", "")
        description = record.get("description", "")
        record_type = record.get("record_type", "")
        domain = record.get("domain", "")
        tags = " ".join(record.get("tags", []))
        code_snippet = record.get("code_snippet", "")
        usage_count = record.get("usage_count", 0)
        success_count = record.get("success_count", 0)
        usage_stats = f"Used {usage_count} times with {((success_count / usage_count) if usage_count > 0 else 0):.0%} success." if usage_count > 0 else ""
        
        interface_info = ""
        if record_type == "TOOL":
            signatures = re.findall(r'def\s+(\w+\([^)]*\))', code_snippet)
            if signatures: interface_info += "Functions: " + ", ".join(signatures) + ". "
            api_endpoints = re.findall(r'@\w+\.(?:route|get|post|put|delete)\([\'"]([^\'"]+)[\'"]', code_snippet)
            if api_endpoints: interface_info += "API endpoints: " + ", ".join(api_endpoints) + ". "
            class_defs = re.findall(r'class\s+(\w+)(?:\([^)]*\))?:', code_snippet)
            if class_defs: interface_info += "Classes: " + ", ".join(class_defs) + ". "
        
        return (
            f"Component Name: {name}. Type: {record_type}. Domain: {domain}. "
            f"Functional Purpose: {description}. {interface_info}"
            f"{('Relevant Tags: ' + tags + '. ') if tags else ''}"
            f"{('Usage Statistics: ' + usage_stats + '. ') if usage_stats else ''}"
            f"Core Implementation Snippet (summary):\n```\n{code_snippet[:1000]}\n```"
        )

    async def generate_applicability_text(self, record_data: Dict[str, Any]) -> str:
        name = record_data.get("name", "Unknown Component")
        description = record_data.get("description", "No description")
        record_type = record_data.get("record_type", "UNKNOWN")
        domain = record_data.get("domain", "general")
        code_snippet = record_data.get("code_snippet", "")
        code_summary = code_snippet[:500] + ("..." if len(code_snippet) > 500 else "")
        
        prompt_template = """
        Analyze the following component specification:
        Name: {name}
        Type: {record_type}
        Domain: {domain}
        Description: {description}
        Code Summary:
        ```
        {code_summary}
        ```
        Generate a concise applicability text (T_raz). This text should clearly and succinctly describe:
        1. **Primary Use Cases:** What specific problems, tasks, or scenarios is this component designed to address or solve effectively?
        2. **Target User/System:** Who or what (e.g., backend developer, data pipeline, another AI agent, end-user via an application) would typically use or benefit from this component?
        3. **Key Benefits/Strengths:** What are the 1-3 most significant advantages or standout features of this component for its intended purpose?
        4. **Contextual Relevance:** In what types of projects, development phases, or operational situations would this component be most valuable or appropriate?
        5. **Integration Points/Synergies:** How might this component typically interact with, depend on, or complement other types of tools, agents, or systems?
        Focus on the functional applicability and intended operational context. The output should be a single, coherent paragraph. Output ONLY the generated T_raz description.
        """
        prompt = prompt_template.format(
            name=name, record_type=record_type, domain=domain,
            description=description, code_summary=code_summary
        )
        applicability_text = await self.llm_service.generate(prompt)
        logger.debug(f"Generated T_raz for {name}: {applicability_text[:100]}...")
        return applicability_text.strip()

    async def save_record(self, record: Dict[str, Any]) -> str:
        if self.components_collection is None:
            logger.error("Cannot save record: components_collection is None. MongoDB connection might have failed.")
            raise ConnectionError("SmartLibrary cannot connect to MongoDB to save record.")
        record_id = record.get("id")
        if not record_id: raise ValueError("Record must have an 'id' to be saved.")
        for key in ["content_embedding", "applicability_embedding"]:
            if key in record and record[key] is not None:
                record[key] = [float(x) for x in record[key]]
        try:
            # Ensure datetime fields are BSON compatible datetimes
            for dt_field in ["created_at", "last_updated"]:
                if dt_field in record and isinstance(record[dt_field], str):
                    try:
                        record[dt_field] = datetime.fromisoformat(record[dt_field].replace("Z", "+00:00"))
                    except ValueError: # if already a datetime object, or different format
                        if not isinstance(record[dt_field], datetime):
                            logger.warning(f"Could not parse {dt_field} string '{record[dt_field]}' to datetime for record {record_id}. Leaving as is or consider UTC default.")
                            record[dt_field] = datetime.now(timezone.utc) # Fallback
                elif dt_field in record and not isinstance(record[dt_field], datetime):
                     record[dt_field] = datetime.now(timezone.utc) # Fallback if not str or datetime

            result = await self.components_collection.replace_one({"id": record_id}, record, upsert=True)
            log_action = "Inserted new" if result.upserted_id else "Updated" if result.modified_count > 0 else "Refreshed (no change to)"
            logger.info(f"{log_action} record {record_id} ({record.get('name')}) in MongoDB.")
            return record_id
        except Exception as e:
            logger.error(f"Error saving record {record_id} to MongoDB: {e}", exc_info=True); raise

    async def find_record_by_id(self, record_id: str) -> Optional[Dict[str, Any]]:
        if self.components_collection is None: return None
        return await self.components_collection.find_one({"id": record_id}, {"_id": 0})

    async def find_record_by_name(self, name: str, record_type: Optional[str] = None) -> Optional[Dict[str, Any]]:
        if self.components_collection is None: return None
        query = {"name": name}
        if record_type: query["record_type"] = record_type
        return await self.components_collection.find_one(query, {"_id": 0})

    async def find_records_by_domain(self, domain: str, record_type: Optional[str] = None) -> List[Dict[str, Any]]:
        if self.components_collection is None: return []
        query = {"domain": domain}
        if record_type: query["record_type"] = record_type
        cursor = self.components_collection.find(query, {"_id": 0})
        return await cursor.to_list(length=None)

    async def compute_similarity(self, embedding1: List[float], embedding2: List[float]) -> float:
        if not embedding1 or not embedding2: return 0.0
        vec1, vec2 = np.array(embedding1, dtype=np.float32), np.array(embedding2, dtype=np.float32)
        norm1, norm2 = np.linalg.norm(vec1), np.linalg.norm(vec2)
        if norm1 == 0 or norm2 == 0: return 0.0
        similarity = np.dot(vec1, vec2) / (norm1 * norm2)
        return float(np.clip(similarity, -1.0, 1.0))

    async def semantic_search(
        self, query: str, task_context: Optional[str] = None,
        record_type: Optional[str] = None, domain: Optional[str] = None,
        limit: int = 5, threshold: float = 0.0, task_weight: Optional[float] = 0.7
    ) -> List[Tuple[Dict[str, Any], float, float, float]]:
        if self.components_collection is None:
            logger.error("Semantic search unavailable: components_collection is None.")
            return []
        if query is None: raise ValueError("Query string cannot be None")

        effective_task_weight = task_weight if task_context and task_weight is not None else (0.7 if task_context else 0.0)
        query_embedding_orig = await self.llm_service.embed(query)
        
        task_embedding_for_fallback: Optional[List[float]] = None
        if task_context:
            task_embedding_for_fallback = await self.llm_service.embed(task_context)

        vector_search_enabled = os.environ.get("VECTOR_SEARCH_ENABLED", "true").lower() != "false"
        if not vector_search_enabled:
            logger.info("Vector search is disabled by configuration. Using fallback method directly.")
            return await self._perform_fallback_search(
                query, query_embedding_orig, task_embedding_for_fallback,
                record_type, domain, limit, effective_task_weight, task_context
            )

        current_atlas_search_index_name: str
        vector_path_for_search: str
        query_vector_for_search: List[float]

        if task_context and task_embedding_for_fallback:
            current_atlas_search_index_name = self.atlas_vs_idx_applicability_embedding
            vector_path_for_search = "applicability_embedding"
            query_vector_for_search = task_embedding_for_fallback
        else:
            current_atlas_search_index_name = self.atlas_vs_idx_content_embedding
            vector_path_for_search = "content_embedding"
            query_vector_for_search = query_embedding_orig
        
        vs_stage_definition: Dict[str, Any] = {
            "index": current_atlas_search_index_name,
            "path": vector_path_for_search,
            "queryVector": query_vector_for_search,
            "numCandidates": limit * 20, 
            "limit": limit * 10 
        }
        
        mql_filter_conditions = []
        if record_type: mql_filter_conditions.append({"record_type": record_type})
        if domain: mql_filter_conditions.append({"domain": domain})
        mql_filter_conditions.append({"status": "active"})

        if mql_filter_conditions:
            vs_stage_definition["filter"] = {"$and": mql_filter_conditions} if len(mql_filter_conditions) > 1 else mql_filter_conditions[0]
        
        # Define fields to project, ensuring embeddings are included for re-ranking if needed,
        # but can be excluded from the final result passed to the caller.
        projected_doc_fields = {
            "_id": 0, "id": 1, "name": 1, "record_type": 1, "domain": 1, "description": 1,
            "version": 1, "usage_count": 1, "success_count": 1, "tags": 1, "metadata": 1,
            "content_embedding": 1, "applicability_embedding": 1 # Needed for re-ranking
        }
            
        search_pipeline: List[Dict[str, Any]] = [
            {"$vectorSearch": vs_stage_definition},
            {"$project": {**projected_doc_fields, "similarity_score": {"$meta": "vectorSearchScore"}}},
            # $vectorSearch already sorts by score and applies its limit.
            # Additional sort/limit here applies to the results from $vectorSearch.
            # {"$sort": {"similarity_score": -1}}, # Usually redundant
            # {"$limit": limit * 3 } # Redundant if $vectorSearch limit is well-tuned
        ]
        
        logger.debug(f"MongoDB $vectorSearch Pipeline for semantic search (SmartLibrary): {json.dumps(search_pipeline, indent=2)}")
        
        candidate_docs = []
        try:
            candidate_docs_cursor = self.components_collection.aggregate(search_pipeline)
            # The limit in $vectorSearch stage should fetch enough candidates.
            # The to_list length here should match or be slightly more than the final desired limit.
            candidate_docs = await candidate_docs_cursor.to_list(length=limit * 3) 
            
            if not candidate_docs: 
                logger.warning(f"Vector search with index '{current_atlas_search_index_name}' returned no results. Falling back.")
                return await self._perform_fallback_search(
                    query, query_embedding_orig, task_embedding_for_fallback,
                    record_type, domain, limit, effective_task_weight, task_context
                )
                
        except pymongo.errors.OperationFailure as e: 
            logger.error(f"MongoDB $vectorSearch failed: {e}", exc_info=False) 
            if "index not found" in str(e).lower() or "unknown search index" in str(e).lower() or "Invalid $vectorSearch" in str(e).lower():
                 logger.error(f"CRITICAL: Atlas Vector Search index '{current_atlas_search_index_name}' likely missing or misconfigured. Details: {str(e)}. Query: {vs_stage_definition}")
            
            logger.info("Attempting fallback search due to vector search error...")
            return await self._perform_fallback_search(
                query, query_embedding_orig, task_embedding_for_fallback, 
                record_type, domain, limit, effective_task_weight, task_context
            )
        except Exception as e: 
            logger.error(f"Unexpected error during semantic search: {e}", exc_info=True)
            logger.info("Attempting fallback search due to unexpected error...")
            return await self._perform_fallback_search(
                query, query_embedding_orig, task_embedding_for_fallback, 
                record_type, domain, limit, effective_task_weight, task_context
            )
        
        search_results_tuples = []
        for doc in candidate_docs:
            # The primary score from $vectorSearch is in 'similarity_score'
            raw_vector_score = doc.get("similarity_score", 0.0) 
            content_embedding_from_doc = doc.get("content_embedding", [])
            applicability_embedding_from_doc = doc.get("applicability_embedding", [])

            content_score = 0.0
            task_score = 0.0

            if task_context and task_embedding_for_fallback: 
                # If task_context was used, raw_vector_score is based on applicability_embedding
                task_score = raw_vector_score 
                # Recalculate content_score based on original query and content_embedding
                if query_embedding_orig and content_embedding_from_doc:
                    content_score = await self.compute_similarity(query_embedding_orig, content_embedding_from_doc)
                else: # If content_embedding is missing or query_embedding_orig is missing
                    content_score = 0.0 if content_embedding_from_doc else 0.5 # Penalize if no content embedding
            else: 
                # If no task_context, raw_vector_score is based on content_embedding
                content_score = raw_vector_score
                # Recalculate task_score based on query_embedding_orig and applicability_embedding
                if query_embedding_orig and applicability_embedding_from_doc: 
                    task_score = await self.compute_similarity(query_embedding_orig, applicability_embedding_from_doc)
                else: # If applicability_embedding is missing
                    task_score = 0.0 if applicability_embedding_from_doc else 0.5 # Penalize if no applicability embedding

            final_score = (effective_task_weight * task_score) + ((1.0 - effective_task_weight) * content_score)
            
            usage_count = doc.get("usage_count", 0)
            success_rate = doc.get("success_count", 0) / max(usage_count, 1) if usage_count > 0 else 0.0
            boost = min(0.05, (usage_count / 200.0) * success_rate) # Small boost for usage/success
            adjusted_score = min(1.0, final_score + boost)

            if adjusted_score >= threshold:
                doc_copy = doc.copy()
                # Remove embeddings from the copy returned to the user to save space
                doc_copy.pop("content_embedding", None)
                doc_copy.pop("applicability_embedding", None)
                doc_copy.pop("similarity_score", None) # Remove the raw $vectorSearch score
                search_results_tuples.append((doc_copy, adjusted_score, content_score, task_score))
        
        search_results_tuples.sort(key=lambda x: x[1], reverse=True)
        return search_results_tuples[:limit]

    async def _try_text_search(self, query_text: str, record_type: Optional[str] = None, domain: Optional[str] = None, limit: int = 5) -> List[Dict[str, Any]]:
        if self.components_collection is None: return []
            
        text_search_filter: Dict[str, Any] = {"$text": {"$search": query_text}, "status": "active"}
        if record_type: text_search_filter["record_type"] = record_type
        if domain: text_search_filter["domain"] = domain
            
        try:
            projection_with_score = {
                "_id": 0, "id": 1, "name": 1, "record_type": 1, "domain": 1, 
                "description": 1, "version": 1, "tags": 1, "metadata": 1,
                "usage_count": 1, "success_count": 1, 
                "content_embedding": 1, "applicability_embedding": 1, # Keep for scoring
                "text_search_score": {"$meta": "textScore"} 
            }
            cursor = self.components_collection.find(
                text_search_filter,
                projection_with_score
            ).sort([("text_search_score", {"$meta": "textScore"})]).limit(limit)
            
            results = await cursor.to_list(length=None)
            logger.info(f"Fallback text search found {len(results)} results for query '{query_text}'.")
            return results
        except pymongo.errors.OperationFailure as op_fail:
            if "text index required" in str(op_fail).lower() or "No text index" in str(op_fail).lower() or "text index not found" in str(op_fail).lower():
                logger.warning(f"Text search failed because no suitable text index exists on '{self.components_collection_name}'. Fallback will be less effective.")
            else:
                logger.error(f"Text search failed with OperationFailure: {op_fail}")
            return []
        except Exception as e:
            logger.error(f"Generic error during text search: {e}", exc_info=True)
            return []

    async def _perform_fallback_search(
        self, query: str, query_embedding_orig: List[float], 
        task_embedding: Optional[List[float]], 
        record_type: Optional[str] = None, domain: Optional[str] = None, 
        limit: int = 5, effective_task_weight: float = 0.7,
        task_context: Optional[str] = None # Added task_context parameter
    ) -> List[Tuple[Dict[str, Any], float, float, float]]:
        if self.components_collection is None: return []

        docs_to_score = await self._try_text_search(query, record_type, domain, limit * 3) 

        if not docs_to_score:
            logger.info("Text search yielded no results or failed, trying broader regex/tag match for fallback.")
            fallback_query_filter: Dict[str, Any] = {"status": "active"}
            if record_type: fallback_query_filter["record_type"] = record_type
            if domain: fallback_query_filter["domain"] = domain
            
            query_terms = [term.lower() for term in re.split(r'\s+', query) if len(term) > 2]
            if query_terms:
                regex_conditions = []
                for term in query_terms:
                    safe_term = re.escape(term)
                    regex_conditions.append({"name": {"$regex": safe_term, "$options": "i"}})
                    regex_conditions.append({"description": {"$regex": safe_term, "$options": "i"}})
                if regex_conditions:
                    fallback_query_filter["$or"] = regex_conditions + [{"tags": {"$in": query_terms}}]
            
            projection = { # Ensure embeddings are projected for scoring
                "_id": 0, "id": 1, "name": 1, "record_type": 1, "domain": 1, 
                "description": 1, "version": 1, "tags": 1, "metadata": 1,
                "usage_count": 1, "success_count": 1, 
                "content_embedding": 1, "applicability_embedding": 1
            }
            try:
                cursor = self.components_collection.find(fallback_query_filter, projection).limit(limit * 5)
                docs_to_score = await cursor.to_list(length=None)
                logger.info(f"Fallback regex/tag search found {len(docs_to_score)} documents.")
            except Exception as e:
                logger.error(f"Fallback regex/tag search query failed: {e}", exc_info=True)
                return []
        
        if not docs_to_score:
            logger.info("No documents found even with broader fallback search.")
            return []

        result_tuples = []
        for doc in docs_to_score:
            content_embedding = doc.get("content_embedding", [])
            applicability_embedding = doc.get("applicability_embedding", [])
            
            content_score = 0.5 # Default if no embedding
            if content_embedding and query_embedding_orig:
                content_score = await self.compute_similarity(query_embedding_orig, content_embedding)
            
            task_score = 0.5  # Default if no embedding
            if applicability_embedding:
                embedding_for_task_calc = task_embedding if task_context and task_embedding else query_embedding_orig
                if embedding_for_task_calc:
                    task_score = await self.compute_similarity(embedding_for_task_calc, applicability_embedding)
            
            text_search_score = doc.get("text_search_score", 0.0) # From _try_text_search
            combined_semantic_score = (effective_task_weight * task_score) + ((1.0 - effective_task_weight) * content_score)
            
            final_score = combined_semantic_score
            if text_search_score > 0.5: # If text search provided a good score, blend it
                final_score = (final_score * 0.7) + (text_search_score * 0.3) 

            usage_count = doc.get("usage_count", 0)
            success_rate = doc.get("success_count", 0) / max(1, usage_count) if usage_count > 0 else 0.0
            boost = min(0.05, (usage_count / 200.0) * success_rate)
            adjusted_score = min(1.0, final_score + boost)
            
            doc_copy = doc.copy()
            doc_copy.pop("content_embedding", None)
            doc_copy.pop("applicability_embedding", None)
            doc_copy.pop("text_search_score", None) 
            
            result_tuples.append((doc_copy, adjusted_score, content_score, task_score))
                
        result_tuples.sort(key=lambda x: x[1], reverse=True)
        return result_tuples[:limit]

    async def create_record(
        self, name: str, record_type: str, domain: str, description: str,
        code_snippet: str, version: str = "1.0.0", status: str = "active",
        tags: Optional[List[str]] = None, metadata: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        if self.components_collection is None:
            logger.error("Cannot create record: components_collection is None. MongoDB connection might have failed.")
            raise ConnectionError("SmartLibrary cannot connect to MongoDB to create record.")

        record_id = str(uuid.uuid4())
        current_time_dt = datetime.now(timezone.utc) 
        
        traz_input = {"name": name, "record_type": record_type, "domain": domain, "description": description, "code_snippet": code_snippet}
        applicability_text = await self.generate_applicability_text(traz_input)
        t_orig_input = {**traz_input, "tags": tags or [], "usage_count":0, "success_count":0}
        content_for_E_orig = self._get_record_vector_text(t_orig_input)
        
        try:
            content_embedding = await self.llm_service.embed(content_for_E_orig)
            applicability_embedding = await self.llm_service.embed(applicability_text)
        except Exception as e:
            logger.error(f"Embedding generation failed for {name}: {e}", exc_info=True)
            content_embedding = [0.0] * DEFAULT_EMBEDDING_DIMENSION 
            applicability_embedding = [0.0] * DEFAULT_EMBEDDING_DIMENSION
        
        record = {
            "id": record_id, "name": name, "record_type": record_type, "domain": domain,
            "description": description, "code_snippet": code_snippet, "version": version,
            "usage_count": 0, "success_count": 0, "fail_count": 0, "status": status,
            "created_at": current_time_dt, # Store as datetime object
            "last_updated": current_time_dt, # Store as datetime object
            "tags": tags or [], 
            "metadata": {**(metadata or {}), "applicability_text": applicability_text},
            "content_embedding": content_embedding, 
            "applicability_embedding": applicability_embedding
        }
        await self.save_record(record) # save_record will handle datetime conversion if necessary
        logger.info(f"Created new {record_type} record '{name}' (ID: {record_id}) in MongoDB.")
        return record

    async def update_usage_metrics(self, record_id: str, success: bool = True) -> None:
        if self.components_collection is None: 
            logger.warning(f"Cannot update usage metrics for {record_id}: components_collection is None.")
            return
        update_result = await self.components_collection.update_one(
            {"id": record_id},
            {"$inc": {"usage_count": 1, "success_count": 1 if success else 0, "fail_count": 0 if success else 1},
             "$set": {"last_updated": datetime.now(timezone.utc)}})
        if update_result.matched_count == 0: logger.warning(f"Update metrics: record {record_id} not found.")
        else: logger.info(f"Updated usage metrics for record {record_id} (success={success}).")

    async def get_firmware(self, domain: str) -> Optional[Dict[str, Any]]:
        if self.components_collection is None: return None
        query = {"record_type": "FIRMWARE", "status": "active"}
        firmware = await self.components_collection.find_one({**query, "domain": domain}, {"_id": 0})
        return firmware if firmware else await self.components_collection.find_one({**query, "domain": "general"}, {"_id": 0})

    async def evolve_record(
        self, parent_id: str, new_code_snippet: str, description: Optional[str] = None,
        new_version: Optional[str] = None, status: str = "active"
    ) -> Dict[str, Any]:
        if self.components_collection is None:
            logger.error("Cannot evolve record: components_collection is None. MongoDB connection might have failed.")
            raise ConnectionError("SmartLibrary cannot connect to MongoDB to evolve record.")
            
        parent = await self.find_record_by_id(parent_id)
        if not parent: raise ValueError(f"Parent record {parent_id} not found")
        
        evolved_version_str = new_version or self._increment_version(parent["version"])
        current_time_dt = datetime.now(timezone.utc)
        evolved_desc = description or parent["description"]
        
        traz_input = {"name": parent["name"], "record_type": parent["record_type"], 
                      "domain": parent["domain"], "description": evolved_desc, 
                      "code_snippet": new_code_snippet}
        new_applicability_text = await self.generate_applicability_text(traz_input)
        
        t_orig_input = {**traz_input, "tags": parent.get("tags", []), "usage_count":0, "success_count":0}
        new_content_for_E_orig = self._get_record_vector_text(t_orig_input)
        
        try:
            new_content_embedding = await self.llm_service.embed(new_content_for_E_orig)
            new_applicability_embedding = await self.llm_service.embed(new_applicability_text)
        except Exception as e:
            logger.error(f"Embedding generation failed for evolved {parent['name']}: {e}", exc_info=True)
            new_content_embedding = [0.0] * DEFAULT_EMBEDDING_DIMENSION
            new_applicability_embedding = [0.0] * DEFAULT_EMBEDDING_DIMENSION
        
        evolved_metadata = parent.get("metadata", {}).copy()
        evolved_metadata.update({
            "applicability_text": new_applicability_text, 
            "evolved_at": current_time_dt.isoformat(), 
            "evolved_from": parent_id, 
            "previous_version": parent["version"]
        })
        
        evolved_record = {
            "id": str(uuid.uuid4()), "name": parent["name"], "record_type": parent["record_type"],
            "domain": parent["domain"], "description": evolved_desc, "code_snippet": new_code_snippet,
            "version": evolved_version_str, "usage_count": 0, "success_count": 0, "fail_count": 0, "status": status,
            "created_at": current_time_dt, # Store as datetime object
            "last_updated": current_time_dt, # Store as datetime object
            "parent_id": parent_id, "tags": parent.get("tags", []).copy(),
            "metadata": evolved_metadata, 
            "content_embedding": new_content_embedding,
            "applicability_embedding": new_applicability_embedding
        }
        await self.save_record(evolved_record) # save_record handles datetime
        logger.info(f"Evolved record {parent['name']} to {evolved_version_str} (ID: {evolved_record['id']}).")
        return evolved_record

    async def _update_record_status(self, record_id: str, new_status: str) -> bool:
        """
        Helper method to update the status of a record and its last_updated timestamp.
        """
        if self.components_collection is None:
            logger.error(f"Cannot update status for {record_id}: components_collection is None.")
            return False
        
        record = await self.find_record_by_id(record_id)
        if not record:
            logger.warning(f"Record {record_id} not found for status update to '{new_status}'.")
            return False
        
        record["status"] = new_status
        record["last_updated"] = datetime.now(timezone.utc) # Explicitly update timestamp
        
        try:
            await self.save_record(record)
            logger.info(f"Updated status of record {record_id} to '{new_status}'.")
            return True
        except Exception as e:
            logger.error(f"Failed to save record {record_id} after status update to '{new_status}': {e}", exc_info=True)
            return False

    async def deploy_component_version(
        self, 
        component_id: str, 
        deployed_status: str = "active", 
        parent_archived_status: str = "archived"
    ) -> bool:
        """
        Deploys a new component version by setting its status and archiving its parent.
        """
        self.logger.info(f"Attempting to deploy component version {component_id} with status '{deployed_status}'.")

        # Deploy the specified component version
        deployed_successfully = await self._update_record_status(component_id, deployed_status)
        if not deployed_successfully:
            self.logger.error(f"Failed to deploy component {component_id} (could not update status to '{deployed_status}').")
            return False
        
        self.logger.info(f"Successfully set status of component {component_id} to '{deployed_status}'.")

        # Check for parent and archive it
        component_record = await self.find_record_by_id(component_id)
        if not component_record: # Should not happen if previous step succeeded, but good to check
            self.logger.error(f"Component {component_id} not found after status update. Cannot process parent.")
            return True # Deployed component, but parent issue

        parent_id = component_record.get("parent_id") or component_record.get("metadata", {}).get("evolved_from")

        if parent_id:
            self.logger.info(f"Component {component_id} has parent_id: {parent_id}. Attempting to archive parent with status '{parent_archived_status}'.")
            parent_archived_successfully = await self._update_record_status(parent_id, parent_archived_status)
            if parent_archived_successfully:
                self.logger.info(f"Successfully archived parent component {parent_id} with status '{parent_archived_status}'.")
            else:
                self.logger.warning(f"Failed to archive parent component {parent_id}. The new version {component_id} is deployed, but parent status update failed.")
                # Still return True as the primary component was deployed.
        else:
            self.logger.info(f"Component {component_id} has no parent_id. No parent to archive.")
            
        return True # Primary component deployed successfully

    def _increment_version(self, version: str) -> str:
        parts = version.split("."); parts.extend(["0"] * (3 - len(parts)))
        try: parts[2] = str(int(parts[2]) + 1); return ".".join(parts[:3])
        except (ValueError, IndexError): logger.warning(f"Invalid version format '{version}', appending '.1'"); return f"{version}.1"

    async def search_by_tag(self, tags: List[str], record_type: Optional[str] = None) -> List[Dict[str, Any]]:
        if self.components_collection is None: return []
        query: Dict[str, Any] = {"tags": {"$in": [t.lower() for t in tags]}, "status": "active"}
        if record_type: query["record_type"] = record_type
        cursor = self.components_collection.find(query, {"_id": 0})
        return await cursor.to_list(length=None)

    async def bulk_import(self, records_data: List[Dict[str, Any]]) -> int:
        if not records_data or self.components_collection is None:
            logger.warning("Bulk import skipped: no data or components_collection is None.")
            return 0
            
        operations = []
        for r_dict in records_data:
            record_id = r_dict.get("id", str(uuid.uuid4()))
            now_dt = datetime.now(timezone.utc)

            temp_rec_data = {
                "name": r_dict.get("name", "Unnamed Component"),
                "record_type": r_dict.get("record_type", "UNKNOWN"),
                "domain": r_dict.get("domain", "general"),
                "description": r_dict.get("description", ""),
                "code_snippet": r_dict.get("code_snippet", ""),
                "tags": r_dict.get("tags", []),
                "usage_count": r_dict.get("usage_count", 0),
                "success_count": r_dict.get("success_count", 0)
            }
            applicability_text = await self.generate_applicability_text(temp_rec_data)
            content_for_E_orig = self._get_record_vector_text(temp_rec_data)
            try:
                content_embedding = await self.llm_service.embed(content_for_E_orig)
                applicability_embedding = await self.llm_service.embed(applicability_text)
            except Exception as e:
                logger.error(f"Embedding failed for bulk import {r_dict.get('name', record_id)}: {e}")
                content_embedding = [0.0] * DEFAULT_EMBEDDING_DIMENSION
                applicability_embedding = [0.0] * DEFAULT_EMBEDDING_DIMENSION
            
            # Ensure datetime fields are actual datetime objects for MongoDB
            created_at = r_dict.get("created_at", now_dt)
            if isinstance(created_at, str): created_at = datetime.fromisoformat(created_at.replace("Z", "+00:00"))
            
            last_updated = now_dt # Always set last_updated to now for bulk import/update

            full_rec = {
                **r_dict,
                "id":record_id,
                "created_at": created_at,
                "last_updated": last_updated,
                "status": r_dict.get("status", "active"),
                "metadata": {**(r_dict.get("metadata", {})), "applicability_text": applicability_text},
                "content_embedding": content_embedding,
                "applicability_embedding": applicability_embedding
            }
            for key, default_val in [("usage_count",0),("success_count",0),("fail_count",0),("tags",[])]:
                if key not in full_rec: full_rec[key] = default_val
            
            operations.append(pymongo.ReplaceOne({"id": record_id}, full_rec, upsert=True))
        
        if not operations: return 0
        try:
            result = await self.components_collection.bulk_write(operations, ordered=False)
            success_count = (result.upserted_count or 0) + (result.modified_count or 0)
            logger.info(f"Bulk imported/updated {success_count} records.")
            return success_count
        except pymongo.errors.BulkWriteError as bwe:
            logger.error(f"Error during bulk import: {bwe.details}", exc_info=True)
            return (bwe.details.get('nInserted',0) +
                    bwe.details.get('nUpserted',0) +
                    bwe.details.get('nModified',0))
        except Exception as e:
            logger.error(f"Unexpected error during bulk import: {e}", exc_info=True); return 0

    async def export_records(self, record_type: Optional[str] = None, domain: Optional[str] = None) -> List[Dict[str, Any]]:
        if self.components_collection is None: return []
        query: Dict[str, Any] = {}
        if record_type: query["record_type"] = record_type
        if domain: query["domain"] = domain
        # Exclude embeddings from export by default
        cursor = self.components_collection.find(query, {"_id": 0, "content_embedding": 0, "applicability_embedding": 0})
        
        exported_records = []
        async for doc in cursor:
            # Convert datetime objects to ISO strings for JSON compatibility if needed by caller
            if isinstance(doc.get("created_at"), datetime):
                doc["created_at"] = doc["created_at"].isoformat()
            if isinstance(doc.get("last_updated"), datetime):
                doc["last_updated"] = doc["last_updated"].isoformat()
            if isinstance(doc.get("metadata", {}).get("evolved_at"), datetime):
                doc["metadata"]["evolved_at"] = doc["metadata"]["evolved_at"].isoformat()
            if isinstance(doc.get("metadata", {}).get("creation_strategy", {}).get("timestamp"), datetime):
                 doc["metadata"]["creation_strategy"]["timestamp"] = doc["metadata"]["creation_strategy"]["timestamp"].isoformat()
            if isinstance(doc.get("metadata", {}).get("domain_adaptation", {}).get("adaptation_timestamp"), datetime):
                 doc["metadata"]["domain_adaptation"]["adaptation_timestamp"] = doc["metadata"]["domain_adaptation"]["adaptation_timestamp"].isoformat()

            exported_records.append(doc)
        return exported_records


    async def clear_cache(self, older_than: Optional[int] = None) -> int:
        if not self.llm_service or not hasattr(self.llm_service, 'cache') or not self.llm_service.cache:
            logger.info("LLM Cache is disabled or not available in LLMService, nothing to clear from SmartLibrary perspective.")
            return 0
        return await self.llm_service.clear_cache(older_than_seconds=older_than)

    async def get_status_summary(self) -> Dict[str, Any]:
        if self.components_collection is None:
            logger.error("Cannot get status summary: components_collection is None.")
            return {"error": "components_collection not initialized", "total_records": 0}

        type_pipeline = [{"$group": {"_id": "$record_type", "count": {"$sum": 1}}}]
        status_pipeline = [{"$group": {"_id": "$status", "count": {"$sum": 1}}}]
        
        try:
            type_counts_raw = await self.components_collection.aggregate(type_pipeline).to_list(length=None)
            status_counts_raw = await self.components_collection.aggregate(status_pipeline).to_list(length=None)
            
            type_counts = {item["_id"]: item["count"] for item in type_counts_raw if item.get("_id")}
            status_counts = {item["_id"]: item["count"] for item in status_counts_raw if item.get("_id")}

            distinct_domains_list = await self.components_collection.distinct("domain")
            distinct_domains = [d for d in distinct_domains_list if d] 

            projection = {"_id":0, "name":1, "id":1, "usage_count":1, "success_count":1, "last_updated":1}
            most_used = await self.components_collection.find({"status":"active"}, projection).sort("usage_count", pymongo.DESCENDING).limit(5).to_list(length=None)
            
            successful_candidates_cursor = self.components_collection.find(
                {"status":"active", "usage_count": {"$gte": 5}}, projection).limit(50)
            successful_candidates = await successful_candidates_cursor.to_list(length=None)
            for doc in successful_candidates:
                doc["success_rate"] = doc.get("success_count",0) / doc.get("usage_count",1) if doc.get("usage_count",0) > 0 else 0.0
            most_successful = sorted(successful_candidates, key=lambda x: x["success_rate"], reverse=True)[:5]

            recently_updated = await self.components_collection.find({"status":"active"}, projection).sort("last_updated", pymongo.DESCENDING).limit(5).to_list(length=None)
            total_records = await self.components_collection.count_documents({})

            return {
                "total_records": total_records,
                "by_type": { "AGENT": type_counts.get("AGENT", 0), "TOOL": type_counts.get("TOOL", 0),
                             "FIRMWARE": type_counts.get("FIRMWARE", 0),
                             **{k:v for k,v in type_counts.items() if k not in ["AGENT", "TOOL", "FIRMWARE"] and k}},
                "by_status": status_counts,
                "domains": distinct_domains,
                "most_used": [{"id": r.get("id"), "name": r.get("name"), "usage_count": r.get("usage_count", 0)} for r in most_used],
                "most_successful": [{"id": r.get("id"), "name": r.get("name"), "success_rate": r.get("success_rate", 0.0)} for r in most_successful],
                "recently_updated": [{"id": r.get("id"), "name": r.get("name"), "last_updated": r.get("last_updated").isoformat() if isinstance(r.get("last_updated"), datetime) else str(r.get("last_updated"))} for r in recently_updated]
            }
        except Exception as e:
            logger.error(f"Error getting status summary from MongoDB: {e}", exc_info=True)
            return {"error": f"Failed to get status summary: {e}", "total_records": -1}