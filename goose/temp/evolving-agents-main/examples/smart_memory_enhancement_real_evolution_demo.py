# examples/smart_memory_enhancement_real_evolution_demo.py

import asyncio
import logging
import os
import sys
import json
import uuid
from typing import Dict, Any, Optional, List
from dotenv import load_dotenv
from rich.console import Console
from rich.panel import Panel
from rich.syntax import Syntax
from rich import print as rprint
import re

# --- Evolving Agents Imports ---
# Add project root to sys.path
script_dir = os.path.dirname(os.path.abspath(__file__))
project_root = os.path.dirname(os.path.dirname(script_dir))
sys.path.insert(0, project_root)

from evolving_agents.core.llm_service import LLMService
from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.agent_bus.smart_agent_bus import SmartAgentBus
from evolving_agents.core.system_agent import SystemAgentFactory
from evolving_agents.core.dependency_container import DependencyContainer
from evolving_agents.firmware.firmware import Firmware
from evolving_agents.core.mongodb_client import MongoDBClient
from evolving_agents import config as eat_config
from evolving_agents.utils.json_utils import safe_json_dumps

# Memory System Components
from evolving_agents.agents.memory_manager_agent import MemoryManagerAgent
from evolving_agents.tools.internal.mongo_experience_store_tool import MongoExperienceStoreTool
from evolving_agents.tools.internal.semantic_experience_search_tool import SemanticExperienceSearchTool
from evolving_agents.tools.internal.message_summarization_tool import MessageSummarizationTool
from beeai_framework.memory import UnconstrainedMemory

# --- Configuration ---
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - [%(module)s.%(funcName)s:%(lineno)d] - %(message)s'
)
# Suppress some verbose logs from dependencies for cleaner demo output
logging.getLogger("httpx").setLevel(logging.WARNING)
logging.getLogger("openai").setLevel(logging.WARNING)
logging.getLogger("litellm").setLevel(logging.WARNING)
logging.getLogger("asyncio").setLevel(logging.WARNING)
logging.getLogger("SmartAgentBus").setLevel(logging.INFO) # Keep SAB logs for demo
logging.getLogger("SmartLibrary").setLevel(logging.INFO) # Keep SL logs for demo
logging.getLogger("LLMCache").setLevel(logging.INFO)


logger = logging.getLogger(__name__) # Logger for this script
load_dotenv()

console = Console(width=120)

# --- Constants ---
MEMORY_MANAGER_AGENT_ID = "memory_manager_agent_real_evo_demo"
DEMO_OUTPUT_FILE = "smart_memory_real_evolution_demo_results.json"
SMART_MEMORY_DEMO_TAG = "smart_memory_real_evo_demo_component"

# --- Helper Functions ---
async def setup_framework_environment() -> DependencyContainer:
    """Initializes all core framework services and agents with MongoDB backend."""
    console.print("\n[bold blue]OPERATION: Initializing Framework Environment (MongoDB Backend)[/]")
    container = DependencyContainer()

    # MongoDB Client
    try:
        mongo_uri = eat_config.MONGODB_URI
        mongo_db_name = eat_config.MONGODB_DATABASE_NAME
        if not mongo_uri or not mongo_db_name:
            raise ValueError("MONGODB_URI or MONGODB_DATABASE_NAME not configured.")
        
        mongodb_client_instance = MongoDBClient(uri=mongo_uri, db_name=mongo_db_name)
        if not await mongodb_client_instance.ping_server():
            raise ConnectionError("MongoDB server ping failed.")
        container.register('mongodb_client', mongodb_client_instance)
        console.print(f"  [green]✓[/] MongoDB Client initialized for DB: '{mongodb_client_instance.db_name}'")
    except Exception as e:
        console.print(f"  [bold red]✗ ERROR: Failed to initialize MongoDBClient: {e}[/]")
        raise

    # LLM Service
    llm_service = LLMService(
        provider=eat_config.LLM_PROVIDER, model=eat_config.LLM_MODEL,
        embedding_model=eat_config.LLM_EMBEDDING_MODEL, use_cache=eat_config.LLM_USE_CACHE,
        mongodb_client=mongodb_client_instance, container=container
    )
    container.register('llm_service', llm_service)

    # MemoryManagerAgent and its tools
    experience_store_tool = MongoExperienceStoreTool(mongodb_client_instance, llm_service)
    semantic_search_tool = SemanticExperienceSearchTool(mongodb_client_instance, llm_service)
    message_summarization_tool = MessageSummarizationTool(llm_service)
    memory_manager_agent = MemoryManagerAgent(
        llm_service, experience_store_tool, semantic_search_tool,
        message_summarization_tool, UnconstrainedMemory()
    )
    container.register('memory_manager_agent_instance', memory_manager_agent) # Register instance for bus

    # Core EAT Components
    smart_library = SmartLibrary(llm_service=llm_service, container=container)
    container.register('smart_library', smart_library)
    firmware = Firmware(); container.register('firmware', firmware)
    agent_bus = SmartAgentBus(container=container)
    container.register('agent_bus', agent_bus)
    system_agent = await SystemAgentFactory.create_agent(container=container)
    container.register('system_agent', system_agent)
    
    # Register MemoryManagerAgent on the bus
    await agent_bus.register_agent(
        agent_id=MEMORY_MANAGER_AGENT_ID, name="MemoryManagerAgent",
        description="Manages agent experiences for learning and context building. Call via 'process_task' capability with a natural language description of the memory operation needed.",
        agent_type="MemoryManagement",
        capabilities=[{"id": "process_task", "name": "Process Memory Task", "description": "Handles storage, retrieval, and summarization of experiences/messages."}],
        agent_instance=memory_manager_agent,
        embed_capabilities=True
    )
    console.print(f"  [green]✓[/] MemoryManagerAgent registered on SmartAgentBus with ID: {MEMORY_MANAGER_AGENT_ID}.")

    await smart_library.initialize()
    console.print("[green]✓[/] Framework environment fully initialized.")
    return container

async def clean_demo_data(container: DependencyContainer):
    """Cleans up demo-specific data from MongoDB and local files."""
    console.print("\n[bold blue]OPERATION: Cleaning up previous demo data...[/]")
    smart_library: SmartLibrary = container.get('smart_library')
    mongodb_client: MongoDBClient = container.get('mongodb_client')

    # Clean SmartLibrary components tagged for this demo
    if smart_library.components_collection:
        delete_result_lib = await smart_library.components_collection.delete_many({"tags": SMART_MEMORY_DEMO_TAG})
        console.print(f"  - Cleaned {delete_result_lib.deleted_count} demo components from SmartLibrary.")

    experience_collection_name = "eat_agent_experiences"
    if mongodb_client.db:
        experience_collection = mongodb_client.db[experience_collection_name]
        # To avoid clearing ALL experiences, let's try to filter by ones created in this demo.
        # This is tricky without a specific demo_id in experiences. We can filter by initiator_agent_id if it's unique.
        # Or, if we know the primary_goal_description pattern.
        # For simplicity in this demo, we will still clear all if not careful.
        # Let's filter by a tag if `ExperienceRecorderTool` adds tags from the experience_to_record.
        # The `simulate_system_agent_task_execution` function includes SMART_MEMORY_DEMO_TAG in experience_to_record.
        delete_result_exp = await experience_collection.delete_many({"tags": SMART_MEMORY_DEMO_TAG})
        console.print(f"  - Cleaned {delete_result_exp.deleted_count} demo-tagged entries from '{experience_collection_name}'.")
    
    if os.path.exists(DEMO_OUTPUT_FILE):
        os.remove(DEMO_OUTPUT_FILE)
        console.print(f"  - Removed local output file: {DEMO_OUTPUT_FILE}")
    console.print("[green]✓[/] Demo data cleanup complete.")

async def seed_initial_components(smart_library: SmartLibrary):
    console.print("\n[bold blue]OPERATION: Seeding initial components into SmartLibrary...[/]")
    
    components_to_seed = [
        {
            "name": "GenericTicketRouterAgent_v1_RealEvo", "record_type": "AGENT", "domain": "support",
            "description": "A generic BeeAI agent that attempts to route tickets based on keywords. Uses basic keyword matching. This is the initial version to be evolved.",
            "code_snippet": """
from typing import List, Dict, Any, Optional
from beeai_framework.agents.react import ReActAgent
from beeai_framework.agents.types import AgentMeta
from beeai_framework.memory import TokenMemory
from beeai_framework.backend.chat import ChatModel
from beeai_framework.tools.tool import Tool

class GenericTicketRouterAgent_v1_RealEvoInitializer:
    @staticmethod
    def create_agent(llm: ChatModel, tools: Optional[List[Tool]] = None) -> ReActAgent:
        meta = AgentMeta(
            name="GenericTicketRouterAgent_v1_RealEvo",
            description="I am a generic ticket router. I analyze ticket text and decide which specialized tool to route it to: TechSupportTool_v1_RealEvo, BillingInquiryTool_v1_RealEvo, or GeneralQueueTool_v1_RealEvo.",
            tools=tools or [] # SystemAgent should provide relevant tools
        )
        agent = ReActAgent(llm=llm, tools=tools or [], memory=TokenMemory(llm), meta=meta)
        return agent
            """, "tags": ["routing", "support", "generic", "beeai", SMART_MEMORY_DEMO_TAG],
            "metadata": {"framework": "beeai"}
        },
        {
            "name": "TechSupportTool_v1_RealEvo", "record_type": "TOOL", "domain": "support",
            "description": "Tool for handling common technical support issues like password resets.",
            "code_snippet": """
from pydantic import BaseModel, Field; from beeai_framework.tools.tool import Tool, StringToolOutput
class Input(BaseModel): issue_description: str
class TechSupportTool_v1_RealEvo(Tool[Input, None, StringToolOutput]):
    name="TechSupportTool_v1_RealEvo"; description="Handles tech issues."
    input_schema=Input
    async def _run(self, input: Input, options=None, context=None) -> StringToolOutput:
        if "password reset" in input.issue_description.lower(): return StringToolOutput("Password reset instructions sent to user.")
        return StringToolOutput(f"Technical issue '{input.issue_description}' acknowledged. A specialist will contact the user.")
            """, "tags": ["technical", "support", "beeai", SMART_MEMORY_DEMO_TAG],
            "metadata": {"framework": "beeai"}
        },
        {
            "name": "BillingInquiryTool_v1_RealEvo", "record_type": "TOOL", "domain": "finance",
            "description": "Tool for answering billing inquiries.",
            "code_snippet": """
from pydantic import BaseModel, Field; from beeai_framework.tools.tool import Tool, StringToolOutput
class Input(BaseModel): inquiry_text: str
class BillingInquiryTool_v1_RealEvo(Tool[Input, None, StringToolOutput]):
    name="BillingInquiryTool_v1_RealEvo"; description="Handles billing inquiries."
    input_schema=Input
    async def _run(self, input: Input, options=None, context=None) -> StringToolOutput:
        if "overcharge" in input.inquiry_text.lower(): return StringToolOutput("User claims overcharge. Please provide invoice number and details of the overcharge for investigation.")
        return StringToolOutput(f"Billing inquiry '{input.inquiry_text}' received. Our team will review your account and get back to you.")
            """, "tags": ["billing", "finance", "beeai", SMART_MEMORY_DEMO_TAG],
            "metadata": {"framework": "beeai"}
        },
        {
            "name": "GeneralQueueTool_v1_RealEvo", "record_type": "TOOL", "domain": "support",
            "description": "A fallback tool for issues not handled by specialized tools.",
            "code_snippet": """
from pydantic import BaseModel, Field; from beeai_framework.tools.tool import Tool, StringToolOutput
class Input(BaseModel): ticket_text: str
class GeneralQueueTool_v1_RealEvo(Tool[Input, None, StringToolOutput]):
    name="GeneralQueueTool_v1_RealEvo"; description="Fallback queue for tickets."
    input_schema=Input
    async def _run(self, input: Input, options=None, context=None) -> StringToolOutput:
        return StringToolOutput(f"Ticket '{input.ticket_text[:30]}...' added to general support queue.")
            """, "tags": ["general", "queue", "beeai", SMART_MEMORY_DEMO_TAG],
            "metadata": {"framework": "beeai"}
        }
    ]
    for comp_data in components_to_seed:
        await smart_library.create_record(**comp_data)
    console.print(f"  [green]✓[/] Seeded {len(components_to_seed)} initial components for the real evolution demo.")

def extract_final_json_answer(response_text: str) -> Optional[Dict[str, Any]]:
    """Extracts the JSON object that an agent often provides as its 'Final Answer:'."""
    # Try to find ```json ... ``` blocks first
    json_block_match = re.search(r"```json\s*(\{[\s\S]*?\})\s*```", response_text, re.MULTILINE | re.DOTALL)
    if json_block_match:
        try:
            return json.loads(json_block_match.group(1).strip())
        except json.JSONDecodeError:
            logger.warning("Found JSON block but failed to parse in extract_final_json_answer.")
            
    # If no block, try to find a JSON object, often preceded by "Final Answer:" or "Output:"
    # This regex tries to find a JSON object that might be the last significant part of the text.
    # It looks for '{' and tries to match until the corresponding '}'.
    # This is heuristic and might not be perfect.
    potential_json_match = re.search(r"(?i)(?:Final Answer:|Output:|Here is the JSON output:|Response:)\s*(\{[\s\S]*\})(?:\s*```)?\s*$", response_text, re.MULTILINE | re.DOTALL)
    if potential_json_match:
        try:
            return json.loads(potential_json_match.group(1).strip())
        except json.JSONDecodeError:
            logger.warning("Found potential final JSON but failed to parse in extract_final_json_answer.")
            
    # If still no JSON, and the entire response might be JSON
    try:
        # A simple check: if it starts with { and ends with }
        if response_text.strip().startswith("{") and response_text.strip().endswith("}"):
            return json.loads(response_text.strip())
    except json.JSONDecodeError:
        pass # Not a simple JSON string

    logger.warning(f"Could not extract final JSON answer from response: {response_text[:200]}...")
    return None


async def run_system_agent_task_and_record_experience(
    system_agent,
    task_id: str, # For experience recording
    task_description_for_agent: str, # The actual prompt for SystemAgent
    primary_goal_for_memory: str, # For structuring the experience
    sub_task_for_memory: str # For structuring the experience
) -> Dict[str, Any]:
    """
    Runs a task with SystemAgent and records the experience.
    Returns the final JSON output from SystemAgent for the task, and the experience ID.
    """
    console.print(f"\n[bold cyan]Executing SystemAgent Task:[/] {primary_goal_for_memory} - {sub_task_for_memory}")
    rprint(Panel(f"[italic]Prompting SystemAgent with:\n{task_description_for_agent[:400]}...[/]",
                 title=f"SystemAgent Task: {task_id}", border_style="magenta"))

    start_time = asyncio.get_event_loop().time()
    agent_response_obj = await system_agent.run(task_description_for_agent)
    duration_ms = (asyncio.get_event_loop().time() - start_time) * 1000
    
    agent_response_text = agent_response_obj.result.text if hasattr(agent_response_obj.result, 'text') else str(agent_response_obj)
    logger.info(f"Raw response from SystemAgent for task '{task_id}':\n{agent_response_text[:500]}...")

    final_agent_output_json = extract_final_json_answer(agent_response_text)
    
    outcome_status = "success" if final_agent_output_json and "error" not in final_agent_output_json else "failure"
    if final_agent_output_json and "error" in final_agent_output_json:
        output_summary_for_memory = f"Error processing: {final_agent_output_json['error']}"
    elif final_agent_output_json:
        output_summary_for_memory = f"Successfully processed. Output keys: {list(final_agent_output_json.keys())}"
    else:
        output_summary_for_memory = f"Processing completed. Raw output (first 100 chars): {agent_response_text[:100]}"
        outcome_status = "unknown_output_format"


    # Simplified extraction of components used (this would be complex in a real ReAct trace)
    # For demo, we might infer or SystemAgent might be prompted to list them.
    # Let's assume SystemAgent's thoughts might mention tools/agents it used.
    # This is a very simplified parsing for demo.
    components_used_in_task = list(set(re.findall(r"Using tool: (\w+Tool)", agent_response_text) + 
                                       re.findall(r"Calling agent: (\w+Agent)", agent_response_text)))
    if not components_used_in_task and "GenericTicketRouterAgent_v1_RealEvo" in primary_goal_for_memory: # Heuristic for initial tasks
        components_used_in_task.append("GenericTicketRouterAgent_v1_RealEvo")
    if not components_used_in_task and "SpecializedBillingRouterAgent_v1_RealEvo" in primary_goal_for_memory:
        components_used_in_task.append("SpecializedBillingRouterAgent_v1_RealEvo")


    experience_data = {
        "primary_goal_description": primary_goal_for_memory,
        "sub_task_description": sub_task_for_memory,
        "initiating_agent_id": "SystemAgent_RealEvoDemo",
        "components_used": components_used_in_task or ["Unknown_or_Direct_LLM"],
        "input_context_summary": task_description_for_agent[:200] + "...",
        "final_outcome": outcome_status,
        "output_summary": output_summary_for_memory,
        "key_decisions_made": [f"SystemAgent decided to handle: {task_id}"], # Simplified
        "tags": ["real_evo_demo", SMART_MEMORY_DEMO_TAG, task_id.split('_')[0]] # e.g., "billing", "tech"
    }

    console.print(f"  Attempting to record experience for task '{task_id}'...")
    record_experience_prompt = f"""
    Please record the following task experience using your ExperienceRecorderTool.
    Experience data (JSON format): {safe_json_dumps(experience_data)}
    Respond ONLY with the JSON output from the ExperienceRecorderTool.
    """
    # This specific call is to SystemAgent, asking IT to use ITS tool.
    recorder_response_obj = await system_agent.run(record_experience_prompt)
    recorder_response_text = recorder_response_obj.result.text if hasattr(recorder_response_obj.result, 'text') else str(recorder_response_obj)
    
    experience_id = None
    try:
        recorder_json = json.loads(recorder_response_text)
        if recorder_json.get("status") == "success" and recorder_json.get("experience_id"):
            experience_id = recorder_json["experience_id"]
            console.print(f"  [green]✓[/] Experience for task '{task_id}' recorded. Experience ID: {experience_id}")
        else:
            console.print(f"  [yellow]⚠[/] ExperienceRecorderTool reported an issue for task '{task_id}': {recorder_json.get('message')}")
    except json.JSONDecodeError:
        console.print(f"  [red]✗[/] Failed to parse ExperienceRecorderTool response for task '{task_id}'. Raw: {recorder_response_text[:100]}")

    return {"task_id": task_id, "agent_final_output_json": final_agent_output_json, "agent_raw_output": agent_response_text, "recorded_experience_id": experience_id, "duration_ms": duration_ms}

# --- Main Demo Function ---
async def run_real_smart_memory_demo():
    console.print(Panel.fit(
        "[bold yellow]EAT Smart Memory & Real Evolution Demo (MongoDB Backend)[/]",
        border_style="yellow", padding=(1, 2)
    ))
    # ... (intro prints from previous version)

    container = await setup_framework_environment()
    await clean_demo_data(container)
    await seed_initial_components(container.get('smart_library'))
    
    system_agent = container.get('system_agent')
    all_demo_outputs = {"scenarios": []}

    # --- SCENARIO 1: Initial Suboptimal Billing Ticket Processing ---
    console.print("\n[bold green_yellow]--- SCENARIO 1: Initial Billing Ticket (Processed by Generic Router) ---[/]")
    task1_primary_goal = "Process a customer billing inquiry using GenericTicketRouterAgent_v1_RealEvo."
    task1_sub_task = "Customer was overcharged on invoice #INV789. Correct routing is to BillingInquiryTool_v1_RealEvo."
    task1_description_for_agent = f"""
    A customer reports: "I was overcharged on my last invoice #INV789. Please help."
    Your goal is to process this support ticket.
    1. Use ContextBuilderTool to understand if there are any relevant past experiences or components for 'customer billing overcharge inquiry'.
    2. Based on the context, select the most appropriate AGENT for routing this ticket (e.g., 'GenericTicketRouterAgent_v1_RealEvo').
    3. Then, use RequestAgentTool to execute the chosen routing agent with the ticket text "I was overcharged on my last invoice #INV789."
    4. The routing agent should decide which specialized tool to use (e.g., BillingInquiryTool_v1_RealEvo, TechSupportTool_v1_RealEvo, GeneralQueueTool_v1_RealEvo).
    Return a JSON object with the final routing decision and the response from the specialized tool.
    Example JSON output: {{"routing_agent_used": "...", "specialized_tool_used": "...", "tool_response": "...", "ticket_analysis": "..."}}
    """
    s1_result_data = await run_system_agent_task_and_record_experience(
        system_agent, "billing_ticket_initial", task1_description_for_agent, task1_primary_goal, task1_sub_task
    )
    all_demo_outputs["scenarios"].append(s1_result_data)
    # Expected: GenericTicketRouterAgent_v1_RealEvo might misroute this to GeneralQueueTool_v1_RealEvo or TechSupport if its logic is basic.
    # The experience record should capture this.

    # --- SCENARIO 2: Evolution of the Router Agent, Informed by Past Experience ---
    console.print("\n[bold yellow_green]--- SCENARIO 2: Evolve Generic Router for Better Billing Handling ---[/]")
    # SystemAgent needs to:
    # 1. Use ContextBuilderTool to understand why past billing inquiries were suboptimal.
    # 2. Use this context to formulate changes for EvolveComponentTool.
    # 3. Evolve 'GenericTicketRouterAgent_v1_RealEvo' into 'SpecializedBillingRouterAgent_v1_RealEvo'.
    
    task2_primary_goal = "Evolve GenericTicketRouterAgent_v1_RealEvo based on past performance for billing inquiries."
    task2_sub_task = "Create SpecializedBillingRouterAgent_v1_RealEvo that correctly routes billing to BillingInquiryTool_v1_RealEvo."
    task2_description_for_agent = f"""
    The component 'GenericTicketRouterAgent_v1_RealEvo' has shown suboptimal performance in routing billing-related tickets.
    For example, a ticket like "I was overcharged on my last invoice #INV789" might have been misrouted.
    
    Your task is to improve this:
    1. First, use your ContextBuilderTool to retrieve any experiences related to 'GenericTicketRouterAgent_v1_RealEvo' and 'billing ticket routing'.
    2. Analyze this context. If it confirms misrouting for billing tickets (e.g., not using 'BillingInquiryTool_v1_RealEvo'), proceed.
    3. Use EvolveComponentTool to evolve 'GenericTicketRouterAgent_v1_RealEvo'.
       - The new agent should be named 'SpecializedBillingRouterAgent_v1_RealEvo'.
       - Changes: Modify its routing logic. Specifically, if the ticket text contains keywords like "billing", "invoice", or "overcharge", it MUST route to 'BillingInquiryTool_v1_RealEvo'. For technical issues (e.g., "login", "password"), it should route to 'TechSupportTool_v1_RealEvo'. All other tickets should go to 'GeneralQueueTool_v1_RealEvo'.
       - The evolved agent should still be a BeeAI framework agent.
    Return ONLY the raw JSON output from the EvolveComponentTool, which includes the 'evolved_id'.
    """
    s2_result_data = await run_system_agent_task_and_record_experience( # Using this helper for consistency, though it's not a typical "task execution"
        system_agent, "evolve_router", task2_description_for_agent, task2_primary_goal, task2_sub_task
    )
    all_demo_outputs["scenarios"].append(s2_result_data)
    
    evolved_agent_id = None
    evolved_agent_name = "SpecializedBillingRouterAgent_v1_RealEvo" # Expected name
    if s2_result_data.get("agent_final_output_json") and s2_result_data["agent_final_output_json"].get("status") == "success":
        evolved_agent_id = s2_result_data["agent_final_output_json"].get("evolved_id")
        actual_evolved_name = s2_result_data["agent_final_output_json"].get("evolved_record", {}).get("name")
        if actual_evolved_name: evolved_agent_name = actual_evolved_name # Update if LLM named it differently
        console.print(f"  [green]✓[/] Evolution seems successful. Evolved agent ID: {evolved_agent_id}, Name: {evolved_agent_name}")
    else:
        console.print(f"  [red]✗[/] Evolution step did not return expected success JSON. Check SystemAgent logs and output.")
        console.print(f"    Raw output from evolution attempt: {s2_result_data.get('agent_raw_output', '')[:300]}...")


    # --- SCENARIO 3: Process Another Billing Ticket, Now with Memory and Evolved Agent ---
    console.print("\n[bold green_yellow]--- SCENARIO 3: New Billing Ticket (Using Memory & Potentially Evolved Router) ---[/]")
    task3_primary_goal = f"Process a new billing ticket using the best available router, potentially the evolved '{evolved_agent_name}'."
    task3_sub_task = "Customer reports: 'My bill is incorrect for last month.' Ensure correct routing to BillingInquiryTool_v1_RealEvo."
    task3_description_for_agent = f"""
    A new customer billing ticket has arrived: "My bill is incorrect for last month. There's an unrecognized charge."
    Your goal is to process this support ticket accurately.
    1. Use ContextBuilderTool. Search for experiences related to 'billing ticket routing' and relevant routing AGENTs.
       Consider any recently evolved agents like '{evolved_agent_name}' (ID: {evolved_agent_id if evolved_agent_id else 'UNKNOWN_TRY_SEARCH_BY_NAME'}).
    2. Based on the full context (past experiences, available components), select the BEST routing AGENT.
    3. Use RequestAgentTool to execute the chosen routing agent with the ticket text "My bill is incorrect for last month. There's an unrecognized charge."
    4. The routing agent must ensure this billing ticket is routed to 'BillingInquiryTool_v1_RealEvo'.
    Return a JSON object with the final routing decision, the specialized tool used, and the tool's response.
    Example JSON output: {{"routing_agent_used": "...", "specialized_tool_used": "...", "tool_response": "...", "ticket_analysis": "..."}}
    """
    s3_result_data = await run_system_agent_task_and_record_experience(
        system_agent, "billing_ticket_evolved", task3_description_for_agent, task3_primary_goal, task3_sub_task
    )
    all_demo_outputs["scenarios"].append(s3_result_data)
    
    # Analyze if the evolved agent was used and if routing was correct
    if s3_result_data.get("agent_final_output_json"):
        final_json = s3_result_data["agent_final_output_json"]
        used_router = final_json.get("routing_agent_used", "")
        used_tool = final_json.get("specialized_tool_used", "")
        if evolved_agent_name in used_router and "BillingInquiryTool_v1_RealEvo" in used_tool:
            console.print(f"  [bold green]SUCCESSFUL EVOLUTION & USAGE:[/bold green] Evolved router '{evolved_agent_name}' correctly used 'BillingInquiryTool_v1_RealEvo'.")
        elif "GenericTicketRouterAgent_v1_RealEvo" in used_router and "BillingInquiryTool_v1_RealEvo" in used_tool:
            console.print(f"  [bold yellow]PARTIAL SUCCESS:[/bold yellow] Generic router was used but managed to route correctly (perhaps due to simpler ticket text or better LLM luck this time). Evolution benefit not fully demonstrated if evolved agent wasn't picked.")
        else:
            console.print(f"  [bold red]SUBOPTIMAL/FAILURE:[/bold red] Routing was: {used_router} -> {used_tool}. Expected specialized routing to BillingInquiryTool_v1_RealEvo via evolved agent.")
    else:
        console.print("  [bold red]ERROR:[/bold red] Could not parse final JSON output for Scenario 3 to verify routing.")


    # Save all demo outputs
    with open(DEMO_OUTPUT_FILE, "w") as f:
        f.write(safe_json_dumps(all_demo_outputs, indent=2))
    console.print(f"\n[bold]Demo outputs saved to: {DEMO_OUTPUT_FILE}[/bold]")
    
    console.print(Panel.fit(
        "[bold green]EAT Smart Memory & Real Evolution Demo Completed![/]\n"
        "Inspect the console output, JSON results, and MongoDB collections (`eat_agent_experiences`, `eat_components`) "
        "to observe how Smart Memory influenced context building and component evolution.",
        border_style="green", padding=(1,2)
    ))

if __name__ == "__main__":
    if not os.getenv("OPENAI_API_KEY"):
        console.print("[bold red]ERROR: OPENAI_API_KEY not found in environment variables or .env file.[/]")
    elif not eat_config.MONGODB_URI or not eat_config.MONGODB_DATABASE_NAME:
        console.print("[bold red]ERROR: MONGODB_URI or MONGODB_DATABASE_NAME not configured.[/]")
    else:
        asyncio.run(run_real_smart_memory_demo())