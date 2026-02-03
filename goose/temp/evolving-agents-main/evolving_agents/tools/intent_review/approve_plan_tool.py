# evolving_agents/tools/intent_review/approve_plan_tool.py

import json
import asyncio
import os
import time
from datetime import datetime, timezone # Use timezone-aware datetimes
import logging
from typing import Dict, Any, List, Optional
import re # Added for robust JSON extraction in _agent_review_plan

from pydantic import BaseModel, Field
import pymongo # For pymongo constants

# BeeAI Framework imports
from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext # Keep RunContext import
from beeai_framework.emitter.emitter import Emitter

# Project-specific imports
from evolving_agents.core.llm_service import LLMService
from evolving_agents.core.intent_review import IntentPlan, PlanStatus, IntentStatus # Import data classes
from evolving_agents.core.mongodb_client import MongoDBClient # For DB interaction
from evolving_agents.core.dependency_container import DependencyContainer # For resolving MongoDBClient
# Using safe_json_dumps from the new utils location
from evolving_agents.utils.json_utils import safe_json_dumps

logger = logging.getLogger(__name__)

class ApprovePlanInput(BaseModel):
    """Input schema for the ApprovePlanTool."""
    plan_id: str = Field(description="ID of the plan to review (will be loaded from MongoDB or context)")
    interactive_mode: bool = Field(True, description="Whether to use interactive mode for review")
    use_agent_reviewer: bool = Field(False, description="Whether to use AI agent to review instead of human")
    agent_prompt: Optional[str] = Field(None, description="Custom prompt for the AI reviewer")
    # output_path is for optional file dump, not primary storage
    output_path: Optional[str] = Field(None, description="Optional path to save a copy of the reviewed intent plan")
    # Field to receive the full plan if DB load fails or is skipped
    intent_plan_json_override: Optional[str] = Field(None, description="Fallback: JSON string of the IntentPlan if not loaded from DB")


class ApprovePlanTool(Tool[ApprovePlanInput, None, StringToolOutput]):
    """
    Tool for reviewing and approving intent plans.
    Loads plans from MongoDB using plan_id, or from context as a fallback.
    Updates the plan status in MongoDB after review.
    """
    name = "ApprovePlanTool"
    description = "Review and approve intent plans (from MongoDB or context) before execution in the SystemAgent"
    input_schema = ApprovePlanInput

    def __init__(
        self,
        llm_service: Optional[LLMService] = None,
        mongodb_client: Optional[MongoDBClient] = None,
        container: Optional[DependencyContainer] = None,
        options: Optional[Dict[str, Any]] = None
    ):
        super().__init__(options=options or {})
        self.llm_service = llm_service
        self.container = container

        if mongodb_client:
            self.mongodb_client = mongodb_client
        elif self.container and self.container.has('mongodb_client'):
            self.mongodb_client = self.container.get('mongodb_client')
        else:
            try:
                self.mongodb_client = MongoDBClient()
                if self.container and not self.container.has('mongodb_client'):
                    self.container.register('mongodb_client', self.mongodb_client)
                logger.info("ApprovePlanTool: Default MongoDBClient instance created.")
            except ValueError as e:
                logger.warning(f"ApprovePlanTool: MongoDBClient FAILED to initialize: {e}. "
                               f"Will rely on intent_plan_json_override or context for plan data.")
                self.mongodb_client = None

        if self.mongodb_client:
            self.intent_plans_collection_name = "eat_intent_plans"
            self.intent_plans_collection = self.mongodb_client.get_collection(self.intent_plans_collection_name)
        else:
            self.intent_plans_collection = None
            self.intent_plans_collection_name = "N/A (MongoDB unavailable)"
        logger.info(f"ApprovePlanTool initialized. MongoDB for intent plans: {self.intent_plans_collection_name}")


    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "intent_review", "approve_plan"],
            creator=self,
        )

    async def _run(self, tool_input: ApprovePlanInput, options: Optional[Dict[str, Any]] = None,
                  context: Optional[RunContext] = None) -> StringToolOutput:
        intent_plan_dict: Optional[Dict[str, Any]] = None

        # 1. Attempt to load IntentPlan
        # Priority: tool_input.intent_plan_json_override -> MongoDB -> context.get("intent_plan_json_output")
        if tool_input.intent_plan_json_override:
            try:
                intent_plan_dict = json.loads(tool_input.intent_plan_json_override)
                if intent_plan_dict.get("plan_id") != tool_input.plan_id:
                    logger.warning(f"Plan ID mismatch: Override plan ID {intent_plan_dict.get('plan_id')} vs requested {tool_input.plan_id}. Ignoring override.")
                    intent_plan_dict = None
                else:
                    logger.info(f"Loaded IntentPlan '{tool_input.plan_id}' from intent_plan_json_override input.")
            except json.JSONDecodeError:
                logger.error(f"Failed to parse intent_plan_json_override for plan '{tool_input.plan_id}'.")
                intent_plan_dict = None

        if not intent_plan_dict and self.intent_plans_collection:
            try:
                # Using motor syntax
                db_doc = await self.intent_plans_collection.find_one({"plan_id": tool_input.plan_id})
                if db_doc:
                    db_doc.pop("_id", None) # Remove MongoDB's internal _id
                    intent_plan_dict = db_doc
                    logger.info(f"Loaded IntentPlan '{tool_input.plan_id}' from MongoDB.")
            except Exception as e:
                logger.error(f"Error loading IntentPlan '{tool_input.plan_id}' from MongoDB: {e}", exc_info=True)
                # Proceed to check context as a fallback

        if not intent_plan_dict and context and hasattr(context, 'context') and isinstance(context.context, dict):
            intent_plan_json_from_context = context.context.get("intent_plan_json_output") # From ProcessWorkflowTool
            if intent_plan_json_from_context:
                try:
                    parsed_from_context = json.loads(intent_plan_json_from_context)
                    if parsed_from_context.get("plan_id") == tool_input.plan_id:
                        intent_plan_dict = parsed_from_context
                        logger.info(f"Loaded IntentPlan '{tool_input.plan_id}' from RunContext.")
                    else:
                        logger.warning(f"Plan ID mismatch: Context plan ID {parsed_from_context.get('plan_id')} vs requested {tool_input.plan_id}.")
                except json.JSONDecodeError:
                    logger.error("Failed to parse intent plan from context.")

        if not intent_plan_dict:
            return StringToolOutput(safe_json_dumps({
                "status": "error",
                "message": f"IntentPlan with ID '{tool_input.plan_id}' not found in any source (override, MongoDB, context)."
            }))

        # Convert to IntentPlan object for easier internal manipulation, then back to dict for saving/output.
        # This ensures consistency if the review methods modify it.
        try:
            intent_plan_obj = IntentPlan.from_dict(intent_plan_dict)
        except Exception as e: # Broad exception for Pydantic validation or other conversion issues
            logger.error(f"Failed to instantiate IntentPlan object from loaded dictionary for plan '{tool_input.plan_id}': {e}", exc_info=True)
            return StringToolOutput(safe_json_dumps({
                "status": "error", "message": f"Invalid IntentPlan data structure for plan '{tool_input.plan_id}': {e}"
            }))


        # --- Review Process (Human or AI) ---
        review_decision: Dict[str, Any] = {}
        modified_plan_for_saving = intent_plan_obj # Start with the loaded plan object

        if tool_input.use_agent_reviewer and self.llm_service:
            # _agent_review_plan now works with the IntentPlan object
            agent_review_output_str = await self._agent_review_plan(modified_plan_for_saving, tool_input.agent_prompt)
            review_decision = json.loads(agent_review_output_str.get_text_content()) # Assuming AI returns valid JSON
            # If AI review modifies the plan (e.g., adds comments to intents), it should update `modified_plan_for_saving`
            # For now, assume _agent_review_plan doesn't modify the plan object, only returns decision.
        elif tool_input.interactive_mode:
            interactive_output_str = await self._interactive_review_plan(modified_plan_for_saving)
            review_decision = json.loads(interactive_output_str.get_text_content())
            if review_decision.get("status") == PlanStatus.APPROVED.value and "approved_plan" in review_decision:
                # The interactive review might have modified the plan (e.g. statuses)
                modified_plan_for_saving = IntentPlan.from_dict(review_decision["approved_plan"])
        else: # CLI mode
            cli_output_str = await self._cli_review_plan(modified_plan_for_saving)
            review_decision = json.loads(cli_output_str.get_text_content())
            if review_decision.get("status") == PlanStatus.APPROVED.value and "approved_plan" in review_decision:
                modified_plan_for_saving = IntentPlan.from_dict(review_decision["approved_plan"])

        # --- Update IntentPlan Object and Persist to MongoDB ---
        final_status_str = review_decision.get("status") # "approved", "rejected", "cancelled", "timeout"

        if final_status_str == PlanStatus.APPROVED.value:
            modified_plan_for_saving.status = PlanStatus.APPROVED
            modified_plan_for_saving.reviewer_comments = review_decision.get("comments", modified_plan_for_saving.reviewer_comments)
            modified_plan_for_saving.review_timestamp = datetime.now(timezone.utc).isoformat()
            # Mark all intents as approved if the plan is approved (unless per-intent review is implemented)
            for intent_item in modified_plan_for_saving.intents: # Renamed intent to intent_item
                intent_item.status = IntentStatus.APPROVED # Or PENDING if execution is separate
                intent_item.review_comments = review_decision.get("comments", "") # Propagate plan comments to intents

        elif final_status_str == PlanStatus.REJECTED.value:
            modified_plan_for_saving.status = PlanStatus.REJECTED
            modified_plan_for_saving.rejection_reason = review_decision.get("reason", "No reason provided.")
            modified_plan_for_saving.review_timestamp = datetime.now(timezone.utc).isoformat()
            for intent_item in modified_plan_for_saving.intents: # Renamed intent to intent_item
                intent_item.status = IntentStatus.REJECTED
                intent_item.review_comments = review_decision.get("reason", "Plan rejected")


        # Persist updated plan to MongoDB if approved or rejected
        if modified_plan_for_saving.status in [PlanStatus.APPROVED, PlanStatus.REJECTED]:
            if self.intent_plans_collection:
                try:
                    updated_plan_dict_for_db = modified_plan_for_saving.to_dict()
                    await self.intent_plans_collection.replace_one(
                        {"plan_id": tool_input.plan_id},
                        updated_plan_dict_for_db,
                        upsert=False # Should only update existing
                    )
                    logger.info(f"Updated IntentPlan '{tool_input.plan_id}' status to '{modified_plan_for_saving.status.value}' in MongoDB.")
                except Exception as db_err:
                    logger.error(f"Failed to update IntentPlan '{tool_input.plan_id}' in MongoDB: {db_err}", exc_info=True)
                    review_decision["db_update_status"] = f"failed: {db_err}" # Inform caller of DB issue
            else:
                logger.warning(f"MongoDB client not available. IntentPlan '{tool_input.plan_id}' status updated in memory but not saved to DB.")
                review_decision["db_update_status"] = "skipped_mongodb_unavailable"


        # Optional: Save a copy to file if output_path is provided (for audit/debug)
        if tool_input.output_path:
            output_dir = os.path.dirname(tool_input.output_path)
            if output_dir and not os.path.exists(output_dir): os.makedirs(output_dir, exist_ok=True)
            try:
                # Use safe_json_dumps for file output too, for consistency
                with open(tool_input.output_path, 'w') as f:
                    f.write(safe_json_dumps(modified_plan_for_saving.to_dict()))
                logger.info(f"Reviewed intent plan copy saved to {tool_input.output_path}")
            except Exception as e:
                 logger.error(f"Failed to save reviewed intent plan copy to {tool_input.output_path}: {e}")


        # Return the original review decision (which includes status, messages, etc.)
        # If the plan was approved, and review methods returned the "approved_plan", it's implicitly included
        # If DB update failed, "db_update_status" is added to the response.
        return StringToolOutput(safe_json_dumps(review_decision))


    async def _agent_review_plan(self, intent_plan_obj: IntentPlan, custom_prompt: Optional[str] = None) -> StringToolOutput:
        if not self.llm_service:
            return StringToolOutput(safe_json_dumps({"status": "error", "message": "LLM Service not available for AI review"}))

        plan_dict_for_prompt = intent_plan_obj.to_dict() # Convert object to dict for the prompt

        default_prompt = f"""
        As an AI Safety and Efficacy Inspector, review the following IntentPlan.
        Your goal is to ensure the plan is safe, aligned with its objective, and likely to succeed.

        IntentPlan Details:
        ```json
        {safe_json_dumps(plan_dict_for_prompt)}
        ```

        Review Criteria:
        1.  **Safety & Ethics:** Does any intent pose a risk of harm, data breach, or unethical action?
        2.  **Objective Alignment:** Is each intent and the overall plan clearly aligned with the stated objective: "{intent_plan_obj.objective}"?
        3.  **Parameter Validity:** Are the parameters for each 'EXECUTE' intent complete, correct, and make sense in context?
        4.  **Dependency Correctness:** Are `depends_on` fields logical and correctly specified?
        5.  **Likelihood of Success:** Based on the actions and components, how likely is this plan to achieve the objective?
        6.  **Efficiency:** Could the objective be achieved more simply or with fewer steps? (Optional, if obvious)

        Decision:
        Based on your review, decide whether to:
        -   `APPROVE` the plan if it's sound.
        -   `REJECT` the plan if significant issues are found. Provide clear reasons and suggest critical modifications.

        Output Format (Strict JSON):
        Return your response as a single JSON object with the following fields:
        -   `"status"`: Either "{PlanStatus.APPROVED.value}" or "{PlanStatus.REJECTED.value}".
        -   `"reason"`: A concise overall reason for your decision.
        -   `"overall_risk_assessment"`: (String) "Low", "Medium", or "High".
        -   `"intent_reviews"`: (List of objects) For each intent in the original plan:
            -   `"intent_id"`: (String) The ID of the intent being reviewed.
            -   `"intent_status_assessment"`: (String) "{IntentStatus.APPROVED.value}" (if safe and good) or "{IntentStatus.REJECTED.value}" (if problematic).
            -   `"concerns"`: (List of strings) Specific concerns or reasons for rejection for this intent. Empty if approved.
            -   `"suggestions_for_intent"`: (String, Optional) Specific suggestions for improving this intent if it's problematic.
        -   `"suggestions_for_plan"`: (String, Optional) Overall suggestions for improving the plan if rejected or if minor improvements are noted for an approved plan.
        """
        prompt_to_use = custom_prompt or default_prompt
        review_text = await self.llm_service.generate(prompt_to_use)
        try:
            # Robust JSON extraction
            match = re.search(r"\{[\s\S]*\}", review_text)
            if match:
                json_str = match.group(0)
                review_json = json.loads(json_str)
            else:
                review_json = json.loads(review_text) # Fallback

            review_json["review_source"] = "ai_agent"
            return StringToolOutput(safe_json_dumps(review_json))
        except json.JSONDecodeError as e:
            logger.error(f"AI reviewer response was not valid JSON: {e}. Raw response: {review_text}")
            return StringToolOutput(safe_json_dumps({
                "status": "error", "message": f"Failed to parse AI reviewer response: {e}", "raw_response": review_text
            }))

    async def _interactive_review_plan(self, intent_plan_obj: IntentPlan) -> StringToolOutput:
        # Operates on the IntentPlan object, returns a dict representing the review decision
        # The returned dict can include "approved_plan": modified_plan_obj.to_dict()
        # if interactive modifications were made (status changes are main modifications here).
        print("\n" + "="*60); print("🔍 INTENT PLAN REVIEW 🔍"); print("="*60)
        print(f"\nObjective: {intent_plan_obj.objective}")
        print(f"Plan ID: {intent_plan_obj.plan_id}")
        print(f"\nThe plan contains {len(intent_plan_obj.intents)} intents:")

        for i, intent_obj in enumerate(intent_plan_obj.intents, 1):
            print(f"\n{i}. [{intent_obj.step_type}] {intent_obj.component_name}.{intent_obj.action}")
            if intent_obj.params: print(f"   Parameters: {safe_json_dumps(intent_obj.params, indent=6)}") # Use safe_json_dumps
            print(f"   Justification: {intent_obj.justification}")
            if intent_obj.depends_on: print(f"   Depends on: {', '.join(intent_obj.depends_on)}")

        print("\nReview this plan. Safe to execute?")
        while True:
            choice = input("\nApprove? (y/n/d for details/i for intent details): ").strip().lower()
            if choice == 'd': print(safe_json_dumps(intent_plan_obj.to_dict())); continue
            if choice == 'i':
                try:
                    num_str = input("Enter intent number to inspect: ")
                    num = int(num_str)
                    if 1 <= num <= len(intent_plan_obj.intents):
                        print(safe_json_dumps(intent_plan_obj.intents[num-1].to_dict()))
                    else: print(f"Invalid number (1-{len(intent_plan_obj.intents)}).")
                except ValueError: print("Invalid input.")
                continue
            if choice == 'y':
                comments = input("Optional comments: ").strip()
                # Modify a copy for the return, original object status modified before DB save
                approved_plan_dict = intent_plan_obj.to_dict()
                approved_plan_dict["status"] = PlanStatus.APPROVED.value
                for intent_d in approved_plan_dict.get("intents", []): intent_d["status"] = IntentStatus.APPROVED.value
                return StringToolOutput(safe_json_dumps({
                    "status": PlanStatus.APPROVED.value, "message": "Plan approved by human reviewer.",
                    "comments": comments, "approved_plan": approved_plan_dict
                }))
            if choice == 'n':
                reason = input("Reason for rejection: ").strip()
                return StringToolOutput(safe_json_dumps({
                    "status": PlanStatus.REJECTED.value, "message": "Plan rejected by human reviewer.", "reason": reason
                }))
            print("Invalid choice. y/n/d/i.")

    async def _cli_review_plan(self, intent_plan_obj: IntentPlan) -> StringToolOutput:
        # Similar to interactive, operates on IntentPlan object
        print("\n" + "="*60); print("INTENT PLAN REVIEW (CLI MODE)"); print("="*60)
        # ... (display logic as in _interactive_review_plan using intent_plan_obj) ...
        # For brevity, assume display logic is similar
        print(f"Review Plan ID: {intent_plan_obj.plan_id}. Objective: {intent_plan_obj.objective}")
        print("Enter 'approve' or 'reject [reason]'")
        timeout_seconds = 600; start_time = time.time()
        while time.time() - start_time < timeout_seconds:
            try:
                response = input("Response: ").strip()
                if response.lower().startswith("approve"):
                    approved_plan_dict = intent_plan_obj.to_dict()
                    approved_plan_dict["status"] = PlanStatus.APPROVED.value
                    for intent_d in approved_plan_dict.get("intents", []): intent_d["status"] = IntentStatus.APPROVED.value
                    return StringToolOutput(safe_json_dumps({
                        "status": PlanStatus.APPROVED.value, "approved_plan": approved_plan_dict
                    }))
                elif response.lower().startswith("reject"):
                    reason = response[len("reject"):].strip() or "No reason provided (CLI)"
                    return StringToolOutput(safe_json_dumps({
                        "status": PlanStatus.REJECTED.value, "reason": reason
                    }))
                else: print("Invalid. 'approve' or 'reject [reason]'.")
            except KeyboardInterrupt: return StringToolOutput(safe_json_dumps({"status": "cancelled"}))
            await asyncio.sleep(0.1) # Allow other async tasks
        return StringToolOutput(safe_json_dumps({"status": "timeout"}))