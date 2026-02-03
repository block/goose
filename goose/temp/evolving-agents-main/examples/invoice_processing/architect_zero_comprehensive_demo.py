# examples/invoice_processing/architect_zero_comprehensive_demo.py

import asyncio
import logging
import json
import os
import shutil
from typing import Dict, Any, Optional, List
from dotenv import load_dotenv
import re
from rich.console import Console
from rich.panel import Panel
from rich.syntax import Syntax
from rich import print as rprint
import sys # Ensure sys is imported for path manipulation

# --- Evolving Agents Imports ---
# Ensure the project root is in sys.path if running directly and not as an installed package
script_dir = os.path.dirname(os.path.abspath(__file__))
project_root = os.path.dirname(os.path.dirname(script_dir)) # Go up two levels
if project_root not in sys.path:
    sys.path.insert(0, project_root)

from evolving_agents.core.llm_service import LLMService
from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.agent_bus.smart_agent_bus import SmartAgentBus
from evolving_agents.core.system_agent import SystemAgentFactory
from evolving_agents.core.dependency_container import DependencyContainer
from evolving_agents.firmware.firmware import Firmware
from evolving_agents.agents.architect_zero import ArchitectZeroAgentInitializer
from evolving_agents.core.mongodb_client import MongoDBClient
from evolving_agents import config as eat_config

# MemoryManagerAgent and its dependencies
from evolving_agents.agents.memory_manager_agent import MemoryManagerAgent
from evolving_agents.tools.internal.mongo_experience_store_tool import MongoExperienceStoreTool
from evolving_agents.tools.internal.semantic_experience_search_tool import SemanticExperienceSearchTool
from evolving_agents.tools.internal.message_summarization_tool import MessageSummarizationTool
from beeai_framework.memory import UnconstrainedMemory, TokenMemory


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
# logging.getLogger("SmartAgentBus").setLevel(logging.INFO)
# logging.getLogger("SmartLibrary").setLevel(logging.INFO)
# logging.getLogger("LLMCache").setLevel(logging.WARNING)

logger = logging.getLogger(__name__) # Logger for this script
if not load_dotenv():
    logger.warning(".env file not found or python-dotenv not installed. Environment variables might not be loaded from .env.")

# --- Rich Console for better display ---
console = Console(width=120)

# --- Constants for Demo Output Files ---
WORKFLOW_OUTPUT_PATH = "final_processing_output.json"
SAMPLE_INVOICE_PATH = "sample_invoice_input.txt"
# INTENT_PLAN_FILE_COPY_PATH is now sourced from eat_config

SAMPLE_INVOICE = """
INVOICE #INV-9876
Date: 2024-07-15
Vendor: Advanced Tech Solutions
Address: 555 Innovation Drive, Tech Park, CA 95054

Bill To:
Global Corp Inc.
1 Enterprise Plaza
Metropolis, NY 10020

Items:
1. Quantum Entanglement Router - $15,000.00 (1 unit)
2. AI Co-processor Module    - $2,500.00 (4 units)
3. Holo-Display Unit        - $1,200.00 (2 units)

Subtotal: $27,400.00
Tax (9.25%): $2,534.50
Shipping: $150.00
Total Due: $30,084.50

Payment Terms: Net 45
Due Date: 2024-08-29

Notes: Expedited shipping requested.
"""

# --- Helper Functions ---
async def clean_previous_demo_outputs():
    """Remove previous demo-specific output files to start fresh."""
    console.print("[bold blue]OPERATION: Cleaning previous demo output files...[/]")
    intent_plan_path_from_config = getattr(eat_config, 'INTENT_REVIEW_OUTPUT_PATH', 'intent_plans_output_copy')

    items_to_remove = [
        WORKFLOW_OUTPUT_PATH, SAMPLE_INVOICE_PATH, 
        intent_plan_path_from_config,
        "architect_design_output.json", "architect_raw_output.txt",
        "generated_invoice_workflow.yaml"
    ]
    obsolete_stores = [
        "smart_library_demo.json", "smart_agent_bus_demo.json", "agent_bus_logs_demo.json",
        "./vector_db_demo",
        eat_config.LLM_CACHE_DIR if not eat_config.LLM_USE_CACHE or not eat_config.MONGODB_URI else None
    ]
    items_to_remove.extend(filter(None, obsolete_stores))

    for item_path in items_to_remove:
        try:
            if os.path.exists(item_path):
                if os.path.isdir(item_path): shutil.rmtree(item_path)
                else: os.remove(item_path)
                logger.info(f"Removed demo artifact: {item_path}")
        except Exception as e: logger.warning(f"Could not remove {item_path}: {e}")
    console.print("[green]✓[/] Previous demo-specific output files cleaned.")
    console.print("[yellow]Note:[/yellow] MongoDB data from previous runs is not automatically cleaned by this script. "
                  "If you need a completely fresh DB state, clear the relevant collections in MongoDB manually or "
                  "use a different MONGODB_DATABASE_NAME for this demo run in your .env file.")


async def setup_framework_environment(container: DependencyContainer) -> DependencyContainer:
    """Initializes all core framework services and agents with MongoDB backend."""
    console.print("\n[bold blue]OPERATION: Initializing Framework Environment (MongoDB Backend)[/]")

    # 0. MongoDB Client
    console.print("  → Initializing MongoDB Client...")
    mongodb_client_instance = None
    try:
        mongo_uri = eat_config.MONGODB_URI
        mongo_db_name = eat_config.MONGODB_DATABASE_NAME
        if not mongo_uri: raise ValueError("MONGODB_URI not set.")
        if not mongo_db_name: raise ValueError("MONGODB_DATABASE_NAME not set.")
        
        mongodb_client_instance = MongoDBClient(uri=mongo_uri, db_name=mongo_db_name)
        console.print("    Pinging MongoDB server...")
        if not await mongodb_client_instance.ping_server():
            raise ConnectionError(f"MongoDB server ping failed for URI: {mongo_uri} DB: {mongo_db_name}")
        console.print("    [green]✓[/] MongoDB server ping successful.")
        container.register('mongodb_client', mongodb_client_instance)
        console.print(f"  [green]✓[/] MongoDB Client initialized for DB: '{mongodb_client_instance.db_name}'")
    except Exception as e:
        console.print(f"  [bold red]✗ ERROR: Failed to initialize MongoDBClient: {e}[/]")
        console.print("  [bold red]Please ensure MONGODB_URI & MONGODB_DATABASE_NAME are correctly set in your .env file and MongoDB is accessible.[/]")
        console.print("  [bold red]Refer to `docs/MONGO-SETUP.md` for guidance on setting up MongoDB Atlas CLI Local Deployment.[/]")
        raise

    # 1. LLM Service
    console.print("  → Initializing LLM Service (with MongoDB cache if enabled)...")
    llm_service = LLMService(
        provider=eat_config.LLM_PROVIDER, model=eat_config.LLM_MODEL,
        embedding_model=eat_config.LLM_EMBEDDING_MODEL, use_cache=eat_config.LLM_USE_CACHE,
        container=container
    )
    container.register('llm_service', llm_service)

    # --- Instantiate Internal Tools for MemoryManagerAgent ---
    console.print("  → Initializing Internal Tools for MemoryManagerAgent...")
    experience_store_tool = MongoExperienceStoreTool(mongodb_client_instance, llm_service)
    semantic_search_tool = SemanticExperienceSearchTool(mongodb_client_instance, llm_service)
    message_summarization_tool = MessageSummarizationTool(llm_service)
    console.print("  [green]✓[/] Internal Tools for MemoryManagerAgent initialized.")

    # --- Instantiate MemoryManagerAgent ---
    console.print("  → Initializing MemoryManagerAgent...")
    memory_manager_agent_memory = TokenMemory(llm_service.chat_model)
    memory_manager_agent = MemoryManagerAgent(
        llm_service=llm_service,
        mongo_experience_store_tool=experience_store_tool,
        semantic_search_tool=semantic_search_tool,
        message_summarization_tool=message_summarization_tool,
        memory_override=memory_manager_agent_memory
    )
    container.register('memory_manager_agent_instance', memory_manager_agent)
    console.print("  [green]✓[/] MemoryManagerAgent initialized.")

    # 2. Smart Library
    console.print("  → Initializing Smart Library (MongoDB backend)...")
    smart_library = SmartLibrary(container=container)
    container.register('smart_library', smart_library)

    # 3. Firmware
    console.print("  → Initializing Firmware...")
    firmware = Firmware(); container.register('firmware', firmware)

    # 4. Agent Bus
    console.print("  → Initializing Agent Bus (MongoDB backend)...")
    agent_bus = SmartAgentBus(container=container)
    container.register('agent_bus', agent_bus)

    # --- Register MemoryManagerAgent with SmartAgentBus ---
    console.print("  → Registering MemoryManagerAgent with SmartAgentBus...")
    MEMORY_MANAGER_AGENT_ID = "memory_manager_agent_default_id"
    await agent_bus.register_agent(
        agent_id=MEMORY_MANAGER_AGENT_ID, name="MemoryManagerAgent",
        description="Manages experiences for learning. Call via 'process_task' with natural language.",
        agent_type="MemoryManagement",
        capabilities=[{"id": "process_task", "name": "Process Memory Task", "description": "Handles storage, retrieval, and summarization."}],
        agent_instance=memory_manager_agent,
        embed_capabilities=True
    )
    console.print(f"  [green]✓[/] MemoryManagerAgent registered on SmartAgentBus with ID: {MEMORY_MANAGER_AGENT_ID}.")

    # 5. System Agent
    console.print("  → Initializing System Agent...")
    system_agent = await SystemAgentFactory.create_agent(container=container)
    container.register('system_agent', system_agent)

    # 6. Architect Agent
    console.print("  → Initializing Architect Zero Agent...")
    architect_agent = await ArchitectZeroAgentInitializer.create_agent(container=container)
    container.register('architect_agent', architect_agent)

    console.print("  → Seeding initial library components into MongoDB...")
    await seed_initial_library_mongodb(smart_library)
    
    console.print("  → Performing late initialization of SmartLibrary (triggers AgentBus sync if needed)...")
    await smart_library.initialize()

    console.print("[green]✓[/] Framework environment fully initialized with MongoDB backend.")
    return container

async def seed_initial_library_mongodb(smart_library: SmartLibrary):
    """Seed the MongoDB SmartLibrary with basic components for the demo."""
    components_to_ensure = [
        {
            "name": "BasicDocumentAnalyzer", "record_type": "TOOL", "domain": "document_processing",
            "description": "A basic tool that analyzes documents to guess their type (invoice, receipt, etc.) using keyword matching.",
            "code_snippet": """
from typing import Dict; import json; from pydantic import BaseModel, Field
from beeai_framework.tools.tool import StringToolOutput, Tool
from beeai_framework.context import RunContext; from beeai_framework.emitter import Emitter
class Input(BaseModel): text: str
class BasicDocumentAnalyzer(Tool[Input, None, StringToolOutput]):
    name = "BasicDocumentAnalyzer"; description = "Guesses document type (invoice, receipt, contract) based on keywords."
    input_schema = Input
    def _create_emitter(self) -> Emitter: return Emitter.root().child(creator=self)
    async def _run(self, input: Input, options=None, context=None) -> StringToolOutput:
        txt = input.text.lower(); r = {"type":"unknown","confidence":0.0}
        if "invoice" in txt and ("total" in txt or "amount due" in txt or "bill to" in txt): r = {"type":"invoice","confidence":0.8}
        elif "receipt" in txt and "payment" in txt: r = {"type":"receipt","confidence":0.7}
        elif "contract" in txt or "agreement" in txt and ("party" in txt or "parties" in txt): r = {"type":"contract","confidence":0.7}
        return StringToolOutput(json.dumps(r))
""", "version": "1.0.0", "tags":["document", "analysis", "classification", "beeai"], "metadata":{"framework": "beeai"}
        },
        {
            "name":"BasicInvoiceProcessor", "record_type":"AGENT", "domain":"financial",
            "description": "A basic BeeAI agent designed to extract key fields (invoice number, date, vendor, total) from invoice text.",
            "code_snippet": """
from typing import List, Dict, Any, Optional; import re; import json
from beeai_framework.agents.react import ReActAgent; from beeai_framework.agents.types import AgentMeta
from beeai_framework.memory import TokenMemory; from beeai_framework.backend.chat import ChatModel
from beeai_framework.tools.tool import Tool
class BasicInvoiceProcessorInitializer:
    @staticmethod
    def create_agent(llm: ChatModel, tools: Optional[List[Tool]] = None) -> ReActAgent:
        meta = AgentMeta(
            name="BasicInvoiceProcessor",
            description="I extract invoice number, date, vendor, and total amount from invoice text using simple logic.",
            tools=tools or []
        )
        agent = ReActAgent(llm=llm, tools=tools or [], memory=TokenMemory(llm), meta=meta)
        return agent
""", "version":"1.0.0", "tags":["invoice", "processing", "extraction", "basic", "beeai", "finance"], "metadata":{"framework": "beeai"}
        }
    ]

    for comp_data in components_to_ensure:
        existing = await smart_library.find_record_by_name(comp_data["name"], comp_data["record_type"])
        if not existing:
            console.print(f"    - Creating '{comp_data['name']}' in MongoDB SmartLibrary...")
            await smart_library.create_record(**comp_data)
        else:
            console.print(f"    - Component '{comp_data['name']}' already exists in SmartLibrary. Skipping seed.")
    
    console.print("    - Allowing a moment for MongoDB operations if any were performed...")
    await asyncio.sleep(0.5)


async def extract_json_with_llm(llm_service: LLMService, response_text: str) -> Optional[Dict[str, Any]]:
    extraction_prompt = f"""
    The following text is a response from an AI agent. It is expected to contain a JSON object.
    Please extract ONLY the main JSON object from this text.
    Return JUST the JSON object itself, with no surrounding text, explanations, or markdown code blocks.
    If there's no clear, complete JSON object in the text, return a JSON object with a single field:
    {{"error": "No valid JSON object found in the provided text.", "original_text": "PLACEHOLDER_FOR_ORIGINAL_TEXT_SNIPPET"}}
    where PLACEHOLDER_FOR_ORIGINAL_TEXT_SNIPPET is a short snippet of the original text.

    TEXT TO EXTRACT FROM:
    ---
    {response_text}
    ---
    EXTRACTED JSON:
    """
    try:
        extracted_text_llm = await llm_service.generate(extraction_prompt)
        try:
            return json.loads(extracted_text_llm)
        except json.JSONDecodeError:
            match = re.search(r"```json\s*(\{[\s\S]*?\})\s*```", extracted_text_llm, re.MULTILINE | re.DOTALL)
            if match:
                try:
                    return json.loads(match.group(1).strip())
                except json.JSONDecodeError:
                    logger.warning(f"LLM extraction: Found JSON block but failed to parse: {match.group(1)[:100]}...")
            logger.warning(f"LLM extraction produced non-JSON or invalid JSON: {extracted_text_llm[:200]}...")
            original_snippet = response_text[:100] + "..." if len(response_text) > 100 else response_text
            return {"error": "LLM extraction did not yield a directly parsable JSON object.", "llm_output": extracted_text_llm, "original_text_snippet_for_llm": original_snippet}
    except Exception as e:
        logger.error(f"Error using LLM to extract JSON: {e}")
        return {"error": f"Exception during LLM extraction: {e}"}

def extract_json_from_response(response_text: str) -> Optional[Dict[str, Any]]:
    if not response_text: return None
    patterns = [
        r"```json\s*(\{[\s\S]*?\})\s*```",
        r"```\s*(\{[\s\S]*?\})\s*```",
    ]
    for pattern in patterns:
        match = re.search(pattern, response_text, re.MULTILINE | re.DOTALL)
        if match:
            try:
                potential_json_str = match.group(1).strip()
                if potential_json_str.startswith('{') and potential_json_str.endswith('}') and potential_json_str.count('{') >= potential_json_str.count('}'): # Basic check
                    return json.loads(potential_json_str)
            except json.JSONDecodeError:
                logger.debug(f"Pattern '{pattern}' matched but failed to parse as JSON: '{match.group(1)[:100]}...'")
                continue
    
    json_objects_found = []
    for match in re.finditer(r"(\{[\s\S]*?\})(?=\s*\{|\Z)", response_text, re.DOTALL):
        potential_json_str = match.group(1).strip()
        try:
            if potential_json_str.startswith('{') and potential_json_str.endswith('}') and potential_json_str.count('{') >= potential_json_str.count('}'):
                if ':' in potential_json_str and ('"' in potential_json_str or "'" in potential_json_str):
                    parsed = json.loads(potential_json_str)
                    if isinstance(parsed, dict) and len(parsed.keys()) > 1: 
                        json_objects_found.append(parsed)
        except json.JSONDecodeError:
            logger.debug(f"Loose JSON match failed to parse: '{potential_json_str[:100]}...'")
    
    if json_objects_found:
        return max(json_objects_found, key=lambda x: len(json.dumps(x)))

    logger.warning("Could not extract JSON using pattern matching from SystemAgent response.")
    return None

def get_nested_value(data_dict: Dict[str, Any], keys_list_to_check: List[str], default_val: Any = 'N/A') -> Any:
    """
    Retrieves a value from a dictionary using a list of possible keys.
    Performs case-insensitive and space-insensitive matching for keys, also removing '#'.
    """
    if not isinstance(data_dict, dict):
        return default_val

    normalized_data_keys_map = {
        key.lower().replace(" ", "").replace("#", ""): key 
        for key in data_dict.keys() if isinstance(key, str)
    }

    for key_to_try in keys_list_to_check:
        # Try exact match first (original key from the list)
        if key_to_try in data_dict:
            return data_dict[key_to_try]
        
        # Try normalized match
        normalized_key_to_try = key_to_try.lower().replace(" ", "").replace("#", "")
        if normalized_key_to_try in normalized_data_keys_map:
            original_key_in_dict = normalized_data_keys_map[normalized_key_to_try]
            return data_dict[original_key_in_dict]
            
    return default_val

# --- Main Demo Function ---
async def run_demo():
    console.print(Panel.fit(
        "[bold yellow]Evolving Agents Toolkit - Invoice Processing Demo (MongoDB Backend)[/]",
        border_style="yellow", padding=(1, 2)
    ))
    console.print("\n[bold]This demonstration shows how the SystemAgent can:[/]")
    console.print("  1. Accept a high-level business goal (invoice processing)")
    console.print("  2. Handle component search, creation, and orchestration internally (using MongoDB)")
    console.print("  3. Execute complex tasks without specifying implementation details")
    console.print("  4. Return structured, verified results\n")

    await clean_previous_demo_outputs()
    with open(SAMPLE_INVOICE_PATH, "w") as f: f.write(SAMPLE_INVOICE)
    console.print(f"\n[dim]Sample invoice saved to [cyan]{SAMPLE_INVOICE_PATH}[/][/]")
    container = DependencyContainer()

    try:
        container = await setup_framework_environment(container)
    except Exception as setup_exc:
        console.print(f"[bold red]FATAL: Framework setup failed during initialization: {setup_exc}[/]")
        console.print_exception(show_locals=False)
        return

    system_agent = container.get('system_agent')
    llm_service = container.get('llm_service')

    high_level_task_prompt = f"""
    **Goal:** Accurately process the provided invoice document and return structured, verified data.

    **Functional Requirements:**
    - Extract key fields: Invoice # (or InvoiceNumber), Date, Vendor, Bill To, Line Items (Description, Quantity, Unit Price, Item Total), Subtotal, Tax Amount, Shipping (if present), Total Due, Payment Terms, Due Date.
    - Verify calculations: The sum of line item totals should match the Subtotal. The sum of Subtotal, Tax Amount, and Shipping (if present) must match the Total Due. Report any discrepancies.

    **Non-Functional Requirements:**
    - High accuracy is critical.
    - Output must be a single, valid JSON object containing the extracted data and a 'verification' or 'Verification' section (status: 'ok'/'failed', discrepancies: list).
    - Handle potential variations in invoice layouts gracefully.

    **Input Data:**
    ```
    {SAMPLE_INVOICE}
    ```

    **Action:** Achieve this goal using the best approach available within the framework. Create, evolve, or reuse components as needed. Your final response should consist of ONLY the JSON object containing the extracted and verified invoice data. Do not include any explanatory text before or after the JSON object.
    """

    console.print("\n[bold blue]OPERATION: SystemAgent processing invoice task[/]")
    rprint(Panel(f"[italic]Task Prompt Given to SystemAgent:[/]\n{high_level_task_prompt[:400]}...", title="SystemAgent Input", border_style="magenta", expand=False))

    execution_result = {}
    with console.status("[bold green]SystemAgent is orchestrating the task... This may take a few minutes.[/]", spinner="dots"):
        try:
            sys_agent_response = await system_agent.run(high_level_task_prompt)
            final_output_text = sys_agent_response.result.text if hasattr(sys_agent_response.result, 'text') and sys_agent_response.result.text else str(sys_agent_response)
            logger.info(f"Raw SystemAgent Output (first 1000 chars):\n{final_output_text[:1000]}")

            final_result_json = extract_json_from_response(final_output_text)

            if not final_result_json or ("error" in final_result_json and final_result_json.get("error") != "No valid JSON object found in the provided text."):
                console.print("\r[yellow]⚠ Standard JSON extraction failed or yielded error. Attempting LLM-based JSON extraction...[/]", end="", flush=True)
                await asyncio.sleep(0.1)
                final_result_json = await extract_json_with_llm(llm_service, final_output_text)

            if final_result_json and "error" not in final_result_json:
                final_result_data = final_result_json
                console.print("\r[green]✓[/] Successfully extracted JSON result from SystemAgent output.                                  ")
            else:
                console.print("\r[red]✗[/] Could not extract valid JSON result from SystemAgent output. Raw text output will be used.             ")
                final_result_data = final_output_text 
                if final_result_json and "error" in final_result_json:
                     logger.error(f"LLM-based JSON extraction also failed or returned error: {final_result_json}")

            execution_result = {
                "status": "completed_successfully" if isinstance(final_result_data, dict) and "error" not in final_result_data else "completed_with_warnings_or_error",
                "final_result": final_result_data,
                "agent_raw_output": final_output_text
            }
            with open(WORKFLOW_OUTPUT_PATH, "w", encoding="utf-8") as f:
                json.dump(execution_result, f, indent=2, ensure_ascii=False)
            console.print(f"[dim]Full execution result saved to [cyan]{WORKFLOW_OUTPUT_PATH}[/][/]")

        except Exception as e:
            console.print(f"\r[bold red]✗ ERROR executing SystemAgent task: {e}[/]")
            import traceback
            logger.error("SystemAgent task execution failed:", exc_info=True)
            execution_result = { "status": "error", "error_message": str(e), "traceback": traceback.format_exc(), "agent_raw_output": "" }
            with open(WORKFLOW_OUTPUT_PATH, "w", encoding="utf-8") as f:
                json.dump(execution_result, f, indent=2, ensure_ascii=False)

    # --- Display the final result ---
    console.print("\n[bold green]OPERATION COMPLETE: Invoice Processing Result[/]")
    final_data_to_display = execution_result.get("final_result")

    if isinstance(final_data_to_display, dict) and "error" not in final_data_to_display:
        json_str = json.dumps(final_data_to_display, indent=2, ensure_ascii=False)
        syntax = Syntax(json_str, "json", theme="monokai", line_numbers=True, word_wrap=True)
        console.print(Panel(syntax, title="[bold]Extracted Invoice Data (JSON)[/]", border_style="green", expand=True))
    elif isinstance(final_data_to_display, str):
        console.print(Panel(final_data_to_display, title="[bold]Raw Output/Error from SystemAgent[/]", border_style="yellow", expand=True))
    elif isinstance(final_data_to_display, dict) and "error" in final_data_to_display:
        console.print(Panel(json.dumps(final_data_to_display, indent=2, ensure_ascii=False), title="[bold red]Error in Extracted Data[/]", border_style="red", expand=True))
    else:
        console.print(Panel(str(final_data_to_display or "No displayable result found."), title="[bold]Result[/]", border_style="red", expand=True))

    if isinstance(final_data_to_display, dict) and "error" not in final_data_to_display:
        final_data_dict = final_data_to_display
        console.print("\n[bold]Key Extracted Fields Summary:[/]")
        
        invoice_no_keys = ['InvoiceNumber', 'Invoice #', 'InvoiceID', 'Invoice No']
        vendor_keys = ['Vendor', 'VendorName']
        date_keys = ['Date', 'InvoiceDate']
        total_due_keys = ['TotalDue', 'Total Due', 'AmountDue', 'Total']
        verification_keys = ['Verification', 'verification']

        console.print(f"  • Invoice #: [bold cyan]{get_nested_value(final_data_dict, invoice_no_keys)}[/]")
        console.print(f"  • Vendor: [bold cyan]{get_nested_value(final_data_dict, vendor_keys)}[/]")
        console.print(f"  • Date: [cyan]{get_nested_value(final_data_dict, date_keys)}[/]")
        console.print(f"  • Total Due: [bold cyan]${get_nested_value(final_data_dict, total_due_keys)}[/]")

        verification_details = get_nested_value(final_data_dict, verification_keys, default_val={})
            
        ver_status = str(verification_details.get('status', 'unknown')).lower()
        ver_message = verification_details.get('message', 'Calculations correct' if ver_status == 'ok' else 'Discrepancies found or unable to verify')

        if ver_status in ['ok', 'passed', 'verified', 'success']:
            console.print(f"  • Verification: [bold green]PASSED[/] ([italic]{ver_message}[/])")
        else:
            console.print(f"  • Verification: [bold red]FAILED or UNKNOWN[/] ([italic]{ver_message}[/])")
            discrepancies = verification_details.get('discrepancies', [])
            for discrepancy in discrepancies:
                console.print(f"    [red]- {discrepancy}[/]")
    else:
         console.print("\n[yellow]Key fields summary not available as final result was not structured JSON or contained an error.[/]")

    console.print("\n[bold blue]DEMONSTRATION SUMMARY[/]")
    console.print("This demo showcased the Evolving Agents Toolkit's ability to:")
    console.print("  1. [green]✓[/] Process a high-level business goal using MongoDB as the backend.")
    console.print("  2. [green]✓[/] Dynamically manage components (search, create, evolve) stored in MongoDB.")
    console.print("  3. [green]✓[/] (Attempt to) Verify calculations in extracted data.")
    console.print("  4. [green]✓[/] (Attempt to) Return structured JSON results.")
    console.print("\n[bold]Data Persistence:[/bold]")
    console.print(f"  • All core EAT data (SmartLibrary, AgentBus, LLMCache, IntentPlans) uses MongoDB.")
    console.print(f"  • Database: [cyan]{eat_config.MONGODB_DATABASE_NAME}[/]")
    console.print(f"  • Key Collections: `eat_components`, `eat_agent_registry`, `eat_agent_bus_logs`, `eat_llm_cache`, `eat_intent_plans`, `eat_agent_experiences`.")
    console.print(f"  • Final demo output (this script's result) saved to: [cyan]{WORKFLOW_OUTPUT_PATH}[/]")
    
    intent_plan_path_from_config = getattr(eat_config, 'INTENT_REVIEW_OUTPUT_PATH', 'intent_plans_output_copy')
    if eat_config.INTENT_REVIEW_ENABLED and "intents" in eat_config.INTENT_REVIEW_LEVELS:
        console.print(f"  • Optional IntentPlan file copy (if review enabled and path set): [cyan]{intent_plan_path_from_config}[/]")
    else:
        console.print(f"  • Intent Review for 'intents' level is currently [yellow]disabled[/] in config or not set for this demo.")

if __name__ == "__main__":
    try:
        if not os.getenv("MONGODB_URI") or not os.getenv("OPENAI_API_KEY"):
            console.print("[bold red]ERROR: MONGODB_URI and/or OPENAI_API_KEY not found in .env file or environment.[/]")
            console.print("Please ensure your .env file is correctly configured per .env.example and `docs/MONGO-SETUP.md`.")
            console.print(f"  Loaded MONGODB_URI: {os.getenv('MONGODB_URI')}")
            console.print(f"  Loaded OPENAI_API_KEY: {'SET' if os.getenv('OPENAI_API_KEY') else 'NOT SET'}")
        else:
            if eat_config.INTENT_REVIEW_ENABLED and "intents" in eat_config.INTENT_REVIEW_LEVELS:
                 console.print("[yellow]Intent Review for 'intents' level is ENABLED for this demo run (as per config).[/]")
            else:
                 console.print("[yellow]Intent Review for 'intents' level is DISABLED for this demo run (as per config).[/]")
            asyncio.run(run_demo())
    except Exception as main_error:
        console.print(f"[bold red]CRITICAL ERROR in main demo execution loop:[/]")
        console.print_exception(show_locals=False)