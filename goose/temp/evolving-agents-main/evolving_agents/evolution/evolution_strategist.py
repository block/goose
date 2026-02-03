# evolving_agents/evolution/evolution_strategist.py

import logging
import asyncio
from typing import List, Dict, Any, Optional

from evolving_agents.monitoring.component_experience_tracker import ComponentExperienceTracker
from evolving_agents.auditing.library_auditor import LibraryAuditorAgent
# from evolving_agents.core.mongodb_client import MongoDBClient # For main example
# from evolving_agents.smart_library.smart_library import SmartLibrary # For main example

logger = logging.getLogger(__name__)

AB_TEST_FETCH_LIMIT = 30  # Number of latest A/B tests to fetch
AB_TEST_SIGNIFICANT_IMPROVEMENT_THRESHOLD = 10.0 # Percentage
AB_TEST_WINNER_PRIORITY_BONUS = 15
AB_TEST_PARENT_RECONSIDER_PRIORITY_BONUS = 5


class EvolutionStrategistAgent:
    """
    Identifies components that are prime candidates for evolution
    based on audit results and experience data.
    """

    def __init__(self, experience_tracker: ComponentExperienceTracker, auditor: LibraryAuditorAgent):
        self.experience_tracker = experience_tracker
        self.auditor = auditor
        self.logger = logging.getLogger(__name__)
        if self.experience_tracker.experience_collection is None:
            self.logger.error("Experience collection in ComponentExperienceTracker is not initialized. Strategist may fail.")


    async def identify_evolution_candidates(self, top_n: int = 10) -> Dict[str, List[Dict[str, Any]]]:
        """
        Identifies and prioritizes components for evolution.
        """
        self.logger.info(f"Starting evolution candidate identification (top_n={top_n})...")

        # 1. Fetch Audit Data
        audit_results = await self.auditor.perform_audit()
        self.logger.info(f"Audit results received: { {k: len(v) for k, v in audit_results.items()} }")

        # 2. Fetch Additional Experience Data
        # Components with high failed_invocations
        high_failure_components = []
        if self.experience_tracker.experience_collection:
            try:
                # Using the placeholder method (assuming it's implemented or will be)
                # For now, let's use a direct query as per instructions.
                # high_failure_components = await self.experience_tracker.get_top_n_components_by_metric(
                #     metric="failed_invocations", n=top_n * 2, ascending=False # Fetch more to allow for overlap
                # )
                cursor = self.experience_tracker.experience_collection.find(
                    {"failed_invocations": {"$gt": 0}}
                ).sort("failed_invocations", -1).limit(top_n * 2) # pymongo.DESCENDING == -1
                async for doc in cursor:
                    high_failure_components.append(doc)
                self.logger.info(f"Fetched {len(high_failure_components)} components with high failed invocations.")
            except Exception as e:
                self.logger.error(f"Error fetching high failure components: {e}", exc_info=True)
        
        # Components with many distinct error types (more complex, placeholder for now)
        # This would typically require an aggregation query.
        # Example:
        # pipeline = [
        #     {"$match": {"error_types_summary": {"$exists": True, "$ne": {}}}},
        #     {"$addFields": {"num_error_types": {"$size": {"$objectToArray": "$error_types_summary"}}}},
        #     {"$sort": {"num_error_types": -1}},
        #     {"$limit": top_n * 2}
        # ]
        # diverse_error_components_cursor = self.experience_tracker.experience_collection.aggregate(pipeline)
        # async for doc in diverse_error_components_cursor: diverse_error_components.append(doc)


        # 3. Consolidate and Prioritize Candidates
        candidates: Dict[str, Dict[str, Any]] = {}

        def add_candidate(comp_id: str, name: Optional[str], record_type: Optional[str], reason: str, priority_bonus: int):
            if not comp_id: # Skip if comp_id is None or empty
                self.logger.warning(f"Skipping candidate with no ID. Reason: {reason}, Name: {name}")
                return
            if comp_id not in candidates:
                candidates[comp_id] = {
                    "id": comp_id,
                    "name": name or "N/A",
                    "record_type": record_type or "UNKNOWN",
                    "reasons": [],
                    "priority_score": 0
                }
            candidates[comp_id]["reasons"].append(reason)
            candidates[comp_id]["priority_score"] += priority_bonus
            if name and candidates[comp_id]["name"] == "N/A": # Update name if a better one comes along
                candidates[comp_id]["name"] = name
            if record_type and candidates[comp_id]["record_type"] == "UNKNOWN":
                candidates[comp_id]["record_type"] = record_type


        # Priority 1: Low success rate components (from auditor)
        for comp in audit_results.get("low_success_rate_components", []):
            reason_detail = comp.get('reason', f"Low success rate: {comp.get('success_rate', 'N/A')}")
            add_candidate(comp.get("id"), comp.get("name"), comp.get("record_type"), reason_detail, priority_bonus=20)

        # Priority 2: High failed invocations (direct query)
        for comp_exp in high_failure_components:
            reason = f"High failed invocations: {comp_exp.get('failed_invocations', 0)}"
            # Name might be in experience, or we might need to fetch it if not already covered by audit
            add_candidate(comp_exp.get("component_id"), comp_exp.get("name"), comp_exp.get("record_type"), reason, priority_bonus=15)

        # Priority 3: Potentially orphaned components (from auditor)
        for comp in audit_results.get("potentially_orphaned_components", []):
            add_candidate(comp.get("id"), comp.get("name"), comp.get("record_type"), comp.get('reason', "Potentially orphaned"), priority_bonus=10)
        
        # Priority 4: Unused components (from auditor) - lower priority for evolution, maybe for removal
        for comp in audit_results.get("unused_components", []):
            add_candidate(comp.get("id"), comp.get("name"), comp.get("record_type"), comp.get('reason', "Unused component"), priority_bonus=5)


        # 4. Fetch and Process A/B Test Results
        self.logger.info(f"Fetching last {AB_TEST_FETCH_LIMIT} A/B test results...")
        ab_test_results = []
        if self.experience_tracker.ab_test_collection: # Check if ab_test_collection is initialized
            try:
                ab_test_results = await self.experience_tracker.get_latest_ab_test_results(limit=AB_TEST_FETCH_LIMIT)
                self.logger.info(f"Fetched {len(ab_test_results)} A/B test results.")
            except Exception as e:
                self.logger.error(f"Error fetching A/B test results: {e}", exc_info=True)
        
        processed_ab_components = set() # To track components already influenced by A/B tests

        for test_result in ab_test_results:
            agent_a_id = test_result.get("agent_a_id")
            agent_b_id = test_result.get("agent_b_id")
            overall_winner = test_result.get("overall_winner")
            percentage_diff = test_result.get("percentage_difference")
            
            if not agent_a_id or not agent_b_id:
                self.logger.warning(f"Skipping A/B test result due to missing agent IDs: {test_result.get('_id')}")
                continue

            processed_ab_components.add(agent_a_id)
            processed_ab_components.add(agent_b_id)

            agent_a_name = candidates.get(agent_a_id, {}).get("name", agent_a_id) # Try to get name from existing candidates
            agent_b_name = candidates.get(agent_b_id, {}).get("name", agent_b_id)


            if overall_winner == "Agent B" and percentage_diff is not None and percentage_diff > AB_TEST_SIGNIFICANT_IMPROVEMENT_THRESHOLD:
                reason = f"A/B test confirmed significant improvement ({percentage_diff:.2f}%) over parent {agent_a_name} (ID: {agent_a_id})."
                self.logger.info(f"A/B Test WIN for {agent_b_id}: {reason}")
                add_candidate(agent_b_id, agent_b_name, candidates.get(agent_b_id, {}).get("record_type"), reason, priority_bonus=AB_TEST_WINNER_PRIORITY_BONUS)
                
                # Log insight for future prompt refinement
                insight = f"Insight for {agent_b_id}: Successfully evolved from {agent_a_id}, showing {percentage_diff:.2f}% improvement. Strengths to maintain/build upon."
                # TODO: Store this insight with the candidate if data structure allows, or log for now.
                self.logger.info(insight)

            elif overall_winner == "Agent A" or (overall_winner == "Tie" and (percentage_diff is None or percentage_diff <= 0)):
                reason_for_a = f"Previous evolution ({agent_b_name} ID: {agent_b_id}) failed to improve or performed worse. Re-evaluating parent for new strategy."
                self.logger.info(f"A/B Test FAILED/STAGNATED for {agent_b_id} relative to {agent_a_id}: {reason_for_a}")
                add_candidate(agent_a_id, agent_a_name, candidates.get(agent_a_id, {}).get("record_type"), reason_for_a, priority_bonus=AB_TEST_PARENT_RECONSIDER_PRIORITY_BONUS)
                
                # Log insight for future prompt refinement for agent_a or a new evolution attempt for agent_b
                insight = f"Insight for {agent_a_id} (or new evolution of {agent_b_id}): Evolution to {agent_b_id} was not successful. Analyze A/B test details (ID: {test_result.get('_id')}) to understand failure points (e.g., specific criteria, performance regressions)."
                # TODO: Store this insight.
                self.logger.info(insight)
                
                # Optionally, if agent_b is a candidate, its priority might be reduced or its reasons updated.
                if agent_b_id in candidates:
                    candidates[agent_b_id]["reasons"].append(f"A/B test showed no significant improvement/regression against parent {agent_a_name} (ID: {agent_a_id}).")
                    # candidates[agent_b_id]["priority_score"] -= 5 # Optional: penalty or just no bonus

            elif overall_winner == "Tie":
                 reason = f"A/B test resulted in a tie with parent {agent_a_name} (ID: {agent_a_id}). No significant change observed ({percentage_diff if percentage_diff is not None else 'N/A'}%)."
                 self.logger.info(f"A/B Test TIE for {agent_b_id} relative to {agent_a_id}: {reason}")
                 if agent_b_id in candidates:
                     candidates[agent_b_id]["reasons"].append(reason)
                 # No specific priority change for a tie unless specific criteria are met.
                 insight = f"Insight for {agent_b_id}: A/B test was a tie with {agent_a_id}. Further analysis of detailed scores needed to determine if specific aspects improved or regressed."
                 self.logger.info(insight)


        # 5. Sort candidates by priority_score (descending), then by number of reasons (descending)
        sorted_candidates = sorted(
            list(candidates.values()), # Ensure we are sorting a list of the dictionary values
            key=lambda x: (x["priority_score"], len(x["reasons"])),
            reverse=True
        )

        self.logger.info(f"Identified {len(sorted_candidates)} unique potential evolution candidates after A/B test processing, before limiting to top_n.")

        final_candidates = sorted_candidates[:top_n]

        self.logger.info(f"Returning {len(final_candidates)} prioritized evolution candidates.")
        return {"evolution_candidates": final_candidates}


async def main():
    # --- Mocking/Setup (replace with actual initialization) ---
    # This setup is illustrative and requires a running MongoDB instance
    # or more comprehensive mocking for ComponentExperienceTracker and SmartLibrary.

    from evolving_agents.core.mongodb_client import MongoDBClient
    from evolving_agents.smart_library.smart_library import SmartLibrary # For auditor

    logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
    logger.info("EvolutionStrategistAgent main example starting...")

    # ---- MongoDB Connection (replace with your actual URI and DB name) ----
    MONGODB_URI = "mongodb://localhost:27017/"
    DB_NAME = "evolving_agents_dev_strategist"
    
    try:
        mongodb_client = MongoDBClient(uri=MONGODB_URI, db_name=DB_NAME)
        # You might need an explicit connect method if your MongoDBClient has one
        # await mongodb_client.connect() 
        logger.info(f"Connected to MongoDB: {DB_NAME} (mocked or actual based on client)")

        # --- Instantiate dependencies ---
        # 1. SmartLibrary (needed by LibraryAuditorAgent)
        #    For this example, we'll use a mock version of SmartLibrary's data interactions
        #    by pre-populating its expected collection if it's empty or using mocks.
        smart_library_collection_name = "eat_library_components_strategist"
        smart_lib_mock_data = [
            {"id": "agent_001", "name": "Summarizer Agent", "type": "AGENT", "status": "active"},
            {"id": "tool_001", "name": "Calculator Tool", "type": "TOOL", "status": "active"},
            {"id": "agent_002", "name": "Data Analyst Agent", "type": "AGENT", "status": "active", "metadata": {"parent_id": "non_existent_parent"}}, # Orphaned
            {"id": "tool_002", "name": "File Writer Tool", "type": "TOOL", "status": "active"},
            {"id": "agent_003", "name": "Unused Agent", "type": "AGENT", "status": "active"},
        ]
        # Optional: Clear and insert mock data into SmartLibrary's collection for consistent testing
        # await mongodb_client.db[smart_library_collection_name].delete_many({})
        # if smart_library_collection_name not in await mongodb_client.db.list_collection_names() or await mongodb_client.db[smart_library_collection_name].count_documents({}) == 0 :
        #     await mongodb_client.db[smart_library_collection_name].insert_many(smart_lib_mock_data)
        #     logger.info(f"Inserted mock data into {smart_library_collection_name}")

        smart_library = SmartLibrary(mongodb_client=mongodb_client, components_collection_name=smart_library_collection_name)
        # Ensure SmartLibrary has some data if collection was empty
        if not await smart_library.export_records():
             logger.warning(f"SmartLibrary collection '{smart_library_collection_name}' is empty. Populating with mock data for example.")
             # Manually insert using the client if SmartLibrary doesn't have an import/insert method
             if smart_library_collection_name not in await mongodb_client.db.list_collection_names() or await mongodb_client.db[smart_library_collection_name].count_documents({}) == 0:
                 await mongodb_client.db[smart_library_collection_name].insert_many(smart_lib_mock_data)
                 logger.info(f"Inserted mock data into {smart_library_collection_name}")


        # 2. ComponentExperienceTracker
        experience_collection_name = "eat_component_experiences_strategist"
        exp_mock_data = [
            {"component_id": "agent_001", "name": "Summarizer Agent", "record_type": "AGENT", "total_invocations": 50, "successful_invocations": 20, "failed_invocations": 30, "error_types_summary": {"ValueError": 30}}, # Low success
            {"component_id": "tool_001", "name": "Calculator Tool", "record_type": "TOOL", "total_invocations": 5, "successful_invocations": 5, "failed_invocations": 0}, # Low invocations for success check
            {"component_id": "agent_002", "name": "Data Analyst Agent", "record_type": "AGENT", "total_invocations": 25, "successful_invocations": 23, "failed_invocations": 2}, # OK, but orphaned
            # tool_002 has no experience record -> unused by this metric
            # agent_003 has no experience record -> unused
        ]
        # Optional: Clear and insert mock data
        # await mongodb_client.db[experience_collection_name].delete_many({})
        # if experience_collection_name not in await mongodb_client.db.list_collection_names() or await mongodb_client.db[experience_collection_name].count_documents({}) == 0:
        #    await mongodb_client.db[experience_collection_name].insert_many(exp_mock_data)
        #    logger.info(f"Inserted mock data into {experience_collection_name}")
        
        # For the main example, ensure both collection names are passed if testing A/B features
        experience_tracker = ComponentExperienceTracker(
            mongodb_client=mongodb_client, 
            experience_collection_name=experience_collection_name,
            ab_test_collection_name="eat_ab_test_results_strategist" # Example A/B test collection
        )
        
        # Mock A/B test data for the example run:
        ab_test_mock_data = [
            {
                "agent_a_id": "agent_001", "agent_b_id": "agent_001_v2", 
                "test_inputs_summary": [], "evaluation_criteria": ["quality"],
                "agent_a_results": {"aggregated_scores": {"quality": 0.6}},
                "agent_b_results": {"aggregated_scores": {"quality": 0.8}},
                "overall_winner": "Agent B", "percentage_difference": 33.33,
                "test_timestamp": datetime.now(timezone.utc) 
            },
            {
                "agent_a_id": "tool_001", "agent_b_id": "tool_001_v2", 
                "test_inputs_summary": [], "evaluation_criteria": ["speed"],
                "agent_a_results": {"aggregated_scores": {"speed": 100}},
                "agent_b_results": {"aggregated_scores": {"speed": 90}},
                "overall_winner": "Agent A", "percentage_difference": -10.0,
                "test_timestamp": datetime.now(timezone.utc)
            }
        ]
        if experience_tracker.ab_test_collection is not None and await experience_tracker.ab_test_collection.count_documents({}) == 0:
            logger.info(f"Populating mock A/B test data into {experience_tracker.ab_test_collection.name}")
            await experience_tracker.ab_test_collection.insert_many([ABTestRecordSchema(**data).model_dump(mode="python") for data in ab_test_mock_data])


        # Ensure experience_tracker has some data if collection was empty
        # A bit harder to check without a direct "get_all" method, but we can try inserting if count is 0.
        if experience_tracker.experience_collection is not None and await experience_tracker.experience_collection.count_documents({}) == 0:
            logger.warning(f"ExperienceTracker collection '{experience_collection_name}' is empty. Populating with mock data for example.")
            # Ensure exp_mock_data is defined or accessible here if this block is to run
            await experience_tracker.experience_collection.insert_many(exp_mock_data)
            logger.info(f"Inserted mock data into {experience_collection_name}")
        
        # Need to import these for the mock data population if not already globally available in main()
        from datetime import datetime, timezone
        from evolving_agents.monitoring.component_experience_tracker import ABTestRecordSchema


        # 3. LibraryAuditorAgent
        auditor = LibraryAuditorAgent(smart_library=smart_library, experience_tracker=experience_tracker)

        # 4. EvolutionStrategistAgent
        strategist = EvolutionStrategistAgent(experience_tracker=experience_tracker, auditor=auditor)

        # --- Run identification ---
        evolution_candidates_result = await strategist.identify_evolution_candidates(top_n=5)

        import json
        print("\n--- Evolution Candidates Result ---")
        print(json.dumps(evolution_candidates_result, indent=2))
        print("--- End of Evolution Candidates Result ---")

    except Exception as e:
        logger.error(f"Error in EvolutionStrategistAgent main example: {e}", exc_info=True)
    finally:
        if 'mongodb_client' in locals() and hasattr(mongodb_client, 'close'):
            # await mongodb_client.close() # If your client has an async close
            pass
        logger.info("EvolutionStrategistAgent main example finished.")


if __name__ == "__main__":
    # To run this example:
    # 1. Ensure MongoDB is running and accessible via MONGODB_URI.
    # 2. The example attempts to create/use specific collections for testing.
    #    You might want to clear these collections before runs for consistency.
    # 3. The mocking for SmartLibrary and ComponentExperienceTracker data is basic.
    #    It relies on direct DB inserts for the example.
    asyncio.run(main())
    pass
