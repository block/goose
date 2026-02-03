# evolving_agents/monitoring/component_experience_tracker.py
import logging
from datetime import datetime, timezone
from typing import Optional, Dict, Any, List 

import motor.motor_asyncio
from pymongo import ReturnDocument 
from pydantic import BaseModel, Field # Added for A/B Test Schema
from datetime import datetime, timezone # Ensure datetime and timezone are imported

from evolving_agents.core.mongodb_client import MongoDBClient

logger = logging.getLogger(__name__)


# Pydantic Schemas for A/B Test Data
class AgentABTestResultDetails(BaseModel):
    """Detailed results for a single agent in an A/B test."""
    aggregated_scores: Dict[str, float] = Field(default_factory=dict)
    average_response_time_ms: Optional[float] = None
    # raw_outputs: Optional[List[Dict[str, Any]]] = None # Consider if storing all raw outputs is feasible

class ABTestRecordSchema(BaseModel):
    """Schema for storing A/B test results."""
    agent_a_id: str
    agent_b_id: str
    test_inputs_summary: List[Dict[str, Any]] # Could be list of input hashes or summaries
    evaluation_criteria: List[str]
    agent_a_results: AgentABTestResultDetails
    agent_b_results: AgentABTestResultDetails
    detailed_comparison: Optional[List[Dict[str, Any]]] = None # Per-input scores or full comparison data
    overall_winner: str # "Agent A", "Agent B", "Tie"
    percentage_difference: Optional[float] = None
    test_timestamp: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    # Optional: Link to evolution event if applicable
    # evolution_event_id: Optional[str] = None 

class ComponentExperienceTracker:
    """
    Tracks and stores experience data for components (Agents/Tools) in MongoDB.

    Schema for 'eat_component_experiences' collection:
    - _id: MongoDB's default ObjectId.
    - component_id: str, Indexed. (Primary key for components)
    - name: str. (Component's name, for readability)
    - record_type: str. (e.g., "AGENT", "TOOL")
    - total_invocations: int.
    - successful_invocations: int.
    - failed_invocations: int.
    - total_execution_time_ms: float. (Sum of all execution times)
    - average_response_time_ms: float. (Calculated: total_execution_time_ms / total_invocations)
    - last_invocation_timestamp: datetime. (Timestamp of the most recent invocation)
    - first_invocation_timestamp: datetime. (Timestamp of the first recorded invocation)
    - error_types_summary: dict. (e.g., {"ValueError": 10, "TimeoutError": 5, "OtherError": 1})
    - evolution_history_summary: list. 
      (e.g., [{"evolved_to_id": "new_id", "timestamp": "datetime_iso", "strategy": "manual_refactor", "changes_summary": "updated X"}])
    - input_params_summary: dict. (Optional, for tracking common input patterns, can be empty for now)
    - output_summary_tracking: dict. (Optional, for tracking common output patterns, can be empty for now)
    """

    def __init__(self, mongodb_client: MongoDBClient, experience_collection_name: str = "eat_component_experiences", ab_test_collection_name: str = "eat_ab_test_results"):
        self.mongodb_client = mongodb_client
        self.logger = logging.getLogger(__name__) # Moved logger initialization up

        if not isinstance(self.mongodb_client.client, motor.motor_asyncio.AsyncIOMotorClient):
            logger.critical("ComponentExperienceTracker: MongoDBClient is NOT using an AsyncIOMotorClient (Motor). Async DB ops will fail.")
            self.experience_collection = None
            self.ab_test_collection = None # Initialize ab_test_collection to None as well
        else:
            self.experience_collection = self.mongodb_client.get_collection(experience_collection_name)
            self.ab_test_collection = self.mongodb_client.get_collection(ab_test_collection_name) # Initialize ab_test_collection
        
        # Ensure asyncio tasks are created only if event loop is running
        try:
            loop = asyncio.get_running_loop()
            loop.create_task(self._ensure_indexes())
        except RuntimeError: # No running event loop
            self.logger.warning("No running asyncio event loop. Index creation will be skipped. Ensure init is called within an async context or manage indexes externally.")


    async def _ensure_indexes(self):
        """Ensure necessary indexes exist on the collections."""
        if self.experience_collection is not None:
            try:
                await self.experience_collection.create_index([("component_id", 1)], unique=True, background=True)
                self.logger.info(f"Ensured 'component_id' index on '{self.experience_collection.name}'.")
            except Exception as e:
                self.logger.error(f"Error creating MongoDB indexes for {self.experience_collection.name}: {e}", exc_info=True)
        
        if self.ab_test_collection is not None: # Add index creation for the new collection
            try:
                await self.ab_test_collection.create_index([("agent_a_id", 1)], background=True)
                await self.ab_test_collection.create_index([("agent_b_id", 1)], background=True)
                await self.ab_test_collection.create_index([("test_timestamp", -1)], background=True) # -1 for descending sort
                self.logger.info(f"Ensured indexes on '{self.ab_test_collection.name}' for agent_a_id, agent_b_id, and test_timestamp.")
            except Exception as e:
                self.logger.error(f"Error creating MongoDB indexes for {self.ab_test_collection.name}: {e}", exc_info=True)


    async def record_event(
        self,
        component_id: str,
        name: str,
        record_type: str,
        duration_ms: float,
        success: bool,
        error: Optional[str] = None,
        input_params: Optional[Dict[str, Any]] = None, # Not used yet, but part of schema
        output_summary: Optional[str] = None # Not used yet, but part of schema
    ) -> None:
        if self.experience_collection is None:
            self.logger.error("Experience collection is not initialized. Cannot record event.")
            return

        current_time = datetime.now(timezone.utc)
        update_doc: Dict[str, Any] = {"$set": {}, "$inc": {}}

        update_doc["$inc"]["total_invocations"] = 1
        update_doc["$inc"]["total_execution_time_ms"] = duration_ms
        update_doc["$set"]["last_invocation_timestamp"] = current_time
        # Initialize fields that might not be present in $inc during upsert
        update_doc["$setOnInsert"] = {
            "component_id": component_id,
            "name": name,
            "record_type": record_type,
            "first_invocation_timestamp": current_time,
            "average_response_time_ms": 0.0, # Initial value, will be calculated and updated
            "error_types_summary": {},       # Initial empty dict
            "evolution_history_summary": [], # Initial empty list
            "input_params_summary": {},      # Initial empty dict
            "output_summary_tracking": {}    # Initial empty dict
            # DO NOT include: total_invocations, successful_invocations, 
            # failed_invocations, total_execution_time_ms here.
            # These will be correctly initialized by the $inc operator.
        }

        if success:
            update_doc["$inc"]["successful_invocations"] = 1
        else:
            update_doc["$inc"]["failed_invocations"] = 1
            if error:
                # Extract error type (e.g., "ValueError" from "ValueError: Some message")
                error_type = error.split(":")[0].strip()
                if not error_type: # Handle cases where error message might not have ':'
                    error_type = "UnknownError"
                # Sanitize error_type to be a valid MongoDB key (replace '.' and '$')
                error_type = error_type.replace(".", "_").replace("$", "_")
                update_doc["$inc"][f"error_types_summary.{error_type}"] = 1
        
        # Remove empty $set or $inc to avoid MongoDB errors if they are empty
        if not update_doc["$set"]:
            del update_doc["$set"]
        if not update_doc["$inc"]:
            del update_doc["$inc"]

        try:
            self.logger.debug(f"Attempting to record event for component '{component_id}': {update_doc}")
            updated_document = await self.experience_collection.find_one_and_update(
                {"component_id": component_id},
                update_doc,
                upsert=True,
                return_document=ReturnDocument.AFTER # Get the document after update
            )

            if updated_document:
                # Calculate and set average_response_time_ms
                # total_invocations and total_execution_time_ms are now definitely set by the update
                # or were pre-existing.
                current_total_invocations = updated_document.get("total_invocations", 0)
                current_total_execution_time = updated_document.get("total_execution_time_ms", 0.0)

                if current_total_invocations > 0:
                    avg_response_time = current_total_execution_time / current_total_invocations
                    await self.experience_collection.update_one(
                        {"component_id": component_id},
                        {"$set": {"average_response_time_ms": avg_response_time}}
                    )
                    self.logger.debug(f"Updated average_response_time_ms for '{component_id}' to {avg_response_time:.2f} ms.")
                else: # Should not happen with proper upsert and $inc logic
                    self.logger.warning(f"Total_invocations is 0 for component '{component_id}' after update. Avg response time not updated.")

                self.logger.info(f"Event recorded for component '{component_id}' ({name}). Success: {success}, Duration: {duration_ms:.2f}ms.")

            else: # Should not happen if upsert is true and operation is successful
                 self.logger.error(f"Failed to record event for component '{component_id}'. find_one_and_update returned None despite upsert=True.")

        except Exception as e:
            self.logger.error(f"Error recording experience for component '{component_id}': {e}", exc_info=True)

    async def add_evolution_event(
        self,
        component_id: str,
        evolved_to_id: str,
        strategy: str,
        changes_summary: str,
        timestamp: Optional[datetime] = None
    ) -> None:
        """
        Adds an evolution event to a component's experience record.
        (Placeholder - to be fully implemented in later milestones)
        """
        if self.experience_collection is None:
            self.logger.error("Experience collection is not initialized. Cannot add evolution event.")
            return

        event_timestamp = timestamp or datetime.now(timezone.utc)
        evolution_entry = {
            "evolved_to_id": evolved_to_id,
            "timestamp": event_timestamp.isoformat(),
            "strategy": strategy,
            "changes_summary": changes_summary
        }
        try:
            result = await self.experience_collection.update_one(
                {"component_id": component_id},
                {"$push": {"evolution_history_summary": evolution_entry},
                 "$setOnInsert": { # Ensure base document exists if this is the first interaction
                     "component_id": component_id,
                     "name": f"Unknown (Evolved from {component_id})", # Placeholder name
                     "record_type": "UNKNOWN", # Placeholder type
                     "first_invocation_timestamp": event_timestamp,
                     "last_invocation_timestamp": event_timestamp,
                     "total_invocations": 0,
                     "successful_invocations": 0,
                     "failed_invocations": 0,
                     "total_execution_time_ms": 0.0,
                     "average_response_time_ms": 0.0,
                     "error_types_summary": {},
                     "input_params_summary": {},
                     "output_summary_tracking": {}
                 }},
                upsert=True
            )
            if result.modified_count > 0 or result.upserted_id:
                self.logger.info(f"Evolution event added for component '{component_id}' evolving to '{evolved_to_id}'.")
            else:
                self.logger.warning(f"Evolution event for component '{component_id}' did not modify the document. This might indicate the component_id does not exist and upsert failed, or the update was a no-op.")

        except Exception as e:
            self.logger.error(f"Error adding evolution event for component '{component_id}': {e}", exc_info=True)

    async def get_experience(self, component_id: str) -> Optional[Dict[str, Any]]:
        """
        Retrieves the experience document for a component.
        (Placeholder - to be fully implemented in later milestones)
        """
        if self.experience_collection is None:
            self.logger.error("Experience collection is not initialized. Cannot get experience.")
            return None
        try:
            experience_doc = await self.experience_collection.find_one({"component_id": component_id})
            if experience_doc:
                self.logger.debug(f"Retrieved experience for component '{component_id}'.")
            else:
                self.logger.debug(f"No experience found for component '{component_id}'.")
            return experience_doc
        except Exception as e:
            self.logger.error(f"Error retrieving experience for component '{component_id}': {e}", exc_info=True)
            return None

    async def get_top_n_components_by_metric(
        self,
        metric: str,
        n: int = 10,
        ascending: bool = False,
        record_type_filter: Optional[str] = None
    ) -> List[Dict[str, Any]]:
        """
        Retrieves top N components sorted by a specific metric.
        (Placeholder - to be fully implemented for EvolutionStrategistAgent)
        Args:
            metric: The field to sort by (e.g., "failed_invocations", "average_response_time_ms").
            n: Number of components to return.
            ascending: Sort order.
            record_type_filter: Optional filter by component type ("AGENT" or "TOOL").
        """
        if self.experience_collection is None:
            self.logger.error("Experience collection is not initialized. Cannot get top N components.")
            return []

        self.logger.info(f"Querying top {n} components by metric '{metric}', ascending: {ascending}, type: {record_type_filter}")
        # Example: query = await self.experience_collection.find(query_filter).sort(metric, 1 if ascending else -1).limit(n).to_list(length=n)
        # This is a placeholder implementation.
        # Actual implementation will require careful construction of query_filter and sort order.
        # For instance, for "success_rate", a derived metric, an aggregation pipeline would be needed.
        
        query_filter = {}
        if record_type_filter:
            query_filter["record_type"] = record_type_filter
            
        sort_order = 1 if ascending else -1

        try:
            cursor = self.experience_collection.find(query_filter).sort(metric, sort_order).limit(n)
            results = await cursor.to_list(length=n)
            self.logger.debug(f"Found {len(results)} components for metric '{metric}'.")
            return results
        except Exception as e:
            self.logger.error(f"Error retrieving top N components by metric '{metric}': {e}", exc_info=True)
            return []

# Example of how to ensure asyncio tasks are handled if the class is instantiated globally or in a non-async context
# This is more relevant if the class is used in a way that its __init__ might not be awaited directly in an async loop.
# For this specific project structure, it's likely managed by an outer async framework.

    async def record_ab_test_summary(self, test_data: Dict[str, Any]) -> Optional[str]:
        """
        Records the summary of an A/B test into the 'eat_ab_test_results' collection.
        Validates the input data against ABTestRecordSchema.
        Returns the ID of the inserted record if successful, None otherwise.
        """
        if self.ab_test_collection is None:
            self.logger.error("A/B test result collection is not initialized. Cannot record A/B test summary.")
            return None

        try:
            # Validate data using the Pydantic model
            validated_data = ABTestRecordSchema(**test_data)
            # Convert Pydantic model to dict for MongoDB insertion, ensuring datetime is handled
            data_to_insert = validated_data.model_dump(mode="python") # mode="python" converts datetime to Python datetime

            self.logger.debug(f"Attempting to record A/B test summary: {data_to_insert}")
            
            result = await self.ab_test_collection.insert_one(data_to_insert)
            inserted_id = str(result.inserted_id)
            
            self.logger.info(f"A/B test summary recorded successfully for agents {validated_data.agent_a_id} vs {validated_data.agent_b_id}. Record ID: {inserted_id}")
            return inserted_id
        except Exception as e: # Catch Pydantic validation errors and MongoDB errors
            self.logger.error(f"Error recording A/B test summary: {e}", exc_info=True)
            return None

    async def get_ab_test_results_for_component(self, component_id: str, limit: int = 5) -> List[Dict[str, Any]]:
        """
        Retrieves A/B test results for a specific component, sorted by recency.
        Args:
            component_id: The ID of the component to fetch results for.
            limit: The maximum number of test results to return.
        Returns:
            A list of A/B test result documents.
        """
        if self.ab_test_collection is None:
            self.logger.error("A/B test result collection is not initialized. Cannot get A/B test results.")
            return []

        self.logger.info(f"Fetching latest {limit} A/B test results for component_id: {component_id}")
        try:
            query = {
                "$or": [
                    {"agent_a_id": component_id},
                    {"agent_b_id": component_id}
                ]
            }
            cursor = self.ab_test_collection.find(query).sort("test_timestamp", -1).limit(limit)
            results = await cursor.to_list(length=limit)
            self.logger.debug(f"Found {len(results)} A/B test results for component_id: {component_id}.")
            return results
        except Exception as e:
            self.logger.error(f"Error retrieving A/B test results for component '{component_id}': {e}", exc_info=True)
            return []

    async def get_latest_ab_test_results(self, limit: int = 10) -> List[Dict[str, Any]]:
        """
        Retrieves the latest A/B test results, sorted by recency.
        Args:
            limit: The maximum number of test results to return.
        Returns:
            A list of A/B test result documents.
        """
        if self.ab_test_collection is None:
            self.logger.error("A/B test result collection is not initialized. Cannot get latest A/B test results.")
            return []

        self.logger.info(f"Fetching latest {limit} A/B test results.")
        try:
            cursor = self.ab_test_collection.find({}).sort("test_timestamp", -1).limit(limit)
            results = await cursor.to_list(length=limit)
            self.logger.debug(f"Found {len(results)} latest A/B test results.")
            return results
        except Exception as e:
            self.logger.error(f"Error retrieving latest A/B test results: {e}", exc_info=True)
            return []

import asyncio

async def main(): # Example usage
    # This setup assumes MongoDB is running and accessible.
    # In a real application, MongoDBClient would be initialized and passed.
    # For this example, we'll mock it if direct instantiation is complex.
    
    class MockAsyncMongoCollection:
        async def find_one_and_update(self, *args, **kwargs): print("Mock find_one_and_update called"); return {"component_id": "test_id", "total_invocations": 1, "total_execution_time_ms": 100.0}
        async def update_one(self, *args, **kwargs): print("Mock update_one called"); return True
        async def find_one(self, *args, **kwargs): print("Mock find_one called"); return {"component_id": "test_id", "name": "Test"}
        async def find(self, *args, **kwargs): print("Mock find called"); return self # Mock cursor
        async def sort(self, *args, **kwargs): print("Mock sort called"); return self # Mock cursor
        async def limit(self, *args, **kwargs): print("Mock limit called"); return self # Mock cursor
        async def to_list(self, *args, **kwargs): print("Mock to_list called"); return []
        async def create_index(self, *args, **kwargs): print("Mock create_index called"); return True
        @property
        def name(self): return "mock_collection"


    class MockAsyncMongoClient:
        def __init__(self):
            self.client = motor.motor_asyncio.AsyncIOMotorClient() # Still needs a running mongo for this part to be truly async
        def get_collection(self, name):
            print(f"Mock get_collection called for {name}")
            return MockAsyncMongoCollection()

    # To run this example, you'd need a running MongoDB instance for Motor to connect to,
    # even if we mock the collection methods.
    # Or, fully mock motor.motor_asyncio.AsyncIOMotorClient if no DB is available.
    
    # For now, this main function is primarily for illustration and might not run out-of-the-box
    # without a MongoDB instance or more elaborate mocking of Motor.
    
    print("ComponentExperienceTracker example (conceptual).")
    # mongo_client = MongoDBClient(uri="mongodb://localhost:27017", db_name="evolving_agents_dev")
    # await mongo_client.connect() # Assuming connect method exists and is async
    
    # tracker = ComponentExperienceTracker(mongodb_client=MockAsyncMongoClient()) # Using mock
    
    # await tracker.record_event(
    #     component_id="agent_summarizer_v1",
    #     name="TextSummarizerAgent",
    #     record_type="AGENT",
    #     duration_ms=150.7,
    #     success=True
    # )
    # await tracker.record_event(
    #     component_id="tool_calculator_v2",
    #     name="AdvancedCalculator",
    #     record_type="TOOL",
    #     duration_ms=30.2,
    #     success=False,
    #     error="ValueError: Division by zero"
    # )
    # await tracker.add_evolution_event(
    #     component_id="agent_summarizer_v1",
    #     evolved_to_id="agent_summarizer_v2",
    #     strategy="fine_tuning",
    #     changes_summary="Improved handling of long texts."
    # )
    # experience = await tracker.get_experience("agent_summarizer_v1")
    # if experience:
    #     print(f"Experience for agent_summarizer_v1: {experience.get('total_invocations')} invocations.")
    
    # top_failed = await tracker.get_top_n_components_by_metric("failed_invocations", n=5)
    # print(f"Top failing components: {top_failed}")

    # await mongo_client.close() # Assuming close method exists

if __name__ == "__main__":
    # To run the example:
    # Ensure MongoDB is running locally if not using a fully mocked client.
    # You might need to adjust connection strings or mocking based on your setup.
    # asyncio.run(main())
    pass
