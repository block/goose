# evolving_agents/workflow/process_workflow_tool.py

import logging
import yaml
from typing import Dict, Any, Optional, List, Union
import json
import html # For unescaping HTML entities
import re # For parameter substitution regex
import uuid # For intent IDs
from datetime import datetime, timezone # Use timezone-aware datetimes

# Pydantic for input validation
from pydantic import BaseModel, Field
import pymongo # For pymongo constants if used in index creation

# BeeAI Framework imports for Tool structure
from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

# Import config for intent mode check
from evolving_agents import config # For INTENT_REVIEW_ENABLED, INTENT_REVIEW_LEVELS
from evolving_agents.core.intent_review import IntentPlan, Intent, PlanStatus, IntentStatus # Data classes
from evolving_agents.core.mongodb_client import MongoDBClient # For DB interaction
from evolving_agents.core.dependency_container import DependencyContainer # For resolving MongoDBClient
import asyncio # For async operations

# Import the centralized safe_json_dumps
from evolving_agents.utils.json_utils import safe_json_dumps

logger = logging.getLogger(__name__)

# --- Input Schema ---
class ProcessWorkflowInput(BaseModel):
    """Input schema for the ProcessWorkflowTool."""
    workflow_yaml: str = Field(description="YAML string defining the workflow")
    params: Optional[Dict[str, Any]] = Field(None, description="Parameters to substitute into the workflow (key-value pairs)")
    objective: Optional[str] = Field(None, description="Overall objective of the workflow if not found in params")
    # Optional: to force a specific plan_id if reprocessing or for traceability
    plan_id_override: Optional[str] = Field(None, description="Optional specific ID to use for the IntentPlan")


# --- Tool Implementation ---
class ProcessWorkflowTool(Tool[ProcessWorkflowInput, None, StringToolOutput]):
    """
    Parses and prepares YAML-based workflows. Validates structure, substitutes parameters.
    If Intent Mode is active (globally and for 'intents' level), it converts steps
    into an IntentPlan, saves it to MongoDB, and returns its ID for review.
    Otherwise, it returns a structured list of steps for direct execution by SystemAgent.
    """
    name = "ProcessWorkflowTool"
    description = (
        "Parse, validate, and prepare YAML workflows. "
        "Optionally generates an IntentPlan and saves it to MongoDB for review, "
        "or returns processed steps for direct execution."
    )
    input_schema = ProcessWorkflowInput

    def __init__(self,
                 mongodb_client: Optional[MongoDBClient] = None,
                 container: Optional[DependencyContainer] = None,
                 options: Optional[Dict[str, Any]] = None):
        super().__init__(options=options or {})
        self.container = container
        if mongodb_client:
            self.mongodb_client = mongodb_client
        elif self.container and self.container.has('mongodb_client'):
            self.mongodb_client = self.container.get('mongodb_client')
        else:
            try:
                self.mongodb_client = MongoDBClient() # Assumes MONGODB_URI is in .env
                if self.container and not self.container.has('mongodb_client'):
                    self.container.register('mongodb_client', self.mongodb_client)
                logger.info("ProcessWorkflowTool: Default MongoDBClient instance created.")
            except ValueError as e: # If MONGODB_URI is missing
                logger.warning(f"ProcessWorkflowTool: MongoDBClient FAILED to initialize: {e}. "
                               f"Intent plan saving to DB will be skipped if intent mode is active.")
                self.mongodb_client = None

        if self.mongodb_client:
            self.intent_plans_collection_name = "eat_intent_plans"
            self.intent_plans_collection = self.mongodb_client.get_collection(self.intent_plans_collection_name)
            # Ensure index on plan_id for efficient loading by ApprovePlanTool
            asyncio.create_task(self._ensure_intent_plan_indexes())
        else:
            self.intent_plans_collection = None
            self.intent_plans_collection_name = "N/A (MongoDB unavailable)"

        logger.info(f"ProcessWorkflowTool initialized. MongoDB for intent plans: {self.intent_plans_collection_name}")

    async def _ensure_intent_plan_indexes(self):
        """Ensure MongoDB indexes for the intent_plans collection."""
        if self.intent_plans_collection is not None: 
            try:
                await self.intent_plans_collection.create_index(
                    [("plan_id", pymongo.ASCENDING)], unique=True, background=True
                )
                await self.intent_plans_collection.create_index(
                    [("status", pymongo.ASCENDING)], background=True
                )
                await self.intent_plans_collection.create_index(
                    [("created_at", pymongo.DESCENDING)], background=True 
                ) # Assuming IntentPlan has created_at
                logger.info(f"Ensured indexes on '{self.intent_plans_collection_name}' collection.")
            except Exception as e:
                logger.error(f"Error creating indexes for '{self.intent_plans_collection_name}': {e}", exc_info=True)
        else:
            logger.warning(f"Cannot ensure indexes on '{self.intent_plans_collection_name}' as collection is None.")


    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "workflow", "process"],
            creator=self,
        )

    async def _run(self, tool_input: ProcessWorkflowInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        logger.info("Processing workflow YAML...")
        try:
            # 1. Clean and Load YAML
            cleaned_yaml_string = html.unescape(tool_input.workflow_yaml) 
            
            # More aggressive cleaning for common LLM issues
            cleaned_yaml_string = re.sub(r"```yaml\s*", "", cleaned_yaml_string)
            cleaned_yaml_string = re.sub(r"\s*```", "", cleaned_yaml_string)
            
            lines = cleaned_yaml_string.strip().split('\n')
            start_idx = 0
            for i, line in enumerate(lines):
                stripped_line = line.strip()
                if stripped_line and (stripped_line.split(':')[0] in ["scenario_name", "domain", "description", "steps"] or stripped_line.startswith("- type:")):
                    start_idx = i
                    break
            cleaned_yaml_string = "\n".join(lines[start_idx:])


            logger.debug("Cleaned YAML string (first 500 chars):\n%s", cleaned_yaml_string[:500])

            try:
                workflow = yaml.safe_load(cleaned_yaml_string)
                if not isinstance(workflow, dict):
                    if isinstance(workflow, list) and all(isinstance(s, dict) and "type" in s for s in workflow):
                        logger.warning("YAML loaded as a list of steps. Wrapping with default scenario info.")
                        workflow = {
                            "scenario_name": f"Recovered_Workflow_{uuid.uuid4().hex[:4]}",
                            "domain": "general",
                            "description": "Workflow recovered from a list of steps.",
                            "steps": workflow
                        }
                    else:
                        raise ValueError(f"Workflow YAML must load as a dictionary or a list of valid step dictionaries. Got type: {type(workflow)}")
                if "steps" not in workflow or not isinstance(workflow.get("steps"), list):
                    raise ValueError("Invalid workflow: Missing 'steps' list or 'steps' is not a list.")
            except yaml.YAMLError as e:
                error_mark = getattr(e, 'problem_mark', None)
                line_num_str = str(error_mark.line + 1) if error_mark else 'unknown'
                problem = getattr(e, 'problem', str(e))
                context_msg = getattr(e, 'context', '')
                snippet_start = max(0, error_mark.index - 40) if error_mark else 0
                snippet_end = min(len(cleaned_yaml_string), (error_mark.index + 40) if error_mark else 80)
                snippet = cleaned_yaml_string[snippet_start:snippet_end]
                raise ValueError(f"Error parsing YAML near line {line_num_str}: {problem}. {context_msg}. Snippet: ...{snippet}...") from e
            logger.info(f"Successfully parsed YAML for workflow '{workflow.get('scenario_name', 'N/A')}'")


            # 2. Substitute Parameters
            params_dict = tool_input.params if isinstance(tool_input.params, dict) else {}
            processed_workflow_steps = self._substitute_params_recursive(workflow["steps"], params_dict)
            logger.info("Parameter substitution complete.")


            # 3. Determine if Intent Mode is Active for 'intents' level
            intent_mode_active_for_intents = False
            if context and hasattr(context, "get_value") and context.get_value("intent_review_mode_override", False):
                intent_mode_active_for_intents = True
                logger.debug("Intent mode FORCED by context override for this ProcessWorkflowTool run.")
            elif config.INTENT_REVIEW_ENABLED:
                 intent_mode_active_for_intents = "intents" in getattr(config, "INTENT_REVIEW_LEVELS", [])
            logger.debug(f"Intent mode active for 'intents' level: {intent_mode_active_for_intents}")


            # 4. Extract Objective
            objective = tool_input.objective or \
                        params_dict.get("objective",
                            params_dict.get("user_request",
                                workflow.get("description", "Objective not explicitly specified.")))
            logger.debug(f"Determined objective for IntentPlan: '{objective}'")


            # 5. Validate Steps / Convert to Intent Plan
            validated_output = self._validate_steps(
                processed_workflow_steps,
                intent_mode=intent_mode_active_for_intents, 
                objective=objective,
                workflow_name=workflow.get("scenario_name", "Unnamed Workflow"),
                plan_id_override=tool_input.plan_id_override 
            )
            log_msg_suffix = ""
            if intent_mode_active_for_intents and isinstance(validated_output, IntentPlan):
                log_msg_suffix = f" (IntentPlan '{validated_output.plan_id}' generated)"
            logger.info(f"Workflow steps processed.{log_msg_suffix}")


            # 6. Handle Output Based on Mode
            if isinstance(validated_output, IntentPlan): 
                intent_plan_obj = validated_output
                intent_plan_dict = intent_plan_obj.to_dict()

                if self.intent_plans_collection is not None: 
                    try:
                        await self.intent_plans_collection.replace_one(
                            {"plan_id": intent_plan_obj.plan_id},
                            intent_plan_dict,
                            upsert=True
                        )
                        logger.info(f"IntentPlan '{intent_plan_obj.plan_id}' saved/updated in MongoDB.")
                        return_payload = {
                            "status": "intent_plan_created",
                            "message": "Intent plan created and saved to MongoDB. Review is required before execution.",
                            "plan_id": intent_plan_obj.plan_id
                        }
                    except Exception as db_err:
                        logger.error(f"Failed to save IntentPlan '{intent_plan_obj.plan_id}' to MongoDB: {db_err}", exc_info=True)
                        return_payload = {
                            "status": "intent_plan_created_db_error",
                            "message": f"Intent plan created but FAILED to save to MongoDB: {db_err}. Pass this full plan to review tool.",
                            "plan_id": intent_plan_obj.plan_id,
                            "intent_plan": intent_plan_dict # include full plan if DB save failed
                        }
                else: 
                    logger.warning("MongoDB client/collection not available. IntentPlan was generated but not saved to database.")
                    return_payload = {
                        "status": "intent_plan_created_no_db",
                        "message": "Intent plan created (MongoDB unavailable). Pass this full plan to review tool.",
                        "plan_id": intent_plan_obj.plan_id,
                        "intent_plan": intent_plan_dict # include full plan if no DB
                    }
                
                if context and hasattr(context, "set_value"):
                    try:
                        # Save the potentially modified intent_plan_dict to context
                        context.set_value("intent_plan_json_output", safe_json_dumps(intent_plan_dict))
                        logger.debug(f"Full IntentPlan '{intent_plan_obj.plan_id}' also saved to run context.")
                    except Exception as ctx_err:
                        logger.warning(f"Failed to save full IntentPlan to context: {ctx_err}")

                return StringToolOutput(safe_json_dumps(return_payload))

            elif isinstance(validated_output, list): 
                plan_for_direct_execution = {
                    "status": "success",
                    "scenario_name": workflow.get("scenario_name", "Unnamed Workflow"),
                    "domain": workflow.get("domain", "general"),
                    "description": workflow.get("description", "Workflow for direct execution"),
                    "steps": validated_output, 
                    "execution_guidance": "The SystemAgent should now execute these steps sequentially."
                }
                return StringToolOutput(safe_json_dumps(plan_for_direct_execution))
            else:
                raise TypeError(f"Unexpected output type from _validate_steps: {type(validated_output)}. Expected list or IntentPlan.")

        except Exception as e:
            import traceback
            logger.error(f"Error processing workflow in _run: {e}", exc_info=True)
            return StringToolOutput(safe_json_dumps({
                "status": "error",
                "message": f"Error processing workflow: {str(e)}",
                "details": traceback.format_exc()
            }))

    def _substitute_params_recursive(self, obj: Any, params: Dict[str, Any]) -> Any:
        placeholder_pattern = r'{{\s*params\.([\w_]+)\s*}}'

        if isinstance(obj, dict):
            return {
                self._substitute_params_recursive(k, params) if isinstance(k, str) else k:
                self._substitute_params_recursive(v, params)
                for k, v in obj.items()
            }
        elif isinstance(obj, list):
            return [self._substitute_params_recursive(item, params) for item in obj]
        elif isinstance(obj, str):
            exact_match = re.fullmatch(placeholder_pattern, obj.strip())
            if exact_match:
                key = exact_match.group(1)
                if key in params:
                    logger.debug(f"Substituting exact placeholder '{obj.strip()}' with param '{key}' (type: {type(params[key])}).")
                    return params[key]
                else:
                    logger.warning(f"Parameter '{key}' for exact match placeholder '{obj.strip()}' not found. Returning original placeholder string.")
                    return obj 

            def replace_match_in_string(match_obj):
                key = match_obj.group(1)
                if key in params:
                    return str(params[key])
                else:
                    logger.warning(f"Parameter '{key}' for embedded placeholder '{match_obj.group(0)}' not found. Leaving placeholder.")
                    return match_obj.group(0) 

            return re.sub(placeholder_pattern, replace_match_in_string, obj)
        else:
            return obj


    def _convert_steps_to_intent_plan_obj(self, workflow_steps: List[Dict[str, Any]], objective: str, workflow_name: str, plan_id_override: Optional[str]) -> IntentPlan:
        plan_id = plan_id_override or f"plan_{uuid.uuid4().hex[:10]}" 
        logger.info(f"Converting {len(workflow_steps)} steps to IntentPlan object '{plan_id}' for workflow '{workflow_name}'")

        intents_list: List[Intent] = []
        step_index_to_intent_id: Dict[int, str] = {} 
        current_time_iso = datetime.now(timezone.utc).isoformat()

        for i, step_dict in enumerate(workflow_steps):
            intent_id = f"intent_{plan_id}_{i+1}_{uuid.uuid4().hex[:6]}" 
            step_index_to_intent_id[i] = intent_id

            step_type_str = step_dict.get("type", "UNKNOWN_STEP_TYPE")
            params_or_input_dict: Dict[str, Any] = {}
            if step_type_str == "EXECUTE": params_or_input_dict = step_dict.get("input", {})
            elif step_type_str in ["DEFINE", "CREATE"]:
                core_keys = {"type", "item_type", "name", "description", "code_snippet", "output_var", "from_existing_snippet", "config"}
                params_or_input_dict = {k: v for k, v in step_dict.items() if k not in core_keys}
                if "config" in step_dict: params_or_input_dict["config"] = step_dict["config"]
            elif step_type_str == "RETURN": params_or_input_dict = {"value": step_dict.get("value")}

            depends_on_ids: List[str] = []
            # Use the centralized safe_json_dumps for scanning parameters
            params_str_for_scan = safe_json_dumps(params_or_input_dict, indent=None) # No indent needed for scanning
            step_var_pattern = r'{{\s*([\w_]+)\s*}}' 
            
            found_step_vars = set(re.findall(step_var_pattern, params_str_for_scan))
            
            for var_name in found_step_vars:
                for prev_step_idx, prev_step_dict in enumerate(workflow_steps[:i]): 
                    if prev_step_dict.get("output_var") == var_name:
                        if prev_step_idx in step_index_to_intent_id:
                            depends_on_ids.append(step_index_to_intent_id[prev_step_idx])
                            logger.debug(f"Intent '{intent_id}' depends on Intent '{step_index_to_intent_id[prev_step_idx]}' via output_var '{var_name}'.")
                            break 
                        else: 
                            logger.warning(f"Could not find mapped Intent ID for previous step index {prev_step_idx} defining '{var_name}'.")

            intent_obj = Intent(
                intent_id=intent_id,
                step_type=step_type_str,
                component_type=step_dict.get("item_type", "GENERIC_COMPONENT"),
                component_name=step_dict.get("name", "UnnamedComponent"),
                action=step_dict.get("action", step_type_str.lower()), 
                params=params_or_input_dict,
                justification=step_dict.get("description", f"Execute step {i+1} of type {step_type_str}"),
                depends_on=sorted(list(set(depends_on_ids))),
                status=IntentStatus.PENDING
            )
            intents_list.append(intent_obj)

        return IntentPlan(
            plan_id=plan_id,
            title=f"Intent Plan: {workflow_name}",
            description=f"Generated for objective: {objective}",
            objective=objective,
            intents=intents_list,
            status=PlanStatus.PENDING_REVIEW,
            created_at=current_time_iso # Initialize created_at
        )

    def _validate_steps(self, steps: List[Dict[str, Any]], intent_mode: bool = False,
                        objective: Optional[str] = "Not specified.",
                        workflow_name: Optional[str] = "Unnamed Workflow",
                        plan_id_override: Optional[str] = None
                        ) -> Union[List[Dict[str, Any]], IntentPlan]:
        if not isinstance(steps, list):
             raise ValueError("Workflow 'steps' must be a list.")

        validated_steps_list = []
        current_objective = objective or "Objective not explicitly provided." 

        for i, step in enumerate(steps):
            step_num = i + 1
            if not isinstance(step, dict): raise ValueError(f"Step {step_num} is not a dictionary: {step}")

            step_type = step.get("type")
            if not step_type or not isinstance(step_type, str):
                raise ValueError(f"Step {step_num} missing or invalid 'type' field: {step}")

            valid_types = ["DEFINE", "CREATE", "EXECUTE", "RETURN"]
            if step_type not in valid_types:
                raise ValueError(f"Step {step_num} invalid type '{step_type}'. Must be one of {valid_types}.")

            required_keys = set()
            if step_type == "DEFINE": required_keys = {"item_type", "name", "description"} # code_snippet is often generated
            elif step_type == "CREATE": required_keys = {"item_type", "name"}
            elif step_type == "EXECUTE": required_keys = {"item_type", "name"}
            elif step_type == "RETURN": required_keys = {"value"}

            present_keys = set(step.keys())
            missing_keys = required_keys - present_keys
            if missing_keys:
                 raise ValueError(f"Step {step_num} (type: {step_type}) missing required keys: {missing_keys}. Step: {step}")

            if step_type == "EXECUTE" and "input" in step and not isinstance(step["input"], dict):
                 logger.error(f"Validation Error in Step {step_num}: 'input' field is not a dictionary. Found type: {type(step['input'])}. Step content: {step}")
                 raise ValueError(f"Step {step_num} (EXECUTE): 'input' field MUST be a dictionary, but found type {type(step['input'])}.")

            if intent_mode and current_objective == "Objective not explicitly provided." and \
               step_type == "EXECUTE" and isinstance(step.get("input"), dict):
                input_params = step["input"]
                for key, value in input_params.items():
                    if isinstance(key, str) and key.lower() in ["objective", "goal", "user_request", "task_description"]:
                        if isinstance(value, str) and value:
                            current_objective = value
                            logger.debug(f"Objective updated from step {step_num} input: '{current_objective}'")
                            break
            validated_steps_list.append(step)

        if intent_mode:
            logger.info("Intent mode active, converting validated steps to IntentPlan object.")
            return self._convert_steps_to_intent_plan_obj(validated_steps_list, current_objective, workflow_name, plan_id_override)
        else:
            logger.info("Returning validated steps list for direct execution.")
            return validated_steps_list