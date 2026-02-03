# evolving_agents/tools/internal/mongo_experience_store_tool.py
import uuid
import logging
from datetime import datetime, timezone
from typing import List, Dict, Optional, Any, Union, Type # Added Union, Type

from pydantic import BaseModel, Field, field_validator, model_validator # For input/output schemas

from beeai_framework.tools.tool import Tool, StringToolOutput # Base Tool class
from beeai_framework.emitter.emitter import Emitter # Required by Tool
from beeai_framework.context import RunContext # Required for Tool._run signature

from evolving_agents.core.llm_service import LLMService
from evolving_agents.core.mongodb_client import MongoDBClient

# Configure logging
logger = logging.getLogger(__name__)

# --- Pydantic Input Schemas for Specific Actions ---
class StoreExperienceParams(BaseModel):
    experience_data: Dict[str, Any] = Field(..., description="The experience data dictionary to store. Must conform to the eat_agent_experiences_schema.md.")

class GetExperienceParams(BaseModel):
    experience_id: str = Field(..., description="The unique ID of the experience to retrieve.")

class UpdateExperienceParams(BaseModel):
    experience_id: str = Field(..., description="The unique ID of the experience to update.")
    update_data: Dict[str, Any] = Field(..., description="A dictionary containing the fields and new values to update.")

class DeleteExperienceParams(BaseModel):
    experience_id: str = Field(..., description="The unique ID of the experience to delete.")

# Unified Input Schema for the Tool
class MongoExperienceStoreToolInput(BaseModel):
    action: str = Field(..., description="Action to perform: 'store', 'get', 'update', or 'delete'.")
    params: Dict[str, Any] # Parameters for the specific action, will be validated further in _run

    @model_validator(mode='after')
    def check_params_for_action(cls, data: Any) -> Any:
        if not isinstance(data, dict): # Should be a dict if validation is successful up to this point
            return data 
            
        action = data.get('action')
        params = data.get('params')

        if action == "store":
            StoreExperienceParams(**params) # Validate against specific schema
        elif action == "get":
            GetExperienceParams(**params)
        elif action == "update":
            UpdateExperienceParams(**params)
        elif action == "delete":
            DeleteExperienceParams(**params)
        # No need to do anything else if validation passes, Pydantic raises error otherwise
        return data


# Output Schema for the Tool
class MongoExperienceStoreToolOutput(BaseModel):
    status: str = Field(description="Status of the operation (e.g., 'success', 'error', 'not_found').")
    result: Optional[Any] = Field(default=None, description="The result of the operation, e.g., an experience document for 'get', or an ID for 'store'.")
    message: Optional[str] = Field(default=None, description="A message detailing the outcome or error.")
    action_performed: str


class MongoExperienceStoreTool(Tool[MongoExperienceStoreToolInput, None, MongoExperienceStoreToolOutput]):
    """
    A tool to store, retrieve, update, and delete agent experiences in a MongoDB collection.
    It also handles generating and updating embeddings for specified textual fields within experiences.
    This tool is typically used internally by the MemoryManagerAgent.
    """
    name: str = "MongoExperienceStoreTool"
    description: str = (
        "Manages agent experiences in MongoDB. Supported actions: "
        "'store' (params: experience_data), "
        "'get' (params: experience_id), "
        "'update' (params: experience_id, update_data), "
        "'delete' (params: experience_id)."
    )
    input_schema: Type[BaseModel] = MongoExperienceStoreToolInput
    output_schema: Type[BaseModel] = MongoExperienceStoreToolOutput

    EMBEDDABLE_FIELDS: List[str] = [
        "primary_goal_description",
        "sub_task_description",
        "input_context_summary",
        "output_summary",
    ]

    def __init__(
        self,
        mongodb_client: MongoDBClient,
        llm_service: LLMService,
        collection_name: str = "eat_agent_experiences",
        options: Optional[Dict[str, Any]] = None # For Tool base class
    ):
        super().__init__(options=options) # Call Tool's __init__
        if mongodb_client is None: raise ValueError("mongodb_client is required for MongoExperienceStoreTool")
        if llm_service is None: raise ValueError("llm_service is required for MongoExperienceStoreTool for embedding generation")
        
        self.mongodb_client = mongodb_client
        self.llm_service = llm_service
        self.collection_name = collection_name
        self.collection = self.mongodb_client.get_collection(self.collection_name)

    def _create_emitter(self) -> Emitter: # Implement required method
        return Emitter.root().child(
            namespace=["tool", "internal", "mongo_experience_store"], # Adjusted namespace
            creator=self,
        )

    async def _generate_embeddings(self, experience_data: Dict[str, Any]) -> Dict[str, List[float]]:
        embeddings: Dict[str, List[float]] = {}
        for field_name in self.EMBEDDABLE_FIELDS:
            text_to_embed = experience_data.get(field_name) # Use .get() for safety
            if text_to_embed and isinstance(text_to_embed, str):
                try:
                    embedding_vector = await self.llm_service.embed(text_to_embed)
                    if embedding_vector and isinstance(embedding_vector, list) and all(isinstance(x, float) for x in embedding_vector):
                        embeddings[f"{field_name}_embedding"] = embedding_vector
                    else:
                        logger.warning(f"Received invalid or empty embedding for field: {field_name}. Vector: {embedding_vector}")
                except Exception as e:
                    logger.error(f"Error generating embedding for field {field_name}: {e}", exc_info=True)
        return embeddings

    async def _store_experience(self, params: StoreExperienceParams) -> MongoExperienceStoreToolOutput:
        experience_data = params.experience_data.copy() # Work on a copy

        if "experience_id" not in experience_data or not experience_data["experience_id"]:
            experience_data["experience_id"] = uuid.uuid4().hex
        
        # Ensure timestamp is a BSON-compatible datetime object for MongoDB TTL and queries
        timestamp_str = experience_data.get("timestamp", datetime.now(timezone.utc).isoformat())
        try:
            experience_data["timestamp"] = datetime.fromisoformat(timestamp_str.replace("Z", "+00:00"))
        except ValueError:
            logger.warning(f"Invalid ISO format for timestamp '{timestamp_str}'. Defaulting to now.")
            experience_data["timestamp"] = datetime.now(timezone.utc)

        if "version" not in experience_data: experience_data["version"] = 1

        generated_embeddings = await self._generate_embeddings(experience_data)
        if "embeddings" not in experience_data: experience_data["embeddings"] = {}
        experience_data["embeddings"].update(generated_embeddings)

        await self.collection.insert_one(experience_data)
        logger.info(f"Successfully stored experience with ID: {experience_data['experience_id']}")
        return MongoExperienceStoreToolOutput(
            status="success", 
            result={"experience_id": experience_data['experience_id']},
            message="Experience stored successfully.",
            action_performed="store"
        )

    async def _get_experience(self, params: GetExperienceParams) -> MongoExperienceStoreToolOutput:
        document = await self.collection.find_one({"experience_id": params.experience_id}, {"_id": 0})
        if document:
            logger.debug(f"Successfully retrieved experience with ID: {params.experience_id}")
            return MongoExperienceStoreToolOutput(status="success", result=document, action_performed="get")
        else:
            logger.info(f"No experience found with ID: {params.experience_id}")
            return MongoExperienceStoreToolOutput(status="not_found", message=f"Experience ID {params.experience_id} not found.", action_performed="get")

    async def _update_experience(self, params: UpdateExperienceParams) -> MongoExperienceStoreToolOutput:
        update_data = params.update_data.copy()
        update_payload = {"$set": update_data}

        # If any embeddable fields are being updated, regenerate their embeddings
        # This requires having the full text of the fields to be re-embedded.
        # A more robust way is to fetch the doc, merge updates, then re-embed.
        # For now, we assume `update_data` contains the full new text if an embeddable field is changed.
        needs_embedding_update = any(field in update_data for field in self.EMBEDDABLE_FIELDS)
        if needs_embedding_update:
            # To correctly re-embed, we ideally need the full document context
            # For simplicity, if an embeddable field is in update_data, we re-embed it.
            # This might be suboptimal if only part of an embeddable text changes.
            # A better approach would be:
            # 1. Fetch existing doc.
            # 2. Merge update_data into it.
            # 3. Pass the merged doc to _generate_embeddings.
            # This demo will re-embed based *only* on the text in `update_data`.
            current_doc = await self.collection.find_one({"experience_id": params.experience_id})
            if not current_doc:
                return MongoExperienceStoreToolOutput(status="not_found", message=f"Experience ID {params.experience_id} not found for update.", action_performed="update")

            merged_data_for_embedding = {**current_doc, **update_data} # Merge for full context

            new_embeddings = await self._generate_embeddings(merged_data_for_embedding)
            if new_embeddings:
                if "embeddings" not in update_payload["$set"]:
                    update_payload["$set"]["embeddings"] = {}
                for key, value in new_embeddings.items():
                    update_payload["$set"][f"embeddings.{key}"] = value # Update specific embedding fields

        # Add/update a 'last_modified_at' field
        update_payload["$set"]["last_modified_at"] = datetime.now(timezone.utc)

        result = await self.collection.update_one(
            {"experience_id": params.experience_id},
            update_payload
        )
        if result.matched_count > 0:
            logger.info(f"Successfully updated experience with ID: {params.experience_id}. Modified: {result.modified_count > 0}")
            return MongoExperienceStoreToolOutput(status="success", result={"matched_count": result.matched_count, "modified_count": result.modified_count}, message="Experience updated.", action_performed="update")
        else:
            logger.warning(f"No experience found with ID: {params.experience_id} to update.")
            return MongoExperienceStoreToolOutput(status="not_found", message=f"Experience ID {params.experience_id} not found for update.", action_performed="update")

    async def _delete_experience(self, params: DeleteExperienceParams) -> MongoExperienceStoreToolOutput:
        result = await self.collection.delete_one({"experience_id": params.experience_id})
        if result.deleted_count > 0:
            logger.info(f"Successfully deleted experience with ID: {params.experience_id}")
            return MongoExperienceStoreToolOutput(status="success", result={"deleted_count": result.deleted_count}, message="Experience deleted.", action_performed="delete")
        else:
            logger.warning(f"No experience found with ID: {params.experience_id} to delete.")
            return MongoExperienceStoreToolOutput(status="not_found", message=f"Experience ID {params.experience_id} not found for deletion.", action_performed="delete")

    async def _run(
        self, 
        input: MongoExperienceStoreToolInput, 
        options: Optional[Dict[str, Any]] = None, 
        context: Optional[RunContext] = None
    ) -> MongoExperienceStoreToolOutput:
        
        action = input.action.lower()
        params_dict = input.params # Already a dict due to Pydantic model
        
        try:
            if action == "store":
                # Re-validate/parse params_dict into StoreExperienceParams if necessary
                # Pydantic's Union with a smart @model_validator in the input model already handles this.
                action_params = StoreExperienceParams(**params_dict)
                return await self._store_experience(action_params)
            elif action == "get":
                action_params = GetExperienceParams(**params_dict)
                return await self._get_experience(action_params)
            elif action == "update":
                action_params = UpdateExperienceParams(**params_dict)
                return await self._update_experience(action_params)
            elif action == "delete":
                action_params = DeleteExperienceParams(**params_dict)
                return await self._delete_experience(action_params)
            else:
                logger.error(f"Unsupported action for MongoExperienceStoreTool: {action}")
                return MongoExperienceStoreToolOutput(status="error", message=f"Unsupported action: {action}", action_performed=action)
        
        except Exception as e: # Catches Pydantic validation errors from **params_dict if types are wrong
            logger.error(f"Error processing action '{action}' with params '{params_dict}': {e}", exc_info=True)
            return MongoExperienceStoreToolOutput(status="error", message=f"Error during action '{action}': {str(e)}", action_performed=action)

# Example Usage (Conceptual - requires full EAT setup)
# async def main_example():
#     # ... (Setup container with LLMService, MongoDBClient) ...
#     # mongo_client = container.get('mongodb_client')
#     # llm_service = container.get('llm_service')
#     # experience_tool = MongoExperienceStoreTool(mongodb_client=mongo_client, llm_service=llm_service)

#     # # Test store_experience
#     # store_input_data = MongoExperienceStoreToolInput(
#     #     action="store",
#     #     params=StoreExperienceParams(experience_data={
#     #         "primary_goal_description": "Test storing a new goal via tool",
#     #         "sub_task_description": "This is the sub-task for the new goal.",
#     #         "final_outcome": "pending_tool_test"
#     #     })
#     # )
#     # store_result = await experience_tool._run(input=store_input_data)
#     # print(f"Store result: {store_result.model_dump_json(indent=2)}")
#     # experience_id_to_test = store_result.result.get("experience_id") if store_result.result else None

#     # if experience_id_to_test:
#     #     # Test get_experience
#     #     get_input_data = MongoExperienceStoreToolInput(
#     #         action="get",
#     #         params=GetExperienceParams(experience_id=experience_id_to_test)
#     #     )
#     #     get_result = await experience_tool._run(input=get_input_data)
#     #     print(f"Get result: {get_result.model_dump_json(indent=2)}")
#     pass

# if __name__ == "__main__":
#     import asyncio
#     # asyncio.run(main_example())
#     pass