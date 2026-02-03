# evolving_agents/agent_bus/smart_agent_bus.py

import os
import json
import logging
import asyncio
import time
import uuid
from datetime import datetime, timezone
from typing import Optional, Dict, Any, List, Union

import pymongo
import motor.motor_asyncio

from evolving_agents.core.llm_service import LLMService
from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.core.dependency_container import DependencyContainer
from evolving_agents.core.base import IAgent
from evolving_agents.core.mongodb_client import MongoDBClient
from evolving_agents.monitoring.component_experience_tracker import ComponentExperienceTracker

logger = logging.getLogger(__name__)

DEFAULT_AGENT_EMBEDDING_DIM = 1536 # Example dimension, align with your embedding model

class SmartAgentBus:
    def __init__(
        self,
        smart_library: Optional[SmartLibrary] = None,
        system_agent: Optional[IAgent] = None,
        llm_service: Optional[LLMService] = None,
        container: Optional[DependencyContainer] = None,
        mongodb_client: Optional[MongoDBClient] = None,
        mongodb_uri: Optional[str] = None,
        mongodb_db_name: Optional[str] = None,
        registry_collection_name: str = "eat_agent_registry",
        logs_collection_name: str = "eat_agent_bus_logs",
        circuit_breaker_path: str = "agent_bus_circuit_breakers.json",
        component_experience_tracker: Optional[ComponentExperienceTracker] = None
    ):
        self.container = container
        self._system_agent_instance = system_agent
        self._initialized = False
        self._agent_instances: Dict[str, Any] = {}
        self.circuit_breakers: Dict[str, Dict[str, Any]] = {}
        self.circuit_breaker_path = circuit_breaker_path
        self.mongodb_client: Optional[MongoDBClient] = None
        self.registry_collection: Optional[motor.motor_asyncio.AsyncIOMotorCollection] = None
        self.logs_collection: Optional[motor.motor_asyncio.AsyncIOMotorCollection] = None


        # Resolve core dependencies: SmartLibrary and LLMService
        if container:
            self.smart_library = smart_library or container.get('smart_library', None)
            self.llm_service = llm_service or container.get('llm_service', None)
            if not self.llm_service:
                self.llm_service = LLMService(container=container)
                container.register('llm_service', self.llm_service)
        else:
            self.smart_library = smart_library
            self.llm_service = llm_service or LLMService()

        # Resolve MongoDBClient
        if mongodb_client is not None:
            self.mongodb_client = mongodb_client
            logger.debug("SmartAgentBus: Using directly passed MongoDBClient.")
        elif container and container.has('mongodb_client'):
            self.mongodb_client = container.get('mongodb_client')
            logger.debug("SmartAgentBus: Using MongoDBClient from container.")
        else:
            logger.warning("SmartAgentBus: MongoDBClient not passed directly or found in container. Attempting to create new instance.")
            try:
                self.mongodb_client = MongoDBClient(uri=mongodb_uri, db_name=mongodb_db_name)
                if container:
                    container.register('mongodb_client', self.mongodb_client)
            except ValueError as e:
                logger.error(f"SmartAgentBus: Failed to initialize default MongoDBClient: {e}. DB operations will fail.")
                self.mongodb_client = None # Explicitly set to None if creation fails

        if self.mongodb_client:
            if not isinstance(self.mongodb_client.client, motor.motor_asyncio.AsyncIOMotorClient):
                logger.critical("SmartAgentBus: MongoDBClient is NOT using an AsyncIOMotorClient (Motor). Async DB ops will fail.")
                self.registry_collection = None
                self.logs_collection = None
            else:
                self.registry_collection_name = registry_collection_name
                self.logs_collection_name = logs_collection_name
                self.registry_collection = self.mongodb_client.get_collection(self.registry_collection_name)
                self.logs_collection = self.mongodb_client.get_collection(self.logs_collection_name)
                asyncio.create_task(self._ensure_registry_indexes_and_load())
        else:
            logger.error("SmartAgentBus: MongoDBClient is not available. Database operations will not be possible.")
            self.registry_collection_name = "N/A (MongoDB unavailable)"
            self.logs_collection_name = "N/A (MongoDB unavailable)"
            self.registry_collection = None
            self.logs_collection = None

        # Initialize ComponentExperienceTracker
        self.experience_tracker = None
        if component_experience_tracker:
            self.experience_tracker = component_experience_tracker
            logger.debug("SmartAgentBus: Using directly passed ComponentExperienceTracker.")
        elif self.container and self.container.has('component_experience_tracker'):
            self.experience_tracker = self.container.get('component_experience_tracker')
            logger.debug("SmartAgentBus: Using ComponentExperienceTracker from container.")
        elif self.mongodb_client:
            try:
                self.experience_tracker = ComponentExperienceTracker(mongodb_client=self.mongodb_client)
                logger.info("SmartAgentBus: Initialized new ComponentExperienceTracker instance.")
                if self.container and not self.container.has('component_experience_tracker'):
                    self.container.register('component_experience_tracker', self.experience_tracker)
            except Exception as e:
                logger.error(f"SmartAgentBus: Failed to initialize ComponentExperienceTracker: {e}", exc_info=True)
        else:
            logger.warning("SmartAgentBus: ComponentExperienceTracker not provided and MongoDBClient unavailable for its auto-init.")

        self.agents: Dict[str, Dict[str, Any]] = {} # In-memory cache
        self._load_circuit_breakers()

        if container and not container.has('agent_bus'):
            container.register('agent_bus', self)

        logger.info(f"SmartAgentBus initialized. Registry: '{self.registry_collection_name}', Logs: '{self.logs_collection_name}'")

    @property
    def system_agent(self) -> Optional[IAgent]:
        if not self._system_agent_instance and self.container and self.container.has('system_agent'):
             self._system_agent_instance = self.container.get('system_agent')
        return self._system_agent_instance

    async def _ensure_registry_indexes_and_load(self):
        """Ensure MongoDB indexes for the agent registry and load data into memory."""
        if self.registry_collection is None: 
            logger.error(f"Cannot ensure indexes: registry_collection '{self.registry_collection_name}' is None.")
            return
        try:
            await self.registry_collection.create_index([("id", pymongo.ASCENDING)], unique=True, background=True)
            await self.registry_collection.create_index([("name", pymongo.ASCENDING)], background=True)
            await self.registry_collection.create_index([("status", pymongo.ASCENDING)], background=True)
            await self.registry_collection.create_index([("type", pymongo.ASCENDING)], background=True)
            await self.registry_collection.create_index([("capabilities.id", pymongo.ASCENDING)], background=True)
            await self.registry_collection.create_index([("description", pymongo.TEXT)], name="description_text_index", background=True)

            logger.info(f"Ensured standard indexes on '{self.registry_collection_name}'.")
            logger.info("Reminder: For vector search on agent descriptions (field 'description_embedding'), "
                        "ensure an Atlas Vector Search index (e.g., 'vector_index_agent_description') is configured manually on the 'description_embedding' field.")
        except Exception as e:
            logger.error(f"Error creating MongoDB indexes for {self.registry_collection_name}: {e}", exc_info=True)
        
        await self._load_registry_from_db()


    async def _load_registry_from_db(self) -> None:
        """Load all agent records from MongoDB into the in-memory self.agents dictionary."""
        if self.registry_collection is None: 
            logger.error("Registry collection is not initialized. Cannot load from DB.")
            return

        logger.info(f"Loading agent registry from MongoDB collection '{self.registry_collection_name}'...")
        self.agents = {}
        try:
            projection = {"_id": 0, "description_embedding": 0, "capabilities.description_embedding": 0}
            cursor = self.registry_collection.find({}, projection)
            async for agent_doc in cursor:
                self.agents[agent_doc["id"]] = agent_doc
            logger.info(f"Loaded {len(self.agents)} agents from MongoDB into in-memory cache.")
        except Exception as e:
            logger.error(f"Error loading agent registry from MongoDB: {e}", exc_info=True)


    def _load_circuit_breakers(self) -> None:
        if os.path.exists(self.circuit_breaker_path):
            try:
                with open(self.circuit_breaker_path, 'r') as f:
                    self.circuit_breakers = json.load(f)
                logger.info(f"Loaded {len(self.circuit_breakers)} circuit breaker states from {self.circuit_breaker_path}")
            except Exception as e:
                logger.error(f"Error loading circuit breakers: {e}")
                self.circuit_breakers = {}
        else:
            logger.info(f"Circuit breaker file {self.circuit_breaker_path} not found. Starting with empty states.")
            self.circuit_breakers = {}


    async def _save_circuit_breakers(self) -> None:
        try:
            with open(self.circuit_breaker_path, 'w') as f:
                json.dump(self.circuit_breakers, f, indent=2)
        except Exception as e:
            logger.error(f"Error saving circuit breakers: {e}")


    async def initialize_from_library(self) -> None:
        if self._initialized:
            logger.debug("AgentBus already initialized from library.")
            return
        if not self.smart_library:
            logger.warning("SmartLibrary not provided; cannot initialize AgentBus from library.")
            return

        logger.info("AgentBus initializing from SmartLibrary...")
        try:
            agent_records = await self.smart_library.export_records(record_type="AGENT")
            tool_records = await self.smart_library.export_records(record_type="TOOL")
        except ConnectionError as e: 
            logger.error(f"Failed to export records from SmartLibrary due to DB connection issue: {e}")
            return

        library_records = agent_records + tool_records
        initialized_count = 0

        for record in library_records:
            # We still only want to initialize *from* active records in the library.
            # If a record was active and is now archived in the library, 
            # `update_agent_from_library` should be used for that specific agent.
            # `initialize_from_library` is more for a bulk sync of active, usable components.
            if record.get("status") != "active":
                logger.debug(f"Skipping record {record['id']} from SmartLibrary during init as it's not 'active' (status: {record.get('status')}).")
                continue

            # If agent already in bus, re-register to update its definition from library
            # This ensures the bus reflects the latest "active" version from the library.
            # `register_agent` will handle the upsert logic.
            if record["id"] in self.agents:
                logger.info(f"Agent {record['id']} already in bus registry. Re-registering to sync with SmartLibrary.")
            
            capabilities = record.get("metadata", {}).get("capabilities", [])
            if not capabilities and record.get("name"):
                cap_id = f"{record['name'].lower().replace(' ', '_')}_default_capability"
                capabilities = [{"id": cap_id, "name": record["name"], "description": record.get("description", f"Default capability for {record['name']}"), "confidence": 0.7}]

            agent_instance = None
            if self.container:
                agent_factory = self.container.get('agent_factory', None)
                tool_factory = self.container.get('tool_factory', None)
                try:
                    if record["record_type"] == "AGENT" and agent_factory:
                        agent_instance = await agent_factory.create_agent(record)
                    elif record["record_type"] == "TOOL" and tool_factory:
                        agent_instance = await tool_factory.create_tool(record)
                except Exception as e:
                    logger.error(f"Instance creation failed for SmartLibrary component {record.get('name')} ({record.get('id')}): {e}", exc_info=True)
            
            try:
                await self.register_agent(
                    agent_id=record["id"], name=record["name"], description=record.get("description", ""),
                    capabilities=capabilities, agent_type=record.get("record_type", "GENERIC"),
                    metadata={"source": "SmartLibrary", **record.get("metadata", {})},
                    agent_instance=agent_instance,
                    embed_capabilities=True # Assuming we want fresh embeddings on library sync
                )
                initialized_count += 1
            except RuntimeError as reg_err: # Catch specific error from register_agent
                 logger.error(f"Failed to register/update agent {record['name']} during library sync: {reg_err}")
            except Exception as e: # Catch other potential errors during processing
                 logger.error(f"Unexpected error processing record {record['name']} ({record['id']}) during library sync: {e}", exc_info=True)


        self._initialized = True # Mark as initialized even if some errors occurred.
        logger.info(f"AgentBus initialization/sync from SmartLibrary processed. Updated/Registered {initialized_count} active components.")


    async def update_agent_from_library(self, agent_id: str) -> bool:
        """
        Updates a single agent's definition and status in the AgentBus
        based on its state in the SmartLibrary.
        """
        if not self.smart_library:
            logger.error(f"SmartLibrary not available. Cannot update agent {agent_id}.")
            return False
        if self.registry_collection is None:
            logger.error(f"AgentBus registry collection not available. Cannot update agent {agent_id}.")
            return False

        logger.info(f"Attempting to update agent {agent_id} in AgentBus from SmartLibrary.")
        
        try:
            library_record = await self.smart_library.find_record_by_id(agent_id)
        except Exception as e:
            logger.error(f"Error fetching record {agent_id} from SmartLibrary: {e}", exc_info=True)
            return False

        if not library_record:
            logger.warning(f"Agent {agent_id} not found in SmartLibrary. Considering removal from AgentBus.")
            # Future: Implement removal logic if desired. For now, just log.
            # Example removal:
            # if agent_id in self.agents: del self.agents[agent_id]
            # if agent_id in self._agent_instances: del self._agent_instances[agent_id]
            # await self.registry_collection.delete_one({"id": agent_id})
            # logger.info(f"Agent {agent_id} removed from AgentBus as it's no longer in SmartLibrary.")
            return False # Indicate agent not found in library

        library_status = library_record.get("status")
        agent_name = library_record.get("name", agent_id)

        if library_status == "active" or library_status == "deployed": # Assuming "deployed" is also an active state
            logger.info(f"Agent {agent_id} ({agent_name}) is '{library_status}' in SmartLibrary. Registering/updating in AgentBus.")
            try:
                # Prepare capabilities as register_agent expects them
                capabilities = library_record.get("metadata", {}).get("capabilities", [])
                if not capabilities and library_record.get("name"): # Fallback for default capability
                     cap_id = f"{library_record['name'].lower().replace(' ', '_')}_default_capability"
                     capabilities = [{"id": cap_id, "name": library_record["name"], "description": library_record.get("description", f"Default capability for {library_record['name']}"), "confidence": 0.7}]

                # Attempt to create instance if possible (e.g., if factories are available)
                agent_instance = None
                if self.container:
                    agent_factory = self.container.get('agent_factory', None)
                    tool_factory = self.container.get('tool_factory', None)
                    try:
                        if library_record.get("record_type") == "AGENT" and agent_factory:
                            agent_instance = await agent_factory.create_agent(library_record)
                        elif library_record.get("record_type") == "TOOL" and tool_factory:
                            agent_instance = await tool_factory.create_tool(library_record)
                    except Exception as e_inst:
                        logger.warning(f"Instance creation failed for {agent_id} during update_agent_from_library: {e_inst}", exc_info=False)


                await self.register_agent(
                    agent_id=library_record["id"],
                    name=library_record["name"],
                    description=library_record.get("description", ""),
                    capabilities=capabilities,
                    agent_type=library_record.get("record_type", "GENERIC"),
                    metadata={"source": "SmartLibrary_update", **library_record.get("metadata", {})},
                    agent_instance=agent_instance, # Pass instance if created
                    embed_capabilities=True # Re-embed on update
                )
                logger.info(f"Successfully registered/updated agent {agent_id} ({agent_name}) in AgentBus from SmartLibrary.")
                return True
            except Exception as e_reg:
                logger.error(f"Failed to register/update agent {agent_id} ({agent_name}) in AgentBus: {e_reg}", exc_info=True)
                return False
        
        elif library_status == "archived" or library_status == "inactive":
            logger.info(f"Agent {agent_id} ({agent_name}) is '{library_status}' in SmartLibrary. Updating status in AgentBus and removing instance.")
            
            # Update status in MongoDB registry
            update_result = await self.registry_collection.update_one(
                {"id": agent_id},
                {"$set": {"status": library_status, "last_updated": datetime.now(timezone.utc)}}
            )
            
            # Update status in in-memory cache
            if agent_id in self.agents:
                self.agents[agent_id]["status"] = library_status
                self.agents[agent_id]["last_updated"] = datetime.now(timezone.utc).isoformat()
            
            # Remove live instance if it exists
            if agent_id in self._agent_instances:
                del self._agent_instances[agent_id]
                logger.debug(f"Removed live instance of archived agent {agent_id} ({agent_name}) from AgentBus cache.")
            
            if update_result.modified_count > 0 or update_result.matched_count > 0 : # matched_count for case where status was already set
                logger.info(f"Successfully updated status of agent {agent_id} ({agent_name}) to '{library_status}' in AgentBus registry.")
                return True
            else:
                logger.warning(f"Agent {agent_id} ({agent_name}) was not found in AgentBus registry for status update to '{library_status}', but was '{library_status}' in library.")
                # If it wasn't in the bus registry, but is archived in library, it's effectively not active.
                return True # Consider this a success in terms of reflecting library state.
        else:
            logger.warning(f"Agent {agent_id} ({agent_name}) has an unhandled status '{library_status}' in SmartLibrary. No action taken in AgentBus.")
            return False

    # Log Schema for 'eat_agent_bus_logs' collection:
    # Each log entry is a dictionary with the following fields:
    # - log_id (str): Unique identifier for this log record (e.g., "log_xxxxxxxxxxxx").
    # - timestamp (datetime): Timestamp of when the log was created (UTC).
    # - bus_type (str): Indicates the type of bus operation, e.g., 'data' for capability requests,
    #                   'system' for direct agent executions or internal bus operations like 'register'.
    # - agent_id (str): The ID of the agent or tool that was invoked. Can be 'N/A' if not applicable.
    # - agent_name (str): The name of the invoked agent or tool. Can be 'N/A'.
    # - task_description (str): A high-level description of the task, capability, or operation performed.
    #                           (e.g., "summarize_text", "execute_code_tool", "register_agent").
    # - task_details (dict): Contains the detailed parameters and inputs provided for the task.
    #                        FOR THIS LOG TO BE EFFECTIVELY QUERYABLE AND ANALYZABLE BY STRATEGIC
    #                        AGENTS (LIKE EVOLUTIONSTRATEGISTAGENT), THIS FIELD SHOULD COMPREHENSIVELY
    #                        CAPTURE ALL RELEVANT INPUTS, ARGUMENTS, AND CONFIGURATIONS USED FOR
    #                        THE INVOCATION. This field is directly populated from the `task` argument
    #                        passed to this logging method. For example, if a capability takes 'text' and 'style'
    #                        as inputs, task_details should be like: {'text': '...', 'style': 'formal'}.
    # - result (dict, optional): The result returned by the agent/tool. May be summarized for brevity.
    #                            Example: {'content_summary': 'Summary of the text...'}
    # - error (str, optional): If an error occurred during execution, this field contains the error
    #                          message or a summary of the error.
    # - duration_ms (int, optional): The duration of the agent/tool execution in milliseconds.
    async def _log_agent_execution(
        self, bus_type: str, agent_id: str, task: Dict[str, Any],
        result: Optional[Dict[str, Any]] = None, error: Optional[str] = None,
        duration: Optional[float] = None
    ) -> None:
        if self.logs_collection is None: 
            logger.warning("Logs collection not available. Skipping log.")
            return
            
        agent_name = self.agents.get(agent_id, {}).get("name", "N/A") if agent_id != "N/A" else "N/A"

        log_entry = {
            "log_id": f"log_{uuid.uuid4().hex[:12]}",
            "timestamp": datetime.now(timezone.utc),
            "bus_type": bus_type, "agent_id": agent_id, "agent_name": agent_name,
            "task_description": task.get("capability") or task.get("operation", "direct_execution"),
            "task_details": task, 
            "result": result,   
            "error": error,
            "duration_ms": int(duration * 1000) if duration is not None else None,
        }
        try:
            for key in ["task_details", "result"]:
                if isinstance(log_entry[key], dict):
                    log_entry[key] = json.loads(json.dumps(log_entry[key], default=str)) 
            await self.logs_collection.insert_one(log_entry)
        except Exception as e:
            logger.error(f"Failed to write execution log to MongoDB: {e}", exc_info=True)

        if self.experience_tracker:
            try:
                # Prepare parameters for record_event
                # 'task' dict is available in this method's scope as 'task'
                # 'result' is available as 'result'
                # 'error' is available as 'error'
                # 'duration' is available as 'duration' (in seconds)
                # 'agent_id' is available as 'agent_id'
                # 'agent_name' is available (already resolved in log_entry preparation)

                # Extract input_params from the 'task' dictionary.
                input_params_for_experience = task.get("content", task) if isinstance(task.get("content"), dict) else task

                # Determine record_type (AGENT/TOOL)
                component_type = "UNKNOWN"
                if agent_id != "N/A" and agent_id in self.agents:
                    component_type = self.agents[agent_id].get("type", "UNKNOWN")
                
                output_summary_for_experience = None
                if result and isinstance(result, dict):
                    output_summary_for_experience = str(result.get("content_summary", result))[:500] # Summarize
                elif result:
                    output_summary_for_experience = str(result)[:500]


                await self.experience_tracker.record_event(
                    component_id=agent_id,
                    name=agent_name, # agent_name resolved for log_entry
                    record_type=component_type,
                    duration_ms=duration * 1000 if duration is not None else 0.0,
                    success=error is None,
                    error=error,
                    input_params=input_params_for_experience,
                    output_summary=output_summary_for_experience
                )
            except Exception as e:
                logger.error(f"SmartAgentBus: Failed to record event with ComponentExperienceTracker for agent {agent_id}: {e}", exc_info=True)

    def _check_circuit_breaker(self, agent_id: str) -> bool:
        if agent_id not in self.circuit_breakers: return False
        cb = self.circuit_breakers[agent_id]
        failures = cb.get("failures", 0)
        last_failure_time = cb.get("last_failure", 0.0)
        
        if failures >= 3 and (time.time() - last_failure_time) < 300: 
            logger.warning(f"Circuit breaker tripped for agent {agent_id}. Failures: {failures}, Last failure: {datetime.fromtimestamp(last_failure_time).isoformat()}")
            return True
        return False

    async def _record_failure(self, agent_id: str) -> None:
        if agent_id not in self.circuit_breakers:
            self.circuit_breakers[agent_id] = {"failures": 0, "last_failure": 0.0}
        
        self.circuit_breakers[agent_id]["failures"] = self.circuit_breakers[agent_id].get("failures", 0) + 1
        self.circuit_breakers[agent_id]["last_failure"] = time.time()
        
        await self._save_circuit_breakers()
        logger.warning(f"Failure recorded for agent {agent_id}. Total failures: {self.circuit_breakers[agent_id]['failures']}. Last failure: {datetime.fromtimestamp(self.circuit_breakers[agent_id]['last_failure']).isoformat()}")


    async def _reset_circuit_breaker(self, agent_id: str) -> None:
        if agent_id in self.circuit_breakers:
            logger.info(f"Resetting circuit breaker for agent {agent_id}")
            del self.circuit_breakers[agent_id]
            await self._save_circuit_breakers()

    async def register_agent(
        self, name: str, description: str, capabilities: List[Dict[str, Any]],
        agent_type: str = "GENERIC", metadata: Optional[Dict[str, Any]] = None,
        agent_instance=None, agent_id: Optional[str] = None,
        embed_capabilities: bool = False 
    ) -> str:
        if self.registry_collection is None: 
            logger.error("Registry collection not available. Cannot register agent.")
            raise RuntimeError("Agent registry (MongoDB) is not available.")

        if not self.llm_service:
            logger.error("LLMService not available. Cannot generate embeddings for agent registration.")
            raise RuntimeError("LLMService is required for agent registration if embeddings are needed.")

        if not agent_id:
            agent_id = f"agent_{uuid.uuid4().hex[:8]}"
        
        cap_names_str = ", ".join(c.get("name", c.get("id", "")) for c in capabilities)
        agent_desc_for_embedding = f"Agent Name: {name}\nType: {agent_type}\nDescription: {description}\nCapabilities Offered: {cap_names_str}"
        
        description_embedding = []
        try:
            description_embedding = await self.llm_service.embed(agent_desc_for_embedding)
        except Exception as e:
            logger.error(f"Embedding generation failed for agent '{name}' description: {e}", exc_info=True)
            description_embedding = [0.0] * DEFAULT_AGENT_EMBEDDING_DIM 

        processed_capabilities = []
        for cap_data in capabilities:
            cap_id = cap_data.get("id", f"{cap_data.get('name', 'capability').lower().replace(' ', '_')}_{uuid.uuid4().hex[:4]}")
            cap = {
                "id": cap_id,
                "name": cap_data.get("name", cap_id),
                "description": cap_data.get("description", f"Default description for {cap_id}"),
                "confidence": cap_data.get("confidence", 0.8) 
            }
            if embed_capabilities and cap["description"]:
                try:
                    cap["description_embedding"] = await self.llm_service.embed(cap["description"])
                except Exception as e:
                    logger.error(f"Embedding generation failed for capability '{cap['name']}': {e}", exc_info=True)
                    cap["description_embedding"] = [0.0] * DEFAULT_AGENT_EMBEDDING_DIM 
            processed_capabilities.append(cap)
        
        current_time = datetime.now(timezone.utc)
        existing_doc = await self.registry_collection.find_one({"id": agent_id}, {"registered_at": 1})
        registered_at = existing_doc.get("registered_at", current_time) if existing_doc else current_time

        agent_doc = {
            "id": agent_id, "name": name, "description": description, "type": agent_type,
            "capabilities": processed_capabilities, 
            "metadata": metadata or {}, "status": "active",
            "description_embedding": description_embedding, 
            "registered_at": registered_at, 
            "last_updated": current_time,   
            "last_seen": current_time       
        }
        try:
            result = await self.registry_collection.replace_one({"id": agent_id}, agent_doc, upsert=True)
            op_type = "Registered new" if result.upserted_id else "Updated" if result.modified_count > 0 else "Refreshed (no change)"
            
            cached_doc = agent_doc.copy()
            cached_doc.pop("description_embedding", None)
            for cap_item in cached_doc.get("capabilities", []):
                cap_item.pop("description_embedding", None)
            self.agents[agent_id] = cached_doc

            if agent_instance:
                self._agent_instances[agent_id] = agent_instance 
            
            await self._log_agent_execution('system', agent_id, {'operation': 'register', 'details': {'name': name, 'type': agent_type, 'capabilities_count': len(capabilities)}}, {'status': 'success', 'type': op_type})
            logger.info(f"{op_type} agent: {name} ({agent_id}) in MongoDB registry.")
            return agent_id
        except Exception as e:
            logger.error(f"MongoDB registration error for agent {name}: {e}", exc_info=True)
            raise 

    async def discover_agents(
        self, task_description: Optional[str] = None, capability_id: Optional[str] = None,
        min_confidence: float = 0.6, agent_type: Optional[str] = None, limit: int = 5
    ) -> List[Dict[str, Any]]:
        if self.registry_collection is None:
            logger.error("Registry collection unavailable. Cannot discover agents.")
            return []
        if not task_description and not capability_id:
             raise ValueError("Either task_description or capability_id must be provided for agent discovery.")

        matches: List[Dict[str, Any]] = []
        # Projection: Exclude embedding fields unless needed by caller (they are large)
        # For discovery, we typically don't need the full embeddings in the response.
        projection = {"_id": 0, "description_embedding": 0} 
        # If capabilities also have embeddings and they are not needed:
        # projection["capabilities.description_embedding"] = 0 

        if capability_id:
            logger.debug(f"Discovering agents by capability_id '{capability_id}' via metadata query.")
            query_filter: Dict[str, Any] = {"status": "active", "capabilities.id": capability_id}
            if agent_type: query_filter["type"] = agent_type
            
            cursor = self.registry_collection.find(query_filter, projection).limit(limit * 2) # Fetch more to filter by health
            async for agent_doc in cursor:
                cap_conf = 0.0
                for cap_item in agent_doc.get("capabilities", []): 
                    if cap_item.get("id") == capability_id:
                        cap_conf = cap_item.get("confidence", 0.0)
                        break
                if cap_conf >= min_confidence:
                    is_healthy = not self._check_circuit_breaker(agent_doc["id"])
                    if is_healthy:
                        matches.append({**agent_doc, "similarity_score": cap_conf, "is_healthy": True})
            matches.sort(key=lambda x: x.get("similarity_score", 0.0), reverse=True)

        elif task_description:
            if not self.llm_service:
                logger.error("LLMService not available for embedding task description. Cannot perform vector search for agents.")
                return []

            logger.debug(f"Discovering agents by task_description '{task_description[:50]}...' using vector search on 'description_embedding'.")
            try:
                query_embedding = await self.llm_service.embed(task_description)
                
                # Atlas Vector Search index name for agent descriptions
                # Ensure this index exists on the 'description_embedding' field in the 'eat_agent_registry' collection.
                atlas_vs_index_name = "vector_index_agent_description" 

                vs_stage_definition: Dict[str, Any] = {
                    "index": atlas_vs_index_name,
                    "path": "description_embedding",
                    "queryVector": query_embedding,
                    "numCandidates": limit * 15, # Number of candidates for the KNN search
                    "limit": limit * 3           # Number of documents to return from the $vectorSearch stage
                }

                # MQL filters to be applied by $vectorSearch
                mql_filter_conditions = []
                if agent_type:
                    mql_filter_conditions.append({"type": agent_type})
                mql_filter_conditions.append({"status": "active"})
                
                if mql_filter_conditions:
                    vs_stage_definition["filter"] = {"$and": mql_filter_conditions} if len(mql_filter_conditions) > 1 else mql_filter_conditions[0]

                pipeline = [
                    {"$vectorSearch": vs_stage_definition},
                    # Project the document fields and the search score
                    {"$project": {**projection, "similarity_score": {"$meta": "vectorSearchScore"}}}
                    # Sorting by similarity_score is implicitly handled by $vectorSearch
                    # The final limit is applied after health checks
                ]
                
                logger.debug(f"SmartAgentBus discover_agents ($vectorSearch) pipeline: {json.dumps(pipeline)}")
                cursor = self.registry_collection.aggregate(pipeline)
                
                async for agent_doc in cursor:
                    # $vectorSearch score is typically 0-1, higher is better.
                    # min_confidence can be directly compared.
                    if agent_doc.get("similarity_score", 0.0) >= min_confidence:
                        is_healthy = not self._check_circuit_breaker(agent_doc["id"])
                        if is_healthy:
                            matches.append({**agent_doc, "is_healthy": True}) 
                
                # If $vectorSearch's limit was higher, sort again and limit here if needed,
                # but it's generally better to let $vectorSearch do the primary limiting.
                # matches.sort(key=lambda x: x.get("similarity_score", 0.0), reverse=True) # Already sorted
            
            except pymongo.errors.OperationFailure as e:
                logger.error(f"Vector search for agents failed: {e}. No fallback implemented for this path.", exc_info=True)
                if "index not found" in str(e).lower() or "Unknown $vectorSearch index" in str(e).lower() or "unknown search index" in str(e).lower() or "Invalid $vectorSearch" in str(e).lower():
                    logger.error(f"CRITICAL: Atlas Vector Search index '{atlas_vs_index_name}' likely missing or misconfigured on collection '{self.registry_collection_name}'. Query: {vs_stage_definition}")
            except Exception as e:
                logger.error(f"Unexpected error during agent discovery with vector search: {e}", exc_info=True)

        return matches[:limit] # Apply final limit


    async def request_capability(
        self, capability: str, content: Dict[str, Any], min_confidence: float = 0.6,
        specific_agent_id: Optional[str] = None, timeout: float = 60.0
    ) -> Dict[str, Any]:
        if self.registry_collection is None:
            logger.error("Registry collection unavailable. Cannot request capability.")
            raise RuntimeError("Agent registry (MongoDB) is not available for capability request.")

        logger.info(f"Received data bus request for capability '{capability}'")
        start_time = time.time()
        agent_to_use_doc = None
        agent_id_to_use = None
        task_details = {"capability": capability, "content_summary": str(content)[:200]} 

        if specific_agent_id:
            agent_to_use_doc = self.agents.get(specific_agent_id) 
            if not (agent_to_use_doc and agent_to_use_doc.get("status") == "active"):
                db_doc = await self.registry_collection.find_one(
                    {"id": specific_agent_id, "status":"active"},
                    {"_id": 0, "description_embedding": 0} 
                )
                if db_doc:
                    agent_to_use_doc = db_doc
                    self.agents[specific_agent_id] = db_doc 
                else:
                    error_msg = f"Targeted agent {specific_agent_id} not found or not active in registry."
                    await self._log_agent_execution('data', specific_agent_id or 'N/A', task_details, error=error_msg, duration=time.time()-start_time)
                    raise ValueError(error_msg)
            agent_id_to_use = specific_agent_id
            logger.debug(f"Targeting specific agent: {agent_id_to_use} ({agent_to_use_doc.get('name')})")
        else:
            suitable_agents_info = await self.discover_agents(task_description=capability, capability_id=capability, min_confidence=min_confidence) # Pass both for flexibility
            if not suitable_agents_info:
                error_msg = f"No healthy agent found for capability '{capability}' with confidence >= {min_confidence}."
                await self._log_agent_execution('data', 'N/A', task_details, error=error_msg, duration=time.time()-start_time)
                raise ValueError(error_msg)
            
            best_agent_info = suitable_agents_info[0]
            agent_id_to_use = best_agent_info["id"]
            agent_to_use_doc = best_agent_info
            logger.debug(f"Discovered agent {agent_id_to_use} ({agent_to_use_doc.get('name')}) for capability '{capability}' with score {best_agent_info.get('similarity_score', 0.0):.2f}.")


        if not agent_to_use_doc: 
            critical_error_msg = f"Internal error: Agent ID '{agent_id_to_use}' selected but document could not be retrieved/found."
            await self._log_agent_execution('data', agent_id_to_use or 'N/A', task_details, error=critical_error_msg, duration=time.time()-start_time)
            raise RuntimeError(critical_error_msg)

        if self._check_circuit_breaker(agent_id_to_use):
             error_msg = f"Agent {agent_id_to_use} ({agent_to_use_doc.get('name')}) is temporarily unavailable (circuit breaker tripped)."
             await self._log_agent_execution('data', agent_id_to_use, task_details, error=error_msg, duration=time.time()-start_time)
             raise RuntimeError(error_msg)

        try:
            result_data = await asyncio.wait_for(
                self._execute_agent_task(agent_to_use_doc, task=content), timeout=timeout
            )
            await self._reset_circuit_breaker(agent_id_to_use) 
            
            current_time = datetime.now(timezone.utc)
            await self.registry_collection.update_one({"id": agent_id_to_use}, {"$set": {"last_seen": current_time}})
            if agent_id_to_use in self.agents:
                self.agents[agent_id_to_use]["last_seen"] = current_time.isoformat()

            duration = time.time() - start_time
            response = {
                "status": "success",
                "agent_id": agent_id_to_use,
                "agent_name": agent_to_use_doc.get("name"),
                "capability_executed": capability,
                "content": result_data, 
                "metadata": {"processing_time_ms": int(duration * 1000)}
            }
            await self._log_agent_execution('data', agent_id_to_use, task_details, result={"content_summary": str(result_data)[:200]}, duration=duration)
            logger.info(f"Capability '{capability}' executed by {agent_id_to_use} successfully in {duration:.2f}s.")
            return response
        except asyncio.TimeoutError:
            await self._record_failure(agent_id_to_use)
            error_msg = f"Agent {agent_id_to_use} ({agent_to_use_doc.get('name')}) timed out for capability '{capability}' after {timeout}s."
            await self._log_agent_execution('data', agent_id_to_use, task_details, error=error_msg, duration=timeout)
            raise RuntimeError(error_msg)
        except Exception as e:
            await self._record_failure(agent_id_to_use)
            error_msg = f"Agent {agent_id_to_use} ({agent_to_use_doc.get('name')}) failed capability '{capability}': {str(e)}"
            await self._log_agent_execution('data', agent_id_to_use, task_details, error=error_msg, duration=time.time()-start_time)
            logger.error(f"Capability execution error for agent {agent_id_to_use} on capability '{capability}': {e}", exc_info=True)
            raise RuntimeError(error_msg) 


    async def execute_with_agent(
        self, agent_id: str, task: Dict[str, Any], timeout: float = 30.0
    ) -> Dict[str, Any]:
        if self.registry_collection is None:
            logger.error("Registry collection unavailable. Cannot execute agent.")
            raise RuntimeError("Agent registry (MongoDB) is not available for agent execution.")

        logger.info(f"Received direct execution request for agent {agent_id}")
        start_time = time.time()
        task_summary_for_log = {"task_summary": str(task)[:200]}
        
        agent_doc = self.agents.get(agent_id) 
        if not agent_doc:
            agent_doc_db = await self.registry_collection.find_one({"id": agent_id}, {"_id":0, "description_embedding":0})
            if agent_doc_db:
                self.agents[agent_id] = agent_doc_db 
                agent_doc = agent_doc_db
        
        if not agent_doc:
            raise ValueError(f"Agent with ID '{agent_id}' not found in registry.")
        if agent_doc.get("status") != "active":
            raise ValueError(f"Agent {agent_id} ({agent_doc.get('name')}) is not active.")

        if self._check_circuit_breaker(agent_id):
            error_msg = f"Agent {agent_id} ({agent_doc.get('name')}) temporarily unavailable (circuit breaker)."
            await self._log_agent_execution('system', agent_id, task_summary_for_log, error=error_msg, duration=time.time()-start_time)
            raise RuntimeError(error_msg)
        
        try:
            result_data = await asyncio.wait_for(self._execute_agent_task(agent_doc, task), timeout=timeout)
            await self._reset_circuit_breaker(agent_id) 
            
            current_time = datetime.now(timezone.utc)
            await self.registry_collection.update_one({"id": agent_id}, {"$set": {"last_seen": current_time}})
            if agent_id in self.agents:
                self.agents[agent_id]["last_seen"] = current_time.isoformat()
            
            duration = time.time() - start_time
            response = {
                "result": result_data,
                "metadata": {
                    "agent_id": agent_id,
                    "agent_name": agent_doc.get("name"),
                    "processing_time_ms": int(duration*1000),
                    "success": True
                }
            }
            await self._log_agent_execution('system', agent_id, task_summary_for_log, result={"result_summary": str(result_data)[:200]}, duration=duration)
            logger.info(f"Direct execution for agent {agent_id} completed successfully in {duration:.2f}s.")
            return response
        except asyncio.TimeoutError:
            await self._record_failure(agent_id)
            error_msg = f"Agent {agent_id} ({agent_doc.get('name')}) timed out (direct execution) after {timeout}s."
            await self._log_agent_execution('system', agent_id, task_summary_for_log, error=error_msg, duration=timeout)
            raise RuntimeError(error_msg)
        except Exception as e:
            await self._record_failure(agent_id)
            error_msg = f"Agent {agent_id} ({agent_doc.get('name')}) failed (direct execution): {str(e)}"
            await self._log_agent_execution('system', agent_id, task_summary_for_log, error=error_msg, duration=time.time()-start_time)
            logger.error(f"Direct agent execution error for agent {agent_id}: {e}", exc_info=True)
            raise RuntimeError(error_msg)


    async def _execute_agent_task(self, agent_record_doc: Dict[str, Any], task: Dict[str, Any]) -> Any:
        """Internal helper to execute a task on an agent instance."""
        agent_id = agent_record_doc["id"]
        agent_instance = self._agent_instances.get(agent_id)

        if not agent_instance:
            logger.warning(f"Live instance for agent {agent_id} ({agent_record_doc.get('name')}) not found in cache, attempting dynamic creation...")
            if self.container:
                agent_factory = self.container.get('agent_factory', None)
                tool_factory = self.container.get('tool_factory', None) 
                created = False
                
                # Use record_type from the agent_record_doc, not metadata.record_type
                record_type = agent_record_doc.get("type", "AGENT") # Default to AGENT if "type" (registry) or "record_type" (library) is missing
                if "record_type" in agent_record_doc: # Prefer "record_type" if from library format
                    record_type = agent_record_doc["record_type"]


                if agent_factory and record_type == "AGENT": 
                    try:
                        agent_instance = await agent_factory.create_agent(agent_record_doc)
                        created = True
                    except Exception as e:
                        logger.error(f"Dynamic AGENT creation failed for {agent_id}: {e}", exc_info=True)
                        raise RuntimeError(f"Failed to dynamically create AGENT instance for {agent_id}: {e}") from e
                elif tool_factory and record_type == "TOOL": 
                    try:
                        agent_instance = await tool_factory.create_tool(agent_record_doc) 
                        created = True
                    except Exception as e:
                        logger.error(f"Dynamic TOOL creation failed for {agent_id}: {e}", exc_info=True)
                        raise RuntimeError(f"Failed to dynamically create TOOL instance for {agent_id}: {e}") from e
                
                if created and agent_instance:
                    self._agent_instances[agent_id] = agent_instance
                    logger.info(f"Dynamically created and cached live instance for {agent_id} (type: {record_type})")
                elif not agent_factory and not tool_factory:
                     raise RuntimeError(f"No agent_factory or tool_factory available in container to create instance for {agent_id}")
                elif (record_type == "AGENT" and not agent_factory) or \
                     (record_type == "TOOL" and not tool_factory):
                     raise RuntimeError(f"No suitable factory found for component {agent_id} of type {record_type}")

            else: 
                raise RuntimeError(f"No live instance for agent {agent_id} and no DependencyContainer available for dynamic instantiation.")

        if hasattr(agent_instance, 'run') and callable(agent_instance.run):
            try:
                input_for_run = task.get("text", task) if isinstance(task, dict) else task
                if not isinstance(input_for_run, (str, dict)): 
                    input_for_run = str(input_for_run) 

                logger.debug(f"Executing .run() on instance for agent {agent_id} with input type {type(input_for_run)}")
                run_result = await agent_instance.run(input_for_run) 
                
                if hasattr(run_result, 'result') and hasattr(run_result.result, 'text'):
                    return run_result.result.text
                elif hasattr(run_result, 'text'):
                    return run_result.text
                elif isinstance(run_result, str):
                    return run_result
                try: return json.loads(json.dumps(run_result, default=str))
                except: return str(run_result)
            except Exception as e:
                logger.error(f"Execution via .run() method failed for agent {agent_id}: {e}", exc_info=True)
                raise RuntimeError(f"Agent's .run() method failed: {e}") from e

        elif agent_record_doc.get("metadata", {}).get("framework") == "openai-agents":
            if self.container and self.container.has('agent_factory'):
                agent_factory = self.container.get('agent_factory')
                # Ensure provider_registry is accessible from agent_factory
                if not hasattr(agent_factory, 'provider_registry'):
                    raise RuntimeError("AgentFactory does not have a provider_registry attribute.")

                provider = agent_factory.provider_registry.get_provider_for_framework("openai-agents")
                if provider:
                    try:
                        input_text = task.get("text", json.dumps(task) if isinstance(task, dict) else str(task))
                        logger.debug(f"Executing OpenAI agent {agent_id} via provider with input_text.")
                        exec_result = await provider.execute_agent(agent_instance, input_text) 
                        if exec_result.get("status") == "success":
                            return exec_result.get("result")
                        else:
                            raise RuntimeError(f"OpenAI Agent execution via provider failed: {exec_result.get('message', 'No message')}")
                    except Exception as e:
                        logger.error(f"Execution of OpenAI agent {agent_id} via provider failed: {e}", exc_info=True)
                        raise RuntimeError(f"OpenAI agent execution failed: {e}") from e
                else:
                    raise RuntimeError(f"OpenAI Agents provider not found in registry for agent {agent_id}.")
            else:
                raise RuntimeError(f"AgentFactory not available in container for executing OpenAI agent {agent_id}.")
        
        raise RuntimeError(f"No recognized execution method for agent {agent_id} (type: {type(agent_instance)}, framework: {agent_record_doc.get('metadata', {}).get('framework')})")


    async def get_agent_status(self, agent_id: str) -> Dict[str, Any]:
        if self.registry_collection is None:
            logger.error("Registry collection unavailable. Cannot get agent status.")
            raise RuntimeError("Agent registry (MongoDB) is not available for getting agent status.")

        agent_data = self.agents.get(agent_id) 
        if not agent_data:
            agent_data_db = await self.registry_collection.find_one({"id": agent_id}, {"_id": 0, "description_embedding": 0})
            if not agent_data_db:
                raise ValueError(f"Agent with ID '{agent_id}' not found in registry.")
            self.agents[agent_id] = agent_data_db 
            agent_data = agent_data_db
        
        status_dict = {
            **agent_data, 
            "health_status": "healthy" if not self._check_circuit_breaker(agent_id) else "unhealthy (circuit breaker tripped)",
            "is_instance_loaded": agent_id in self._agent_instances
        }
        
        if agent_id in self.circuit_breakers:
            cb_info = self.circuit_breakers[agent_id]
            status_dict.update({
                "circuit_breaker_failures": cb_info.get("failures", 0),
                "circuit_breaker_last_failure": datetime.fromtimestamp(cb_info.get("last_failure", 0)).isoformat() if cb_info.get("last_failure") else None
            })
        else:
            status_dict.update({
                "circuit_breaker_failures": 0,
                "circuit_breaker_last_failure": None
            })
        return status_dict

    async def health_check(self) -> Dict[str, Any]:
        db_ping_ok = False
        db_name_str = "N/A"
        if self.mongodb_client:
            db_ping_ok = await self.mongodb_client.ping_server()
            db_name_str = self.mongodb_client.db_name or "N/A (DB name not set)"
        
        total_agents_in_cache = len(self.agents)
        active_agents_in_cache = sum(1 for a in self.agents.values() if a.get("status") == "active")
        unhealthy_agents_count = sum(1 for agent_id in self.agents if self._check_circuit_breaker(agent_id))
        
        overall_status = "healthy"
        if not db_ping_ok: overall_status = "degraded (DB ping failed)"
        elif unhealthy_agents_count > 0 : overall_status = f"degraded ({unhealthy_agents_count} agents in circuit breaker)"


        return {
            "status": overall_status,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "mongodb_connection": "ok" if db_ping_ok else "failed_ping",
            "stats": {
                "total_agents_in_registry_cache": total_agents_in_cache,
                "active_agents_in_registry_cache": active_agents_in_cache,
                "loaded_live_instances": len(self._agent_instances),
                "agents_in_circuit_breaker": unhealthy_agents_count,
            },
            "storage_config": {
                "registry_collection": self.registry_collection_name, 
                "logs_collection": self.logs_collection_name,       
                "mongodb_database": db_name_str,
                "circuit_breaker_file": self.circuit_breaker_path
            },
            "dependencies": {
                "llm_service": "ok" if self.llm_service else "N/A",
                "smart_library": "ok" if self.smart_library else "N/A",
                "system_agent": "ok" if self.system_agent else "N/A", 
                "agent_factory": "ok" if self.container and self.container.has('agent_factory') else "N/A",
                "tool_factory": "ok" if self.container and self.container.has('tool_factory') else "N/A"
            }
        }

    async def clear_logs(self) -> Dict[str, Any]:
        if self.logs_collection is None: 
            msg = "Logs collection not available. Cannot clear logs."
            logger.warning(msg)
            return {"status": "skipped", "message": msg, "deleted_count": 0}
        try:
            result = await self.logs_collection.delete_many({})
            deleted_count = result.deleted_count
            logger.info(f"Cleared {deleted_count} execution logs from MongoDB '{self.logs_collection_name}'.")
            return {"status": "success", "message": f"Cleared {deleted_count} logs.", "deleted_count": deleted_count}
        except Exception as e:
            logger.error(f"Failed to clear logs from MongoDB: {e}", exc_info=True)
            return {"status": "error", "message": f"Failed to clear logs: {e}", "deleted_count": 0}


    async def list_all_agents(self, agent_type: Optional[str] = None) -> List[Dict[str, Any]]:
         if not self.agents and self.registry_collection is not None:
             await self._load_registry_from_db()

         agents_list = list(self.agents.values())
         if agent_type:
             agents_list = [a for a in agents_list if a.get("type", "").lower() == agent_type.lower()]
         
         return [{
             "id": a["id"],
             "name": a.get("name", "Unnamed Agent"),
             "type": a.get("type", "UNKNOWN"),
             "status": a.get("status", "unknown"),
             "description_snippet": (a.get("description","No description")[:70]+"...") if a.get("description") else "No description",
             "capabilities_count": len(a.get("capabilities", []))
         } for a in agents_list]

    async def get_logs(
        self,
        limit: int = 100,
        bus_type: Optional[str] = None,
        agent_id: Optional[str] = None,
        task_description: Optional[str] = None
    ) -> List[Dict[str, Any]]:
        """
        Retrieves logs from the logs_collection based on provided filters.

        Args:
            limit: The maximum number of logs to return. Defaults to 100.
            bus_type: Filter logs by the 'bus_type' field.
            agent_id: Filter logs by the 'agent_id' field.
            task_description: Filter logs by the 'task_description' field (exact match).

        Returns:
            A list of log documents, sorted by timestamp in descending order.
        """
        if self.logs_collection is None:
            logger.warning("SmartAgentBus.get_logs: logs_collection is not available. Returning empty list.")
            return []

        query_filter: Dict[str, Any] = {}
        if bus_type:
            query_filter["bus_type"] = bus_type
        if agent_id:
            query_filter["agent_id"] = agent_id
        if task_description:
            query_filter["task_description"] = task_description # Exact match

        projection = {"_id": 0}
        
        try:
            cursor = self.logs_collection.find(
                query_filter,
                projection
            ).sort("timestamp", pymongo.DESCENDING).limit(limit)
            
            logs = await cursor.to_list(length=limit)
            return logs
        except Exception as e:
            logger.error(f"SmartAgentBus.get_logs: Error querying logs: {e}", exc_info=True)
            return []