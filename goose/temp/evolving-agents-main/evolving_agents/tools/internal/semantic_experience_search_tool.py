# evolving_agents/tools/internal/semantic_experience_search_tool.py
import logging
from typing import List, Dict, Optional, Any, Type # Added Type

from pydantic import BaseModel, Field # For input/output schemas
import pymongo # For MongoDB specific types if needed, like pymongo.ASCENDING (not directly used here but good practice)

from beeai_framework.tools.tool import Tool, StringToolOutput # Base Tool class
from beeai_framework.emitter.emitter import Emitter # Required by Tool
from beeai_framework.context import RunContext # Required for Tool._run signature

from evolving_agents.core.llm_service import LLMService
from evolving_agents.core.mongodb_client import MongoDBClient

# Configure logging
logger = logging.getLogger(__name__)

# Default fields to search against if not specified by the user.
# These are the *embedding* field names within the 'embeddings' sub-document.
DEFAULT_EMBEDDING_SEARCH_FIELDS_SEMANTIC_TOOL = [
    "embeddings.primary_goal_description_embedding",
    "embeddings.sub_task_description_embedding",
    "embeddings.input_context_summary_embedding",
    "embeddings.output_summary_embedding",
]

# Assumed name for the MongoDB Atlas Vector Search index.
DEFAULT_VECTOR_INDEX_NAME_SEMANTIC_TOOL = "vector_index_experiences_default"


class SemanticSearchInput(BaseModel):
    query_string: str = Field(..., description="The natural language query string to search for relevant experiences.")
    top_k: int = Field(default=5, gt=0, le=20, description="The number of top relevant experiences to return.")
    search_fields: Optional[List[str]] = Field(
        default=None, 
        description=(
            "Specific embedding field names within the 'embeddings' sub-document to search against "
            "(e.g., 'primary_goal_description_embedding'). If None, uses tool defaults. "
            "Currently, only the first field in this list is actively used per query."
        )
    )
    metadata_filter: Optional[Dict[str, Any]] = Field(
        default=None, 
        description="A MongoDB query document to pre-filter experiences (e.g., {'tags': 'important'})."
    )

class ExperienceSearchResult(BaseModel): # Model for a single search result item
    experience_id: str
    primary_goal_description: Optional[str] = None
    sub_task_description: Optional[str] = None
    final_outcome: Optional[str] = None
    output_summary: Optional[str] = None
    similarity_score: float
    # You can add more fields from the experience document that you want to return

class SemanticSearchOutput(BaseModel): # Overall output model for the tool
    status: str = Field(description="Status of the search operation ('success' or 'error').")
    query_used: str
    results: Optional[List[ExperienceSearchResult]] = None
    message: Optional[str] = None # For error messages or additional info
    search_details: Dict[str, Any] # To store details like fields searched, top_k


class SemanticExperienceSearchTool(Tool[SemanticSearchInput, None, SemanticSearchOutput]):
    """
    A tool to search for relevant agent experiences in MongoDB using semantic vector search.
    This tool queries the 'eat_agent_experiences' collection.
    It requires a MongoDB Atlas Vector Search index to be configured on this collection.
    """
    name: str = "SemanticExperienceSearchTool"
    description: str = (
        "Searches for relevant agent experiences in MongoDB using vector embeddings "
        "and a natural language query. Crucial for retrieving past learnings."
    )
    input_schema: Type[BaseModel] = SemanticSearchInput
    output_schema: Type[BaseModel] = SemanticSearchOutput


    def __init__(
        self,
        mongodb_client: MongoDBClient,
        llm_service: LLMService,
        collection_name: str = "eat_agent_experiences",
        default_search_fields: Optional[List[str]] = None,
        vector_index_name: str = DEFAULT_VECTOR_INDEX_NAME_SEMANTIC_TOOL,
        options: Optional[Dict[str, Any]] = None # For Tool base class
    ):
        super().__init__(options=options) # Call Tool's __init__
        if mongodb_client is None: raise ValueError("mongodb_client is required")
        if llm_service is None: raise ValueError("llm_service is required")

        self.mongodb_client = mongodb_client
        self.llm_service = llm_service
        self.collection_name = collection_name
        self.collection = self.mongodb_client.get_collection(self.collection_name)
        self.default_search_fields = default_search_fields or list(DEFAULT_EMBEDDING_SEARCH_FIELDS_SEMANTIC_TOOL)
        self.vector_index_name = vector_index_name

        if not self.default_search_fields:
            logger.warning(f"{self.name} initialized with no default_search_fields. Searches will require 'search_fields' to be explicitly provided.")

    def _create_emitter(self) -> Emitter: # Implement required method
        return Emitter.root().child(
            namespace=["tool", "internal", "semantic_experience_search"], # Adjusted namespace
            creator=self,
        )

    async def _run(
        self, 
        input: SemanticSearchInput, 
        options: Optional[Dict[str, Any]] = None, 
        context: Optional[RunContext] = None
    ) -> SemanticSearchOutput:
        """
        Searches for relevant experiences using vector similarity.
        """
        search_details_log = {
            "query": input.query_string,
            "top_k": input.top_k,
            "search_fields_requested": input.search_fields,
            "metadata_filter": input.metadata_filter,
            "vector_index_name_used": self.vector_index_name
        }

        if not input.query_string:
            logger.warning(f"{self.name}: called with empty query_string.")
            return SemanticSearchOutput(status="error", query_used=input.query_string, message="Query string cannot be empty.", search_details=search_details_log)

        try:
            query_embedding = await self.llm_service.embed(input.query_string)
            if not query_embedding or not all(isinstance(x, float) for x in query_embedding):
                logger.error(f"{self.name}: Failed to generate a valid query embedding for: '{input.query_string}'. Embedding: {query_embedding}")
                return SemanticSearchOutput(status="error", query_used=input.query_string, message="Failed to generate valid query embedding.", search_details=search_details_log)
        except Exception as e:
            logger.error(f"{self.name}: Error generating query embedding: {e}", exc_info=True)
            return SemanticSearchOutput(status="error", query_used=input.query_string, message=f"Embedding generation error: {e}", search_details=search_details_log)

        target_search_fields = input.search_fields or self.default_search_fields
        if not target_search_fields:
            msg = "No search fields specified or configured for semantic search."
            logger.error(f"{self.name}: {msg}")
            return SemanticSearchOutput(status="error", query_used=input.query_string, message=msg, search_details=search_details_log)

        # Use the first field for the $vectorSearch path.
        # Full path needs to be like "embeddings.primary_goal_description_embedding"
        embedding_field_path = target_search_fields[0]
        if not embedding_field_path.startswith("embeddings.") and "_embedding" in embedding_field_path:
            embedding_field_path = f"embeddings.{embedding_field_path}"
        
        search_details_log["actual_embedding_field_path_used"] = embedding_field_path
        
        if len(target_search_fields) > 1:
            logger.warning(
                f"{self.name}: Multiple search_fields provided: {target_search_fields}. "
                f"Currently using only the first field for $vectorSearch: '{embedding_field_path}'."
            )
        
        num_candidates = input.top_k * 15 # Fetch more candidates for KNN

        vector_search_stage = {
            "$vectorSearch": {
                "index": self.vector_index_name,
                "path": embedding_field_path,
                "queryVector": query_embedding,
                "numCandidates": num_candidates,
                "limit": input.top_k, # Limit results from the vector search stage itself
            }
        }
        if input.metadata_filter:
            vector_search_stage["$vectorSearch"]["filter"] = input.metadata_filter

        # Define fields to project from the experience document, plus the score
        # Exclude the large 'embeddings' object from the results by default
        projection = {
            "_id": 0, # Exclude MongoDB's default _id
            "experience_id": 1,
            "primary_goal_description": 1,
            "sub_task_description": 1,
            "final_outcome": 1,
            "output_summary": 1,
            "tags": 1,
            "timestamp": 1,
            # "involved_components": 1, # Add more fields as needed
            "similarity_score": {"$meta": "vectorSearchScore"}
        }
        project_stage = {"$project": projection}

        pipeline = [vector_search_stage, project_stage]
        logger.debug(f"{self.name}: Executing MongoDB aggregation pipeline: {pipeline}")

        try:
            cursor = self.collection.aggregate(pipeline)
            raw_results = await cursor.to_list(length=input.top_k)
            
            # Parse into Pydantic models for clean output
            parsed_results = []
            for res_dict in raw_results:
                # Map fields from res_dict to ExperienceSearchResult fields
                search_result_item = ExperienceSearchResult(
                    experience_id=res_dict.get("experience_id", "N/A"),
                    primary_goal_description=res_dict.get("primary_goal_description"),
                    sub_task_description=res_dict.get("sub_task_description"),
                    final_outcome=res_dict.get("final_outcome"),
                    output_summary=res_dict.get("output_summary"),
                    similarity_score=res_dict.get("similarity_score", 0.0)
                    # map other fields if added to ExperienceSearchResult and projection
                )
                parsed_results.append(search_result_item)

            logger.info(f"{self.name}: Found {len(parsed_results)} relevant experiences for query: '{input.query_string}'.")
            return SemanticSearchOutput(
                status="success", 
                query_used=input.query_string, 
                results=parsed_results, 
                message=f"Found {len(parsed_results)} results.",
                search_details=search_details_log
            )
        except pymongo.errors.OperationFailure as e:
            # Handle specific errors like index not found
            error_msg = f"MongoDB operation error during semantic search: {e}"
            logger.error(f"{self.name}: {error_msg}", exc_info=True)
            if "index not found" in str(e).lower() or "unknown search index" in str(e).lower() or "Invalid $vectorSearch" in str(e).lower():
                 error_msg = f"CRITICAL: Atlas Vector Search index '{self.vector_index_name}' on collection '{self.collection_name}' is likely missing, misconfigured, or not yet active. Details: {e}"
                 logger.error(f"{self.name}: {error_msg}")
            return SemanticSearchOutput(status="error", query_used=input.query_string, message=error_msg, search_details=search_details_log)
        except Exception as e:
            error_msg = f"Unexpected error during semantic search: {e}"
            logger.error(f"{self.name}: {error_msg}", exc_info=True)
            return SemanticSearchOutput(status="error", query_used=input.query_string, message=error_msg, search_details=search_details_log)


# Example Usage (Conceptual - requires full EAT setup)
# async def main_example():
#     # ... (Setup container with LLMService, MongoDBClient) ...
#     # mongo_client = container.get('mongodb_client')
#     # llm_service = container.get('llm_service')
#     # search_tool = SemanticExperienceSearchTool(mongodb_client=mongo_client, llm_service=llm_service)

#     # search_input_data = SemanticSearchInput(
#     #     query_string="How to effectively route billing inquiries?",
#     #     top_k=3,
#     #     search_fields=["primary_goal_description_embedding", "sub_task_description_embedding"], # Tool will prefix with "embeddings."
#     #     metadata_filter={"tags": "support"}
#     # )
#     # search_results = await search_tool._run(input=search_input_data)
#     # print(f"\n--- Semantic Search Results for 'billing inquiries' ---")
#     # print(search_results.model_dump_json(indent=2))

#     # search_input_data_2 = SemanticSearchInput(
#     #     query_string="Best practices for technical troubleshooting via chat",
#     #     top_k=2
#     # )
#     # search_results_2 = await search_tool._run(input=search_input_data_2)
#     # print(f"\n--- Semantic Search Results for 'technical troubleshooting' ---")
#     # print(search_results_2.model_dump_json(indent=2))
#     pass

# if __name__ == "__main__":
#     import asyncio
#     # asyncio.run(main_example())
#     pass