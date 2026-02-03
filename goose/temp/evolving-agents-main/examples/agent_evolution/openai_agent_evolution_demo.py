# /examples/agent_evolution/openai_agent_evolution_demo.py

import asyncio
import logging
import os
import sys
import json
import time
import re 
from typing import Dict, Any, List, Optional, Tuple
from dotenv import load_dotenv

script_dir = os.path.dirname(os.path.abspath(__file__))
project_root = os.path.dirname(os.path.dirname(script_dir)) 
sys.path.insert(0, project_root)

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.core.llm_service import LLMService
from evolving_agents.core.system_agent import SystemAgentFactory
from evolving_agents.agent_bus.smart_agent_bus import SmartAgentBus
from evolving_agents.providers.registry import ProviderRegistry
from evolving_agents.providers.beeai_provider import BeeAIProvider
from evolving_agents.providers.openai_agents_provider import OpenAIAgentsProvider
from evolving_agents.agents.agent_factory import AgentFactory
from evolving_agents.core.dependency_container import DependencyContainer
from evolving_agents.firmware.firmware import Firmware
from evolving_agents.core.mongodb_client import MongoDBClient
from evolving_agents import config as eat_config
from evolving_agents.utils.json_utils import safe_json_dumps
# Evolving Agents Imports
from evolving_agents.agents.memory_manager_agent import MemoryManagerAgent
from evolving_agents.tools.internal.mongo_experience_store_tool import MongoExperienceStoreTool
from evolving_agents.tools.internal.semantic_experience_search_tool import SemanticExperienceSearchTool
from evolving_agents.tools.internal.message_summarization_tool import MessageSummarizationTool
from beeai_framework.memory import TokenMemory


logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

if not load_dotenv():
    logger.warning(".env file not found or python-dotenv not installed. Environment variables might not be loaded from .env.")

# --- Sample Data (remains the same) ---
SAMPLE_INVOICE = """
INVOICE #12345
Date: 2023-05-15
Vendor: TechSupplies Inc.
Address: 123 Business Ave, Commerce City, CA 90210
Items:
1. Laptop Computer - $1,200.00 (2 units)
2. Wireless Mouse - $25.00 (5 units)
3. External Hard Drive - $85.00 (3 units)
Subtotal: $1,680.00
Tax (8.5%): $142.80
Total Due: $1,822.80
Payment Terms: Net 30
Due Date: 2023-06-14
"""
SAMPLE_MEDICAL_RECORD = """
PATIENT MEDICAL RECORD
Patient ID: P789456
Name: John Smith
DOB: 1975-03-12
Visit Date: 2023-05-10
Chief Complaint: Patient presents with persistent cough for 2 weeks, mild fever, and fatigue.
Vitals:
- Temperature: 100.2°F
- Blood Pressure: 128/82
- Heart Rate: 88 bpm
- Respiratory Rate: 18/min
- Oxygen Saturation: 97%
Assessment: Acute bronchitis
Plan: Prescribed antibiotics (Azithromycin 500mg) for 5 days, recommended rest and increased fluid intake.
Follow-up in 1 week if symptoms persist.
"""
SAMPLE_CONTRACT = """
SERVICE AGREEMENT CONTRACT
Contract ID: CA-78901
Date: 2023-06-01
BETWEEN:
ABC Consulting Ltd. ("Provider")
123 Business Lane, Corporate City, BZ 54321
AND:
XYZ Corporation ("Client")
456 Commerce Ave, Enterprise Town, ET 12345
SERVICES:
Provider agrees to deliver the following services:
1. Strategic business consulting - 40 hours at $200/hour
2. Market analysis report - Fixed fee $5,000
3. Implementation support - 20 hours at $250/hour
TERM:
This agreement commences on July 1, 2023 and terminates on December 31, 2023.
PAYMENT TERMS:
- 30% deposit due upon signing
- 30% due upon delivery of market analysis report
- 40% due upon completion of implementation support
- All invoices due Net 15
TERMINATION:
Either party may terminate with 30 days written notice.
"""

def extract_id_from_json_text(response_text: str, key_to_find: str = "id") -> Optional[str]:
    if not response_text: return None
    try:
        json_match = re.search(r"(\{.*\})", response_text, re.DOTALL) 
        if json_match:
            json_str = json_match.group(1)
            data = json.loads(json_str)
            if isinstance(data, dict):
                # Direct key checks
                if key_to_find in data and isinstance(data[key_to_find], str): return data[key_to_find]
                for k_var in ["record_id", "evolved_id", "plan_id", "id"]: 
                    if k_var == key_to_find: continue 
                    if k_var in data and isinstance(data[k_var], str): return data[k_var]
                
                # Nested key checks
                for nested_key in ["record", "evolved_record", "saved_record", "component", "agent", "tool", "evolved_agent", "created_agent"]:
                     if nested_key in data and isinstance(data[nested_key], dict):
                        target_dict = data[nested_key]
                        if key_to_find in target_dict and isinstance(target_dict[key_to_find], str): return target_dict[key_to_find]
                        if "id" in target_dict and isinstance(target_dict["id"], str): return target_dict["id"]
                
                if "results" in data and isinstance(data["results"], list) and data["results"]:
                    if isinstance(data["results"][0], dict):
                        first_result = data["results"][0]
                        if key_to_find in first_result: return first_result.get(key_to_find)
                        if "id" in first_result: return first_result.get("id") 
    except Exception: pass
    
    patterns = [ rf'"{key_to_find}":\s*"([^"]+)"' ]
    if key_to_find not in ["id", "record_id", "evolved_id"]: 
        patterns.extend([rf'"record_id":\s*"([^"]+)"', rf'"evolved_id":\s*"([^"]+)"', rf'"id":\s*"([^"]+)"'])
    
    patterns.extend([
        r'ID:\s*([a-fA-F0-9]{8}-(?:[a-fA-F0-9]{4}-){3}[a-fA-F0-9]{12}|[a-fA-F0-9]{24}|[a-zA-Z0-9_.\-]+)',
        r'([a-fA-F0-9]{8}-(?:[a-fA-F0-9]{4}-){3}[a-fA-F0-9]{12})',
    ])
    for pattern in patterns:
        match = re.search(pattern, response_text)
        if match and match.group(1):
            extracted_id = match.group(1).strip()
            if len(extracted_id) > 5 and not extracted_id.lower().startswith("http"):
                logger.info(f"Regex extracted ID '{extracted_id}' for key '{key_to_find}' using pattern: {pattern}")
                return extracted_id
    logger.warning(f"Could not extract ID for key '{key_to_find}' from SystemAgent response: {response_text[:200]}...")
    return None


async def use_llm_for_analysis(llm_service: LLMService, text: str, analysis_type: str) -> str:
    prompt = f"""
    Analyze the following text as a {analysis_type} document:
    {text}
    Please extract all key information and return a structured analysis.
    For invoices: Extract invoice number, date, vendor, items, costs, subtotal, tax, total.
    For medical records: Extract patient info, visit details, vitals, assessment, and plan.
    For contracts: Extract parties, services, terms, payment details, and termination conditions.
    Return your analysis in a structured, readable format.
    """
    result = await llm_service.generate(prompt)
    return result

async def text_analysis_demo(llm_service: LLMService, system_agent: Any, library: SmartLibrary):
    print("\n" + "-"*80 + "\nTEXT ANALYSIS DEMO: LLM SERVICE VS HARDCODED LOGIC\n" + "-"*80)
    
    print("\nAnalyzing invoice using LLM service...")
    start_time = time.time()
    llm_invoice_analysis = await use_llm_for_analysis(llm_service, SAMPLE_INVOICE, "invoice")
    llm_time = time.time() - start_time
    
    print("\nAnalyzing invoice using hardcoded regex logic...")
    start_time = time.time()
    hardcoded_analysis = {} 
    invoice_match = re.search(r'INVOICE #(\d+)', SAMPLE_INVOICE); 
    if invoice_match: hardcoded_analysis["invoice_number"] = invoice_match.group(1)
    date_match = re.search(r'Date: ([\d-]+)', SAMPLE_INVOICE); 
    if date_match: hardcoded_analysis["date"] = date_match.group(1)
    vendor_match = re.search(r'Vendor: ([^\n]+)', SAMPLE_INVOICE); 
    if vendor_match: hardcoded_analysis["vendor"] = vendor_match.group(1)
    subtotal_match = re.search(r'Subtotal: \$([0-9,.]+)', SAMPLE_INVOICE); 
    if subtotal_match: hardcoded_analysis["subtotal"] = subtotal_match.group(1)
    tax_match = re.search(r'Tax [^:]+: \$([0-9,.]+)', SAMPLE_INVOICE); 
    if tax_match: hardcoded_analysis["tax"] = tax_match.group(1)
    total_match = re.search(r'Total Due: \$([0-9,.]+)', SAMPLE_INVOICE); 
    if total_match: hardcoded_analysis["total"] = total_match.group(1)
    hardcoded_time = time.time() - start_time
    
    print("\nAnalyzing invoice using System Agent with SmartLibrary tools...")
    start_time = time.time()
    demo_invoice_tool_name = "DemoInvoiceProcessorTool_OpenAI_TextDemo" 
    system_prompt = f"""
    Your task is to analyze an invoice.
    1. Use the 'SearchComponentTool' with the query "invoice processing" to find components in the SmartLibrary.
    2. If suitable components are found, use the best one to process the invoice below.
    3. If no suitable component is found, use the 'CreateComponentTool' to create a new TOOL named "{demo_invoice_tool_name}" in the "finance" domain, framework "openai-agents". It should extract invoice number, date, vendor, items (name, price, quantity, total), subtotal, tax, and total_due. 
       After calling CreateComponentTool, your response MUST be the exact JSON output from that tool.
    4. After ensuring a tool exists (either found or created), use it to process the invoice.
    
    Invoice to process:
    ```
    {SAMPLE_INVOICE}
    ```
    Provide a final structured analysis of the invoice using the chosen/created tool.
    """
    system_response_message = await system_agent.run(system_prompt)
    system_response_text = system_response_message.result.text if hasattr(system_response_message, 'result') and hasattr(system_response_message.result, 'text') else str(system_response_message)
    system_time = time.time() - start_time
    
    await asyncio.sleep(1) 
    created_tool_record = await library.find_record_by_name(demo_invoice_tool_name, "TOOL")
    if created_tool_record:
        print(f"✓ SystemAgent created or confirmed existence of '{demo_invoice_tool_name}' with ID: {created_tool_record['id']}")
    else:
        print(f"✗ Could not verify creation of '{demo_invoice_tool_name}' in library by name after SystemAgent run. SystemAgent response: {system_response_text[:200]}...")

    
    print("\n" + "-"*30 + "\nANALYSIS COMPARISON\n" + "-"*30)
    print(f"\n1. LLM Analysis (took {llm_time:.2f}s):\n{llm_invoice_analysis[:500] + '...' if len(llm_invoice_analysis) > 500 else llm_invoice_analysis}")
    print(f"\n2. Hardcoded Regex Analysis (took {hardcoded_time:.2f}s):\n{json.dumps(hardcoded_analysis, indent=2)}")
    print(f"\n3. System Agent Analysis (took {system_time:.2f}s):\n{system_response_text[:500] + '...' if len(system_response_text) > 500 else system_response_text}")
    print("\n" + "-"*30 + "\nCOMPARISON INSIGHTS\n" + "-"*30)
    print(f"""
    PERFORMANCE COMPARISON:
    - Hardcoded Regex: {hardcoded_time:.2f}s (fastest, but most limited)
    - LLM Service: {llm_time:.2f}s (slower, but flexible)
    - System Agent: {system_time:.2f}s (slowest, but most comprehensive)
    QUALITY COMPARISON: (Based on potential if tools work correctly)
    - Hardcoded Regex: Extracts only specifically programmed fields.
    - LLM Service: Can extract more information with better formatting.
    - System Agent: Can leverage SmartLibrary intelligence and provide comprehensive analysis by finding/creating/using specialized components.
    WHEN TO USE EACH:
    - Hardcoded Logic: For highly predictable, performance-critical tasks.
    - LLM Service: For flexible ad-hoc analysis without library overhead.
    - System Agent: For complex tasks requiring component reuse, evolution, and robust processing.
    """)

async def demonstrate_system_agent_tools(system_agent: Any, library: SmartLibrary, llm_service: LLMService):
    print("\n" + "-"*80 + "\nSYSTEM AGENT TOOLS DEMONSTRATION\n" + "-"*80)
    
    print("\n1. DEMONSTRATING SEARCH COMPONENT TOOL")
    search_prompt = """
    Use your SearchComponentTool to find components in the SmartLibrary that can process invoices.
    The query should be "invoice processing".
    Return all matching components with their similarity scores.
    If no components are found, indicate that clearly.
    Your final response for this step should be ONLY the raw JSON output from the SearchComponentTool.
    """
    print("\nSearching for invoice processing components...")
    search_response_message = await system_agent.run(search_prompt)
    search_response_text = search_response_message.result.text if hasattr(search_response_message, 'result') and hasattr(search_response_message.result, 'text') else str(search_response_message)
    print("\nSearch results from SystemAgent:")
    print(search_response_text) 
    
    print("\n2. DEMONSTRATING CREATE COMPONENT TOOL")
    created_component_name = "AdvancedInvoiceExtractor_OpenAI_Demo"
    # Make this prompt extremely focused on JUST the creation task.
    create_prompt = f"""
    Your SOLE task is to use your CreateComponentTool to create a new component with the following exact specifications:
    Name: "{created_component_name}"
    Record Type: TOOL
    Domain: finance
    Framework: openai-agents
    Description: OpenAI Tool to extract detailed invoice info, including line items, and validate calculations.
    Requirements: Must extract invoice number, date, vendor, line items (name, price, quantity, total for each), subtotal, tax, and total_due. Must validate subtotal + tax = total_due. Output should be clean JSON. Code should be suitable for OpenAI Agents SDK.
    
    Your final response for this entire interaction MUST be ONLY the raw JSON output that the CreateComponentTool itself returns. This JSON will contain the 'record_id'. Do not add any surrounding text, explanation, or summary.
    """
    print(f"\nCreating a new component: {created_component_name}...")
    create_response_message = await system_agent.run(create_prompt)
    create_response_text = create_response_message.result.text if hasattr(create_response_message, 'result') and hasattr(create_response_message.result, 'text') else str(create_response_message)
    print("\nComponent creation response from SystemAgent (should be direct JSON from CreateComponentTool):")
    print(create_response_text)
    
    await asyncio.sleep(2) 
    created_record = await library.find_record_by_name(created_component_name, "TOOL")
    advanced_invoice_extractor_id = None
    if created_record:
        advanced_invoice_extractor_id = created_record["id"]
        print(f"✓ Verified: {created_component_name} found in library with ID: {advanced_invoice_extractor_id}")
    else:
        # Try to extract from the SystemAgent's text if direct find failed
        extracted_id_from_text = extract_id_from_json_text(create_response_text, "record_id")
        if extracted_id_from_text:
            advanced_invoice_extractor_id = extracted_id_from_text
            # Optionally, try to verify this extracted ID against the library
            record_by_extracted_id = await library.find_record_by_id(extracted_id_from_text)
            if record_by_extracted_id and record_by_extracted_id.get("name") == created_component_name:
                 print(f"✓ Verified: {created_component_name} found in library using ID extracted from SystemAgent's response: {advanced_invoice_extractor_id}")
            else:
                print(f"✓ Note: Extracted ID '{advanced_invoice_extractor_id}' from SystemAgent response for '{created_component_name}', but could not definitively match it in library by this ID and name.")
        else:
            print(f"✗ Verification failed: Could not find '{created_component_name}' in library by name, and could not extract its ID from SystemAgent's response.")


    component_name_to_evolve = "InvoiceProcessor_V1" 
    initial_invoice_processor_record = await library.find_record_by_name(component_name_to_evolve, "AGENT")
    
    if initial_invoice_processor_record:
        initial_invoice_processor_id = initial_invoice_processor_record["id"]
        evolved_component_name = f"{component_name_to_evolve}_Evolved_OpenAI_Demo"
        print(f"\n3. DEMONSTRATING EVOLVE COMPONENT TOOL (evolving AGENT {component_name_to_evolve} ID: {initial_invoice_processor_id})")
        # Make this prompt extremely focused on JUST the evolution task.
        evolve_prompt = f"""
        Your SOLE task is to use your EvolveComponentTool to evolve the AGENT component with ID "{initial_invoice_processor_id}"
        into a new version specifically named "{evolved_component_name}".
        Changes needed:
        - Better structured JSON output for its processing results.
        - Validation of calculations (subtotal + tax = total) within its logic.
        - Detection of due dates and payment terms.
        - Improved error handling.
        - Ensure it remains compatible with OpenAI Agents SDK.
        
        Your final response for this entire interaction MUST be ONLY the raw JSON output that the EvolveComponentTool itself returns. This JSON will contain the 'evolved_id'. Do not add any surrounding text, explanation, or summary.
        """
        print(f"\nEvolving {component_name_to_evolve}...")
        evolve_response_message = await system_agent.run(evolve_prompt)
        evolve_response_text = evolve_response_message.result.text if hasattr(evolve_response_message, 'result') and hasattr(evolve_response_message.result, 'text') else str(evolve_response_message)
        print("\nComponent evolution response from SystemAgent (should be direct JSON from EvolveComponentTool):")
        print(evolve_response_text)

        await asyncio.sleep(2) 
        # Primary verification: find by new name and parent_id
        evolved_records_by_parent_and_name = await library.components_collection.find({"parent_id": initial_invoice_processor_id, "name": evolved_component_name}).sort("created_at", -1).to_list(length=1)
        evolved_invoice_processor_id = None 
        if evolved_records_by_parent_and_name:
            evolved_invoice_processor_id = evolved_records_by_parent_and_name[0]["id"]
            print(f"✓ Verified: {component_name_to_evolve} evolved in library. New component ID: {evolved_invoice_processor_id}, Name: {evolved_records_by_parent_and_name[0]['name']}")
        else:
            # Fallback: Try finding by just the new name
            evolved_record_by_name = await library.find_record_by_name(evolved_component_name, "AGENT")
            if evolved_record_by_name and evolved_record_by_name.get("parent_id") == initial_invoice_processor_id :
                evolved_invoice_processor_id = evolved_record_by_name["id"]
                print(f"✓ Verified (by name): {component_name_to_evolve} evolved. New component ID: {evolved_invoice_processor_id}, Name: {evolved_record_by_name['name']}")
            else:
                print(f"✗ Verification failed: Could not find evolved version of '{component_name_to_evolve}' named '{evolved_component_name}' in library. SystemAgent might not have completed the EvolveComponentTool call successfully or named it differently.")
    else:
        print(f"\n3. SKIPPING EVOLVE COMPONENT TOOL (initial component {component_name_to_evolve} not found)")
    
    print("\n4. USING SYSTEM AGENT WITH LIBRARY COMPONENTS")
    # Use names that should exist based on successful prior steps or setup
    created_tool_name_for_process = created_component_name # Name from create step
    evolved_agent_name_for_process = evolved_component_name if initial_invoice_processor_record else "InvoiceProcessor_V1"

    process_prompt = f"""
    Use the best available component in the SmartLibrary to process this invoice:
    ```
    {SAMPLE_INVOICE}
    ```
    First, use SearchComponentTool to find appropriate invoice processing components (prefer OpenAI agents like "InvoiceProcessor_V1" or its evolution "{evolved_agent_name_for_process}").
    Then, use the most suitable one. If a tool like "{created_tool_name_for_process}" was created and is suitable, consider it.
    Extract all information.
    If no suitable component is found after searching, use CreateComponentTool to create one (prefer OpenAI framework for an AGENT named "FallbackInvoiceAgent_OpenAI_Demo"), then use it.
    If you use CreateComponentTool or EvolveComponentTool as the primary action of your response, your final response for that step is ONLY its raw JSON output. For execution of a processing component, provide a summary of the extracted invoice data.
    """
    print("\nProcessing invoice using SmartLibrary components...")
    process_response_message = await system_agent.run(process_prompt)
    process_response_text = process_response_message.result.text if hasattr(process_response_message, 'result') and hasattr(process_response_message.result, 'text') else str(process_response_message)
    print("\nProcessing result:")
    print(process_response_text)


async def setup_evolution_demo_library(library: SmartLibrary):
    print("\n[SMART LIBRARY DEMO SETUP]")
    demo_component_names = [
        "InvoiceProcessor_V1", "InvoiceProcessor_V1_Evolved_OpenAI_Demo", 
        "AdvancedInvoiceExtractor_OpenAI_Demo", "BasicInvoiceParser", 
        "SimpleContractAnalyzer", "MedicalRecordProcessor_OpenAI_Demo",
        "DemoInvoiceProcessorTool_OpenAI_TextDemo", 
        "FallbackInvoiceAgent_OpenAI_Demo" 
    ]
    delete_count = 0
    if library.components_collection is not None:
        for name in demo_component_names:
            result = await library.components_collection.delete_many({"name": name})
            if result.deleted_count > 0:
                delete_count += result.deleted_count
                print(f"    Cleaned up {result.deleted_count} records with name '{name}'.")
    print(f"  MongoDB: Cleaned up {delete_count} demo components.")
    print("Setting up initial OpenAI agent and tools for evolution demo...")
    
    await library.create_record(
        name="InvoiceProcessor_V1", record_type="AGENT", domain="finance",
        description="OpenAI agent for processing invoice documents",
        code_snippet="""from agents import Agent, Runner, ModelSettings\nagent = Agent(name="InvoiceProcessor", instructions='Extract key invoice details from the provided text. Focus on invoice number, date, vendor, items with quantities and prices, subtotal, tax, and total due. Format as structured JSON.', model="gpt-4o-mini")\nasync def p(t): result = await Runner.run(agent, input=t); return result.final_output""",
        metadata={"framework": "openai-agents", "model": "gpt-4o-mini"}, tags=["openai", "invoice", "finance"]
    )
    print("✓ Created initial InvoiceProcessor_V1 agent")
    
    await library.create_record(
        name="BasicInvoiceParser", record_type="TOOL", domain="finance",
        description="Simple tool for parsing basic invoice information using regex patterns",
        code_snippet='import re\ndef p(t): return {"invoice_number": (m.group(1) if (m:=re.search(r"INVOICE #(\d+)",t)) else None)}',
        tags=["invoice", "parser", "finance", "regex"]
    )
    print("✓ Created BasicInvoiceParser tool")
    
    await library.create_record(
        name="SimpleContractAnalyzer", record_type="TOOL", domain="legal",
        description="Tool for extracting basic information from legal contracts",
        code_snippet='import re\ndef a(t): return {"parties": re.findall(r"([A-Z][A-Za-z\s]+(?:Ltd\.))[^\n]*",t)}',
        tags=["contract", "legal", "analysis"]
    )
    print("✓ Created SimpleContractAnalyzer tool")
    print(f"\nLibrary setup complete for OpenAI demo.")


async def cleanup_openai_demo_environment(container: DependencyContainer):
    logger.info("Starting OpenAI demo environment cleanup...")
    smart_library: Optional[SmartLibrary] = container.get('smart_library')
    mongo_client: Optional[MongoDBClient] = container.get('mongodb_client')

    demo_component_names = [
        "AdvancedInvoiceExtractor_OpenAI_Demo",
        "InvoiceProcessor_V2_OpenAI_Demo", # Evolved from InvoiceProcessor_V1
        "MedicalRecordProcessor_OpenAI_Demo", # Adapted from InvoiceProcessor_V*
        "DemoInvoiceProcessorTool_OpenAI_TextDemo",
        "FallbackInvoiceAgent_OpenAI_Demo",
        "InvoiceProcessor_V1_Evolved_OpenAI_Demo" # Name used in demonstrate_system_agent_tools
    ]

    # Component cleanup using direct collection access, similar to setup_evolution_demo_library
    if smart_library and smart_library.components_collection is not None:
        logger.info("Cleaning up demo components from SmartLibrary using direct collection access...")
        total_components_deleted_this_run = 0
        for component_name in demo_component_names:
            try:
                # Directly use the components_collection for deletion
                result = await smart_library.components_collection.delete_many({"name": component_name})
                if result.deleted_count > 0:
                    logger.info(f"    Cleaned up {result.deleted_count} records with name '{component_name}'.")
                    total_components_deleted_this_run += result.deleted_count
                else:
                    # This case means no records matched the name.
                    logger.info(f"    No records found with name '{component_name}' to delete via direct collection access.")
            except Exception as e:
                import traceback
                logger.error(f"  Error deleting component '{component_name}' via direct collection access: {e}\nTraceback: {traceback.format_exc()}")

        if total_components_deleted_this_run > 0:
            logger.info(f"  Total demo components deleted via direct collection access: {total_components_deleted_this_run}")
    else:
        logger.warning("SmartLibrary or its components_collection not available, skipping demo component cleanup by name.")

    # Cleanup experiences from MongoDB
    # Using hardcoded "eat_agent_experiences" as MongoExperienceStoreTool.DEFAULT_COLLECTION_NAME
    # might not be readily available here without extra imports or setup.
    experiences_collection_name = "eat_agent_experiences"
    if mongo_client and mongo_client.db is not None: # Corrected check
        try:
            experiences_collection = mongo_client.db[experiences_collection_name]
            delete_result = await experiences_collection.delete_many({})
            logger.info(f"Deleted {delete_result.deleted_count} documents from '{experiences_collection_name}' collection.")
        except Exception as e:
            logger.error(f"Error cleaning up '{experiences_collection_name}' collection: {e}")
    else:
        logger.warning(f"MongoDB client or database not available for '{experiences_collection_name}' cleanup.")

    # Remove old local tracker file if it exists
    tracker_path = "openai_agent_experiences.json"
    if os.path.exists(tracker_path):
        try:
            os.remove(tracker_path)
            logger.info(f"Removed old local tracker file: {tracker_path}")
        except Exception as e:
            logger.error(f"Error removing local tracker file '{tracker_path}': {e}")
            
    logger.info("OpenAI demo environment cleanup complete.")


async def main():
    try:
        print("\n" + "="*80 + "\nOPENAI AGENT EVOLUTION DEMONSTRATION (MongoDB Backend)\n" + "="*80)
        
        container = DependencyContainer()
        
        # Initialize MongoDB Client first as it's a foundational service
        mongo_client = None # Define mongo_client here to ensure it's in scope for the finally block of cleanup
        try:
            mongo_uri = os.getenv("MONGODB_URI", eat_config.MONGODB_URI)
            mongo_db_name = os.getenv("MONGODB_DATABASE_NAME", eat_config.MONGODB_DATABASE_NAME)
            mongo_client = MongoDBClient(uri=mongo_uri, db_name=mongo_db_name) 
            await mongo_client.ping_server() 
            container.register('mongodb_client', mongo_client)
            print(f"✓ MongoDB Client initialized: DB='{mongo_client.db_name}'")
        except Exception as e:
            mongo_uri_log = os.getenv("MONGODB_URI", "Not Set") 
            mongo_db_name_log = os.getenv("MONGODB_DATABASE_NAME", "Not Set")
            logger.error(f"CRITICAL: MongoDBClient failed: {e}. URI: {mongo_uri_log}, DB: {mongo_db_name_log}")
            # Attempt to run cleanup even if mongo client failed, for local files.
            # The cleanup function itself should handle mongo_client being None.
            await cleanup_openai_demo_environment(container)
            return # Exit if MongoDB connection fails as it's critical

        # Initialize LLMService
        llm_service = LLMService(
            provider=eat_config.LLM_PROVIDER,
            model=eat_config.LLM_MODEL,
            embedding_model="text-embedding-ada-002", # Explicitly set for this demo
            use_cache=eat_config.LLM_USE_CACHE,
            container=container
        )
        container.register('llm_service', llm_service)
        # Update the log message to reflect the override
        logger.info(
            f"LLMService initialized with provider: {eat_config.LLM_PROVIDER}, "
            f"model: {eat_config.LLM_MODEL}, "
            f"EMBEDDING MODEL OVERRIDDEN TO: 'text-embedding-ada-002', " # Clearly indicate the override
            f"cache enabled: {eat_config.LLM_USE_CACHE}"
        )

        # Initialize SmartLibrary AFTER MongoDBClient and LLMService
        library = SmartLibrary(container=container)
        container.register('smart_library', library)
        print("✓ SmartLibrary initialized and registered.")

        # Now call cleanup_openai_demo_environment as SmartLibrary and other core services are available
        await cleanup_openai_demo_environment(container)

        # If MongoDB failed during the initial try-except and we proceeded to cleanup,
        # mongo_client might be None. We should have exited already if it was critical.
        # This is an additional check, though the one above should catch it.
        if not container.get('mongodb_client'):
             logger.error("MongoDB client not available after cleanup. Exiting demo.")
             return

        # Instantiate Memory Management Tools
        print("  → Initializing Internal Tools for MemoryManagerAgent...")
        experience_store_tool = MongoExperienceStoreTool(mongodb_client=mongo_client, llm_service=llm_service) # mongo_client is from the outer scope
        semantic_search_tool = SemanticExperienceSearchTool(mongodb_client=mongo_client, llm_service=llm_service)
        message_summarization_tool = MessageSummarizationTool(llm_service=llm_service)
        print("✓ Internal Tools for MemoryManagerAgent initialized.") # Adjusted print message

        # Instantiate MemoryManagerAgent
        print("  → Initializing MemoryManagerAgent...")
        memory_manager_agent_memory = TokenMemory(llm_service.chat_model)
        memory_manager_agent = MemoryManagerAgent(
            llm_service=llm_service,
            mongo_experience_store_tool=experience_store_tool,
            semantic_search_tool=semantic_search_tool,
            message_summarization_tool=message_summarization_tool,
            memory_override=memory_manager_agent_memory
        )
        container.register('memory_manager_agent_instance', memory_manager_agent)
        print("✓ MemoryManagerAgent initialized and registered in container.") # Adjusted print message

        # Firmware, AgentBus, etc.
        firmware = Firmware(); container.register('firmware', firmware)
        print("✓ Firmware initialized.")

        agent_bus = SmartAgentBus(container=container); container.register('agent_bus', agent_bus)
        print("✓ SmartAgentBus initialized.")
        
        # Register MemoryManagerAgent with SmartAgentBus
        print("  → Registering MemoryManagerAgent with SmartAgentBus...")
        MEMORY_MANAGER_AGENT_ID = "memory_manager_agent_openai_evolution_demo"
        await agent_bus.register_agent(
            agent_id=MEMORY_MANAGER_AGENT_ID,
            name="MemoryManagerAgent",
            description="Manages agent experiences using MongoDB for storage, retrieval, and summarization of interactions.",
            agent_type="MemoryManagement",
            capabilities=[{
                "id": "process_task",
                "name": "Process Memory Task",
                "description": "Handles storage, retrieval, and summarization of agent experiences."
            }],
            agent_instance=memory_manager_agent,
            embed_capabilities=True
        )
        print(f"✓ MemoryManagerAgent registered with SmartAgentBus under ID: {MEMORY_MANAGER_AGENT_ID}") # Adjusted print

        # ProviderRegistry, AgentFactory
        provider_registry = ProviderRegistry()
        provider_registry.register_provider(OpenAIAgentsProvider(llm_service))
        provider_registry.register_provider(BeeAIProvider(llm_service))
        container.register('provider_registry', provider_registry)
        print("✓ ProviderRegistry initialized and providers registered.")

        agent_factory = AgentFactory(library, llm_service, provider_registry) # library is already initialized
        container.register('agent_factory', agent_factory)
        print("✓ AgentFactory initialized.")
        
        print("\nInitializing System Agent...")
        system_agent = await SystemAgentFactory.create_agent(container=container)
        container.register('system_agent', system_agent)
        print("✓ SystemAgent initialized.")

        # Call setup_evolution_demo_library AFTER SmartLibrary is initialized and cleanup has run
        await setup_evolution_demo_library(library)

        # Initialize library and agent bus from library (late initializations)
        await library.initialize() 
        await agent_bus.initialize_from_library() 
        print("✓ SmartLibrary and SmartAgentBus late initializations complete.")
        
        initial_agent_record = await library.find_record_by_name("InvoiceProcessor_V1", "AGENT")
        initial_agent_id = initial_agent_record["id"] if initial_agent_record else None
        if initial_agent_id: print(f"✓ Found InvoiceProcessor_V1 ID: {initial_agent_id}")
        else: logger.error("CRITICAL: InvoiceProcessor_V1 not found. Demo aborted."); return

        await text_analysis_demo(llm_service, system_agent, library)
        await demonstrate_system_agent_tools(system_agent, library, llm_service)
        
        # --- AGENT EVOLUTION ---
        print("\n" + "="*80 + "\nAGENT EVOLUTION VIA SYSTEM AGENT\n" + "="*80)
        evolved_agent_name = "InvoiceProcessor_V2_OpenAI_Demo" 
        evolution_prompt = f"""
        Your task is to evolve the "InvoiceProcessor_V1" AGENT (ID: {initial_agent_id}).
        1. Use SearchComponentTool to find the agent with ID "{initial_agent_id}". (You can summarize this search step briefly if found, or state if not found).
        2. Then, use EvolveComponentTool to create a new AGENT version specifically named "{evolved_agent_name}" with these improvements:
           - Better structured JSON output for its processing results.
           - Validation of calculations (subtotal + tax = total) within its logic.
           - Detection of due dates and payment terms.
           - Improved error handling.
           - Ensure it remains compatible with OpenAI Agents SDK.
        After successfully calling EvolveComponentTool, your final response for this entire task MUST be ONLY the raw JSON output that EvolveComponentTool itself returns. This JSON output will contain the 'evolved_id'. Do not add any other text or summary to this final JSON output.
        """
        print("\nPrompting System Agent for evolution...")
        evolution_response_message = await system_agent.run(evolution_prompt)
        evolution_response_text = evolution_response_message.result.text if hasattr(evolution_response_message, 'result') and hasattr(evolution_response_message.result, 'text') else str(evolution_response_message)
        print("\nEvolution demonstration response from SystemAgent (for debugging):")
        print(evolution_response_text)
        
        evolved_record = None; evolved_id = None
        await asyncio.sleep(2) 
        evolved_record = await library.find_record_by_name(evolved_agent_name, "AGENT")
        if evolved_record and evolved_record.get("parent_id") == initial_agent_id:
            evolved_id = evolved_record["id"]
            print(f"✓ Verified evolution in SmartLibrary: Original ID {initial_agent_id} -> New ID {evolved_id} (Name: {evolved_record['name']})")
        else: 
            if evolved_record:
                 print(f"✓ Found component named '{evolved_agent_name}' (ID: {evolved_record['id']}), but its parent_id does not match {initial_agent_id} or was not set.")
            else:
                print(f"✗ Could not verify distinct evolved agent '{evolved_agent_name}' in SmartLibrary by querying name and parent_id. SystemAgent might not have completed the evolution correctly.")


        # --- CROSS-DOMAIN ADAPTATION ---
        print("\n" + "="*80 + "\nCROSS-DOMAIN ADAPTATION DEMONSTRATION\n" + "="*80)
        adapted_agent_name = "MedicalRecordProcessor_OpenAI_Demo"
        source_agent_id_for_adaptation = evolved_id if evolved_id and evolved_id != initial_agent_id else initial_agent_id 
        
        if not source_agent_id_for_adaptation:
             logger.error("Cannot proceed with adaptation, source agent ID for adaptation is missing.")
        else:
            adaptation_prompt = f"""
            Adapt an existing document processing agent for medical records.
            1. Use SearchComponentTool to find agent with ID "{source_agent_id_for_adaptation}". (Summarize this search step briefly).
            2. Use EvolveComponentTool with a 'domain_adaptation' strategy to create a new AGENT specifically named "{adapted_agent_name}".
               Target domain: "medical".
               Framework: "openai-agents".
               Changes: Adapt to extract patient ID, name, DOB, visit date, chief complaint, vitals, assessment, and plan from medical records.
               It should process text like: "{SAMPLE_MEDICAL_RECORD[:150]}..."
            After successfully calling EvolveComponentTool for this adaptation, your final response for this entire task MUST be ONLY its raw JSON output, which includes the 'evolved_id'. Do not add any other text or summary.
            """
            print("\nPrompting System Agent for cross-domain adaptation...")
            adaptation_response_message = await system_agent.run(adaptation_prompt)
            adaptation_response_text = adaptation_response_message.result.text if hasattr(adaptation_response_message, 'result') and hasattr(adaptation_response_message.result, 'text') else str(adaptation_response_message)
            print("\nCross-domain adaptation response from SystemAgent (for debugging):")
            print(adaptation_response_text)

            await asyncio.sleep(2)
            adapted_record = await library.find_record_by_name(adapted_agent_name, "AGENT")
            medical_processor_id = adapted_record["id"] if adapted_record else None
            
            if medical_processor_id:
                print(f"✓ Verified: {adapted_agent_name} adapted/created in library with ID: {medical_processor_id}")
                
                test_medical_prompt = f"Use the agent with ID '{medical_processor_id}' (which should be {adapted_agent_name}) to process: {SAMPLE_MEDICAL_RECORD}"
                test_med_res_msg = await system_agent.run(test_medical_prompt)
                print(f"\nTest of {adapted_agent_name}:\n{test_med_res_msg.result.text if hasattr(test_med_res_msg, 'result') else str(test_med_res_msg)}")
            else:
                print(f"✗ Could not verify {adapted_agent_name} ID in library after adaptation attempt by SystemAgent.")
            
        print("\nDemonstration completed successfully!")
        
    except Exception as e:
        logger.error(f"Error in main demo: {str(e)}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(os.path.dirname(script_dir))
    dotenv_path = os.path.join(project_root, '.env') 
    if not os.path.exists(dotenv_path):
        logger.warning(f".env file not found at {dotenv_path}. Trying CWD...")
        dotenv_path_cwd = os.path.join(os.getcwd(), '.env')
        if os.path.exists(dotenv_path_cwd): load_dotenv(dotenv_path_cwd); logger.info(f"Loaded .env from CWD: {dotenv_path_cwd}")
        else: logger.error(f"No .env at {dotenv_path} or {dotenv_path_cwd}.")
    else: load_dotenv(dotenv_path); logger.info(f"Loaded .env from: {dotenv_path}")

    if not os.getenv("MONGODB_URI") or not os.getenv("OPENAI_API_KEY"):
        logger.error("CRITICAL: MONGODB_URI and/or OPENAI_API_KEY missing.")
        logger.error(f"MONGODB_URI: {os.getenv('MONGODB_URI')}, OPENAI_API_KEY: {'SET' if os.getenv('OPENAI_API_KEY') else 'NOT SET'}")
    else:
        asyncio.run(main())