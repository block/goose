# evolving_agents/auditing/library_auditor.py

import logging
import asyncio
from typing import Optional, List, Dict, Any

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.monitoring.component_experience_tracker import ComponentExperienceTracker
# from evolving_agents.core.mongodb_client import MongoDBClient # Only if direct instantiation is needed

logger = logging.getLogger(__name__)

class LibraryAuditorAgent:
    """
    Audits the SmartLibrary and Component Experiences for potential issues
    like unused components, orphaned components, or low-performing components.
    """

    def __init__(self, smart_library: SmartLibrary, experience_tracker: ComponentExperienceTracker):
        self.smart_library = smart_library
        self.experience_tracker = experience_tracker
        self.logger = logging.getLogger(__name__)
        if self.experience_tracker.experience_collection is None:
            self.logger.error("Experience collection in ComponentExperienceTracker is not initialized. Audit may fail.")

    async def perform_audit(
        self,
        low_success_threshold: float = 0.5,
        min_invocations_for_success_check: int = 20
    ) -> Dict[str, List[Dict[str, Any]]]:
        """
        Orchestrates the different audit checks.
        Fetches all relevant data from SmartLibrary and ComponentExperienceTracker.
        """
        self.logger.info("Starting library audit...")

        # 1. Fetch all components from SmartLibrary
        all_components_list = await self.smart_library.export_records(record_type=None) # Get all types
        all_components: Dict[str, Dict] = {comp["id"]: comp for comp in all_components_list}
        self.logger.info(f"Fetched {len(all_components)} components from SmartLibrary.")

        # 2. Fetch all experience data
        all_experiences: Dict[str, Dict] = {}
        if self.experience_tracker.experience_collection is not None:
            try:
                cursor = self.experience_tracker.experience_collection.find({})
                async for exp_doc in cursor:
                    all_experiences[exp_doc["component_id"]] = exp_doc
                self.logger.info(f"Fetched {len(all_experiences)} experience records.")
            except Exception as e:
                self.logger.error(f"Error fetching experience records: {e}", exc_info=True)
        else:
            self.logger.warning("Experience collection is not available. Skipping experience-based audits.")


        # 3. Perform individual audit checks
        unused_components = await self._find_unused_components(all_components, all_experiences)
        potentially_orphaned_components = await self._find_potentially_orphaned_components(all_components)
        low_success_rate_components = await self._find_low_success_rate_components(
            all_experiences, all_components, low_success_threshold, min_invocations_for_success_check
        )

        self.logger.info("Library audit completed.")

        return {
            "unused_components": unused_components,
            "potentially_orphaned_components": potentially_orphaned_components,
            "low_success_rate_components": low_success_rate_components,
        }

    async def _find_unused_components(
        self, all_components: Dict[str, Dict], all_experiences: Dict[str, Dict]
    ) -> List[Dict[str, Any]]:
        """
        Identifies components that have no recorded invocations.
        """
        self.logger.debug("Finding unused components...")
        unused = []
        for comp_id, component in all_components.items():
            experience = all_experiences.get(comp_id)
            if not experience or experience.get("total_invocations", 0) == 0:
                unused.append({
                    "id": comp_id,
                    "name": component.get("name", "N/A"),
                    "record_type": component.get("record_type", component.get("type", "N/A")),
                    "reason": "No invocations recorded in experience tracker."
                })
        self.logger.info(f"Found {len(unused)} unused components.")
        return unused

    async def _find_potentially_orphaned_components(
        self, all_components: Dict[str, Dict]
    ) -> List[Dict[str, Any]]:
        """
        Identifies components whose parent_id points to a non-existent or inactive component.
        This primarily applies to components that are versions or derived from others.
        """
        self.logger.debug("Finding potentially orphaned components...")
        orphaned = []
        for comp_id, component in all_components.items():
            # Assuming 'parent_id' is stored in 'metadata' or directly on the component
            parent_id = component.get("metadata", {}).get("parent_id") or component.get("parent_id")

            if parent_id:
                parent_component = all_components.get(parent_id)
                reason = None
                if not parent_component:
                    reason = f"Parent component with ID '{parent_id}' not found in SmartLibrary."
                elif parent_component.get("status") != "active":
                    reason = f"Parent component '{parent_component.get('name', parent_id)}' (ID: {parent_id}) is not 'active' (status: {parent_component.get('status')})."

                if reason:
                    orphaned.append({
                        "id": comp_id,
                        "name": component.get("name", "N/A"),
                        "record_type": component.get("record_type", component.get("type", "N/A")),
                        "parent_id": parent_id,
                        "reason": reason
                    })
        self.logger.info(f"Found {len(orphaned)} potentially orphaned components.")
        return orphaned

    async def _find_low_success_rate_components(
        self,
        all_experiences: Dict[str, Dict],
        all_components: Dict[str, Dict], # Added to fetch name if not in experience
        low_success_threshold: float,
        min_invocations_for_success_check: int
    ) -> List[Dict[str, Any]]:
        """
        Identifies components with a success rate below a given threshold,
        considering only those with a minimum number of invocations.
        """
        self.logger.debug(f"Finding components with success rate < {low_success_threshold} and >= {min_invocations_for_success_check} invocations...")
        low_performers = []
        for comp_id, experience in all_experiences.items():
            total_invocations = experience.get("total_invocations", 0)
            successful_invocations = experience.get("successful_invocations", 0)

            if total_invocations >= min_invocations_for_success_check:
                success_rate = (successful_invocations / total_invocations) if total_invocations > 0 else 0
                if success_rate < low_success_threshold:
                    component_name = experience.get("name") # Name might be in experience record
                    if not component_name and comp_id in all_components: # Fallback to smart library
                         component_name = all_components[comp_id].get("name", "N/A")
                    elif not component_name:
                        component_name = "N/A (Not in SmartLibrary or Experience Name)"

                    low_performers.append({
                        "id": comp_id,
                        "name": component_name,
                        "record_type": experience.get("record_type", all_components.get(comp_id, {}).get("type", "N/A")),
                        "success_rate": round(success_rate, 3),
                        "total_invocations": total_invocations,
                        "successful_invocations": successful_invocations,
                        "failed_invocations": experience.get("failed_invocations", 0),
                        "reason": f"Success rate {success_rate:.2%} is below threshold {low_success_threshold:.2%} with {total_invocations} invocations."
                    })
        self.logger.info(f"Found {len(low_performers)} low success rate components.")
        return low_performers


# Example usage (async main)
async def main():
    # This is an example and requires actual instances of MongoDBClient,
    # SmartLibrary, and ComponentExperienceTracker to be set up.
    # For demonstration, it's kept conceptual.

    print("Starting LibraryAuditorAgent example (conceptual)...")

    # --- Mocking/Setup (replace with actual initialization) ---
    class MockAsyncMongoCollection:
        def __init__(self, items=None):
            self._items = items if items else []
        async def find(self, query, projection=None): # Added projection
            # Basic filtering for demonstration, not a full MongoDB query engine
            # For this example, we'll assume query is {} to return all items
            if query == {}:
                return self # Return self to simulate a cursor
            return self # Simplified
        async def __aiter__(self): # Make it an async iterator
            for item in self._items:
                yield item
        @property
        def name(self): return "mock_collection"

    class MockMongoDBClient:
        def __init__(self, db_name="mock_db"):
            self.db_name = db_name
            self._collections = {}
        def get_collection(self, collection_name):
            if collection_name not in self._collections:
                # Populate with some dummy data for testing
                if collection_name == "eat_component_experiences":
                    self._collections[collection_name] = MockAsyncMongoCollection([
                        {"component_id": "agent_001", "name": "Summarizer Agent", "record_type": "AGENT", "total_invocations": 50, "successful_invocations": 45, "failed_invocations": 5},
                        {"component_id": "tool_001", "name": "Calculator Tool", "record_type": "TOOL", "total_invocations": 100, "successful_invocations": 98, "failed_invocations": 2},
                        {"component_id": "agent_002", "name": "Data Analyst Agent", "record_type": "AGENT", "total_invocations": 25, "successful_invocations": 10, "failed_invocations": 15}, # Low success
                        {"component_id": "tool_002", "name": "File Writer Tool", "record_type": "TOOL", "total_invocations": 5, "successful_invocations": 5}, # Low invocations for success check
                        {"component_id": "agent_003", "name": "Unused Agent", "record_type": "AGENT", "total_invocations": 0, "successful_invocations": 0},
                        {"component_id": "tool_003", "name": "Beta Feature Tool", "record_type": "TOOL"}, # No invocations in experience
                    ])
                elif collection_name == "eat_library_components": # For SmartLibrary
                     self._collections[collection_name] = MockAsyncMongoCollection([
                        {"id": "agent_001", "name": "Summarizer Agent", "type": "AGENT", "status": "active"},
                        {"id": "tool_001", "name": "Calculator Tool", "type": "TOOL", "status": "active"},
                        {"id": "agent_002", "name": "Data Analyst Agent", "type": "AGENT", "status": "active"},
                        {"id": "tool_002", "name": "File Writer Tool", "type": "TOOL", "status": "active"},
                        {"id": "agent_003", "name": "Unused Agent", "type": "AGENT", "status": "active"},
                        {"id": "tool_003", "name": "Beta Feature Tool", "type": "TOOL", "status": "active"},
                        {"id": "agent_004", "name": "Orphaned Agent", "type": "AGENT", "status": "active", "parent_id": "non_existent_parent"},
                        {"id": "agent_005", "name": "Orphaned By Inactive", "type": "AGENT", "status": "active", "parent_id": "agent_006"},
                        {"id": "agent_006", "name": "Inactive Parent", "type": "AGENT", "status": "inactive"},
                        {"id": "agent_007", "name": "Well Used Agent", "type": "AGENT", "status": "active"},
                     ])
                else:
                    self._collections[collection_name] = MockAsyncMongoCollection()
            return self._collections[collection_name]
        async def ping_server(self): return True # Mock ping

    class MockComponentExperienceTracker:
        def __init__(self, mongodb_client):
            self.mongodb_client = mongodb_client
            self.experience_collection = self.mongodb_client.get_collection("eat_component_experiences")
            self.logger = logging.getLogger("MockExperienceTracker")

    class MockSmartLibrary:
        def __init__(self, mongodb_client):
            self.mongodb_client = mongodb_client
            self.components_collection = self.mongodb_client.get_collection("eat_library_components")
            self.logger = logging.getLogger("MockSmartLibrary")

        async def export_records(self, record_type: Optional[str] = None) -> List[Dict[str, Any]]:
            # Simplified export, does not filter by record_type for this mock
            records = []
            async for item in self.components_collection.find({}): # Pass empty query
                records.append(item)
            return records


    # Setup logging to see output
    logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')

    # Instantiate with mocks
    mock_mongo_client = MockMongoDBClient()
    smart_library = MockSmartLibrary(mongodb_client=mock_mongo_client)
    experience_tracker = MockComponentExperienceTracker(mongodb_client=mock_mongo_client)

    # Add a component to experience tracker that is not in smart library for one test case
    experience_tracker.experience_collection._items.append(
         {"component_id": "exp_only_001", "name": "ExperienceOnlyComponent", "record_type": "AGENT", "total_invocations": 30, "successful_invocations": 10}
    )


    auditor = LibraryAuditorAgent(smart_library, experience_tracker)
    audit_results = await auditor.perform_audit(low_success_threshold=0.5, min_invocations_for_success_check=10)

    import json
    print("\n--- Audit Results ---")
    print(json.dumps(audit_results, indent=2))
    print("--- End of Audit Results ---")

    # Example: How to access specific parts of the audit
    # print("\nLow Success Rate Components:")
    # for comp in audit_results.get("low_success_rate_components", []):
    #     print(f"  ID: {comp['id']}, Name: {comp['name']}, Rate: {comp['success_rate']:.2%}, Invocations: {comp['total_invocations']}")


if __name__ == "__main__":
    # To run this example:
    # Ensure you have an async environment or use asyncio.run()
    # The mocks are basic; a real test would require more sophisticated data setup.
    asyncio.run(main())
    pass
