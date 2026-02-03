# examples/self_improvement/evolve_smart_library.py

import asyncio
import logging
import os
import sys 
import json
import re
from typing import Dict, Any, List, Optional, Union

# Add project root to sys.path to allow imports from evolving_agents
script_dir = os.path.dirname(os.path.abspath(__file__))
project_root = os.path.dirname(os.path.dirname(script_dir))
sys.path.insert(0, project_root)

from dotenv import load_dotenv

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Import toolkit components
from evolving_agents.core.dependency_container import DependencyContainer
from evolving_agents.core.llm_service import LLMService
from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.agent_bus.smart_agent_bus import SmartAgentBus
from evolving_agents.core.system_agent import SystemAgentFactory
from evolving_agents.firmware.firmware import Firmware
from evolving_agents.core.mongodb_client import MongoDBClient 
from evolving_agents import config as eat_config 
from evolving_agents.utils.json_utils import safe_json_dumps # Corrected import

# Load environment variables
if not load_dotenv():
    logger.warning(".env file not found. Relying on environment variables or defaults.")


def extract_code_from_response(response_text: str) -> Optional[str]:
    """Extract code from LLM response using multiple strategies."""
    # Strategy 1: Look for ```python...``` code blocks
    code_pattern = r"```python\s*(.*?)\s*```"
    matches = re.findall(code_pattern, response_text, re.DOTALL)
    
    if matches:
        return matches[0].strip()
    
    # Strategy 2: Look for any code blocks
    generic_code_pattern = r"```\s*(.*?)\s*```"
    matches = re.findall(generic_code_pattern, response_text, re.DOTALL)
    
    if matches:
        # Check if it looks like Python code
        for match in matches:
            # A bit more lenient for Python code detection
            if ('class' in match and 'def' in match) or ('def' in match and 'self' in match) or 'import' in match:
                return match.strip()
    
    # Strategy 3: Look for class definition (more specific to this demo)
    # This pattern attempts to capture the whole class definition, robustly.
    # It looks for 'class EnhancedSmartLibrary' and continues until a line that's likely outside the class
    # or the end of the string. This is still heuristic.
    class_pattern = r"(class\s+EnhancedSmartLibrary[\s\S]*?)(?=\n\n\w|\n# Example usage|\nif __name__ == \"__main__\":|\Z)"
    match_obj = re.search(class_pattern, response_text, re.DOTALL)
    
    if match_obj:
        # Further clean up just in case some trailing non-code text got included by the regex
        class_code = match_obj.group(1).strip()
        # If the LLM includes the "```python" and "```" markers *inside* what it claims is code, remove them.
        if class_code.startswith("```python"):
            class_code = class_code[len("```python"):].strip()
        if class_code.endswith("```"):
            class_code = class_code[:-len("```")].strip()
        if len(class_code.splitlines()) > 3: # Basic check for substance
            logger.info("Extracted code using refined class definition pattern.")
            return class_code

    logger.warning("Could not extract code using primary patterns (```python or specific class regex).")
    
    # Fallback: if the response is ONLY code, as requested by the prompt.
    # This assumes the LLM strictly followed "Your response MUST BE ONLY the complete Python code".
    # We check if the response text itself looks like a Python class block.
    lines = response_text.strip().split('\n')
    if lines and lines[0].strip().startswith("import ") or lines[0].strip().startswith("from ") or lines[0].strip().startswith("class EnhancedSmartLibrary"):
        # Check if it has some typical class structure
        if "class EnhancedSmartLibrary" in response_text and "def __init__" in response_text and "async def semantic_search" in response_text:
            logger.info("Assuming entire response is code due to strict formatting adherence by LLM.")
            return response_text.strip()

    logger.error("Failed to extract meaningful code for EnhancedSmartLibrary from SystemAgent response.")
    return None

async def setup_clean_test_environment(container: DependencyContainer):
    """Cleans the MongoDB collection for this demo."""
    logger.info("Setting up clean test environment for self-improvement demo...")
    smart_library: Optional[SmartLibrary] = container.get('smart_library', None)
    
    if smart_library and smart_library.components_collection is not None:
        # Define names or domains specific to this demo to clear
        demo_component_names = [
            "EnhancedSmartLibrary_GeneratedCode", # Note: name used when saving to library
            "InvoiceProcessor", "ContractAnalyzer", 
            "DocumentClassifier", "CustomerFeedbackAnalyzer", "FinancialReportGenerator"
        ]
        # More targeted cleanup using a specific tag
        demo_tag = "self_improvement_demo" 
        
        deleted_count = 0
        for name in demo_component_names:
            result = await smart_library.components_collection.delete_many({"name": name, "tags": demo_tag})
            deleted_count += result.deleted_count
        
        # General cleanup for the tag if any missed by name
        result_tag_cleanup = await smart_library.components_collection.delete_many({"tags": demo_tag})
        if result_tag_cleanup.deleted_count > 0:
             logger.info(f"Additionally cleaned up {result_tag_cleanup.deleted_count} components with tag '{demo_tag}'.")
             deleted_count += result_tag_cleanup.deleted_count


        logger.info(f"Cleaned up {deleted_count} demo-related components from MongoDB collection '{smart_library.components_collection_name}'.")
    else:
        logger.warning("SmartLibrary or its collection not available, skipping MongoDB cleanup.")

    local_files_to_clean = ["enhanced_smart_library.py", "raw_evolution_response_for_code_extraction.txt", "self_improvement_results.json"]
    for f_path in local_files_to_clean:
        if os.path.exists(f_path):
            try:
                os.remove(f_path)
                logger.info(f"Removed local file: {f_path}")
            except Exception as e:
                logger.warning(f"Could not remove local file {f_path}: {e}")


async def evolve_smart_library():
    """Demonstrate self-improvement by enhancing the SmartLibrary component."""
    
    container = DependencyContainer()
    
    try:
        mongo_uri = os.getenv("MONGODB_URI", eat_config.MONGODB_URI)
        mongo_db_name = os.getenv("MONGODB_DATABASE_NAME", eat_config.MONGODB_DATABASE_NAME)
        mongo_client = MongoDBClient(uri=mongo_uri, db_name=mongo_db_name)
        await mongo_client.ping_server()
        container.register('mongodb_client', mongo_client)
        logger.info(f"MongoDB Client initialized for DB: '{mongo_client.db_name}'")
    except Exception as e:
        logger.error(f"CRITICAL: Failed to initialize MongoDBClient: {e}. Demo cannot proceed.")
        return

    llm_service = LLMService(
        provider=os.getenv("LLM_PROVIDER", eat_config.LLM_PROVIDER),
        model=os.getenv("LLM_MODEL", eat_config.LLM_MODEL),
        embedding_model=os.getenv("LLM_EMBEDDING_MODEL", eat_config.LLM_EMBEDDING_MODEL),
        use_cache=eat_config.LLM_USE_CACHE,
        container=container
    )
    container.register('llm_service', llm_service)
    
    smart_library = SmartLibrary(container=container)
    container.register('smart_library', smart_library)
    
    await setup_clean_test_environment(container)
    
    firmware = Firmware()
    container.register('firmware', firmware)
    
    agent_bus = SmartAgentBus(container=container)
    container.register('agent_bus', agent_bus)
    
    system_agent = await SystemAgentFactory.create_agent(container=container)
    container.register('system_agent', system_agent)
    
    await smart_library.initialize() 
    await agent_bus.initialize_from_library()
    
    logger.info("PHASE 1: Analyzing current SmartLibrary semantic search implementation")
    analysis_prompt = """
    I need you to analyze the semantic search capabilities of the SmartLibrary component, which is a Python class.
    The SmartLibrary has a method `async def semantic_search(self, query: str, task_context: Optional[str] = None, record_type: Optional[str] = None, domain: Optional[str] = None, limit: int = 5, threshold: float = 0.0, task_weight: Optional[float] = 0.7) -> List[Tuple[Dict[str, Any], float, float, float]]:`
    This method uses vector embeddings (content_embedding for query, applicability_embedding for task_context) and MongoDB Atlas Vector Search.
    It currently performs a `$vectorSearch` and then re-ranks/combines scores.
    
    Please analyze limitations of this approach and identify specific enhancement opportunities for the `semantic_search` method.
    Provide concrete technical suggestions for Python code changes within this method that would make semantic search more powerful and accurate.
    Focus on aspects like:
    - Contextual domain boosting (if a `domain` filter is provided, how to boost matching records).
    - Recency weighting (how to factor in `last_updated` field, which is a datetime object).
    - Tag-based boosting (if `tags` field - a list of strings - matches query terms).
    - Multi-field relevance (how to better incorporate text matches on `name`, `description`, `tags` if vector search isn't perfect).
    - Explanation generation for results (a string explaining key reasons for relevance).
    """
    
    analysis_result_message = await system_agent.run(analysis_prompt)
    analysis_text = analysis_result_message.result.text if hasattr(analysis_result_message, 'result') and hasattr(analysis_result_message.result, 'text') else str(analysis_result_message)
    logger.info("Analysis complete. Identified enhancement opportunities:\n%s", analysis_text)
    
    logger.info("\nPHASE 2: Requesting SystemAgent to generate EnhancedSmartLibrary code")
    # The prompt for code generation already exists and is quite specific.
    evolution_prompt = f"""
    Based on the previous analysis of SmartLibrary's semantic_search, create Python code for an enhanced class named `EnhancedSmartLibrary`.
    This class must inherit from `SmartLibrary` (from `evolving_agents.smart_library.smart_library import SmartLibrary`).
    Override the `async def semantic_search(...)` method to incorporate the following improvements we discussed:

    1.  **Contextual Domain Boosting:** If the `domain` parameter is provided in the search, slightly increase the `final_score` of results that match this domain. For example, add a small constant boost (e.g., 0.05 to 0.1) if `doc['domain'] == domain_filter`.
    2.  **Recency Weighting:** Give a small score boost to components that were recently updated. You can use the `last_updated` field (a datetime object). For example, calculate a recency factor (e.g., `1 / (days_since_updated + 1)`) and add a fraction of it to the score. Remember to handle `datetime` objects correctly, likely using `datetime.now(timezone.utc)`.
    3.  **Tag-Based Boosting:** If the `query` string (or parts of it, like tokenized words) matches any of the component's `tags` (a list of strings), apply a small score boost.
    4.  **Improved Score Combination:** Ensure the `final_score` thoughtfully combines the Atlas Vector Search score (`vectorSearchScore` or re-calculated semantic scores), content similarity, task relevance similarity, and these new boosts. All scores should be normalized or combined such that the final score ideally remains between 0 and 1. Be careful with score accumulation to prevent exceeding 1.0.
    5.  **Explanation Generation:** For each result, add a new key `'relevance_explanation'` (string) to the returned document dictionary. This string should briefly state why the item was considered relevant (e.g., "High task context match; matched tags: ['finance', 'report']; recent update boost applied.").

    The `EnhancedSmartLibrary` class should be complete. It will need an `__init__` method that calls `super().__init__(...)`.
    The overridden `semantic_search` method should still call `await super().semantic_search(...)` to get initial candidates from Atlas Vector Search (or the fallback mechanism if Atlas VS fails) and then apply the additional boosting, re-ranking, and explanation logic to these candidates.

    Your response MUST BE ONLY the complete Python code for the `EnhancedSmartLibrary` class, including necessary imports like `datetime`, `re`, `numpy` if used inside the method. Do not include any surrounding text, explanations, or markdown code fences (```python ... ```). Start directly with `import ...` or `class EnhancedSmartLibrary...`.
    """
    
    evolution_result_message = await system_agent.run(evolution_prompt)
    evolution_response_text = evolution_result_message.result.text if hasattr(evolution_result_message, 'result') and hasattr(evolution_result_message.result, 'text') else str(evolution_result_message)
    logger.info("SystemAgent response for EnhancedSmartLibrary code received.")
    # print(f"DEBUG: Raw evolution response text:\n{evolution_response_text}") # For debugging

    logger.info("\nPHASE 3: Extracting and saving the evolved component code")
    evolved_code = extract_code_from_response(evolution_response_text)
    
    if evolved_code:
        enhanced_library_path = "enhanced_smart_library.py" 
        with open(enhanced_library_path, "w") as f:
            f.write(evolved_code)
        logger.info(f"EnhancedSmartLibrary code saved to {enhanced_library_path}")
        
        await smart_library.create_record(
            name="EnhancedSmartLibrary_GeneratedCode",
            record_type="TOOL", 
            domain="core_framework_evolution",
            description="SystemAgent-generated Python code for an EnhancedSmartLibrary with improved semantic search.",
            code_snippet=evolved_code,
            tags=["self_improvement", "semantic_search", "library_evolution", "generated_code", "self_improvement_demo"],
            metadata={"generation_details": "Generated by SystemAgent based on analysis of SmartLibrary."}
        )
        logger.info("EnhancedSmartLibrary code snippet saved as a component in SmartLibrary (MongoDB)")
    else:
        logger.error("Failed to extract EnhancedSmartLibrary code from SystemAgent's response.")
        with open("raw_evolution_response_for_code_extraction.txt", "w") as f:
            f.write(evolution_response_text)
        logger.info("Raw SystemAgent response saved to raw_evolution_response_for_code_extraction.txt for inspection.")

    logger.info("\nPHASE 4: Preparing test data for search comparison")
    test_components_data = [
        {"name": "InvoiceProcessor", "record_type": "AGENT", "domain": "finance", "description": "Agent for processing invoice documents", "code_snippet": "# Invoice code", "tags": ["invoice", "finance", "document", "extraction", "self_improvement_demo"]},
        {"name": "ContractAnalyzer", "record_type": "AGENT", "domain": "legal", "description": "Agent for analyzing legal contracts", "code_snippet": "# Contract code", "tags": ["contract", "legal", "analysis", "self_improvement_demo"]},
        {"name": "DocumentClassifier", "record_type": "TOOL", "domain": "document_processing", "description": "Tool for classifying document types", "code_snippet": "# Classifier code", "tags": ["document", "classification", "ai", "self_improvement_demo"]},
        {"name": "CustomerFeedbackAnalyzer", "record_type": "AGENT", "domain": "customer_service", "description": "Agent for customer feedback sentiment", "code_snippet": "# Feedback code", "tags": ["feedback", "sentiment", "self_improvement_demo"]},
        {"name": "FinancialReportGenerator", "record_type": "TOOL", "domain": "finance", "description": "Tool for generating financial reports", "code_snippet": "# Report gen code", "tags": ["finance", "reports", "self_improvement_demo"]},
    ]
    for comp_data in test_components_data:
        # Ensure the 'self_improvement_demo' tag is present for easy cleanup
        if "self_improvement_demo" not in comp_data.get("tags", []):
            comp_data["tags"] = comp_data.get("tags", []) + ["self_improvement_demo"]
        await smart_library.create_record(**comp_data)
    logger.info(f"Seeded SmartLibrary with {len(test_components_data)} test components.")

    logger.info("\nPHASE 5: Testing and comparing the (conceptual) enhanced semantic search")
    # This prompt remains a conceptual analysis by the SystemAgent
    comparison_prompt = f"""
    You previously generated Python code for an `EnhancedSmartLibrary` class. 
    Assume that class is now available and its `semantic_search` method includes:
    1. Contextual domain boosting.
    2. Recency weighting.
    3. Tag-based boosting.
    4. Multi-field relevance (name, description, tags beyond just vector scores).
    5. Explanation generation for each result.

    The SmartLibrary currently contains these test components (among others):
    - InvoiceProcessor (AGENT, finance domain, tags: ["invoice", "finance", "document", "extraction"])
    - ContractAnalyzer (AGENT, legal domain, tags: ["contract", "legal", "analysis"])
    - DocumentClassifier (TOOL, document_processing domain, tags: ["document", "classification", "ai"])
    - CustomerFeedbackAnalyzer (AGENT, customer_service domain, tags: ["feedback", "sentiment"])
    - FinancialReportGenerator (TOOL, finance domain, tags: ["finance", "reports"])

    For each of these search queries, explain how the `EnhancedSmartLibrary.semantic_search` would likely provide *better or more nuanced results* than a basic semantic search that only uses vector similarity on a combined text field:
    
    Query 1: "financial data extraction from invoices" (Task Context: "Need to automate accounts payable")
    Query 2: "legal document analysis tools with entity extraction" (Domain filter: "legal")
    Query 3: "classify various office documents" (Task Context: "Organizing company knowledge base")
    
    Also, briefly answer:
    a. What other components in the Evolving Agents Toolkit could benefit from similar self-evolution driven by analysis and code generation? (e.g., SystemAgent itself, AgentBus, specific tools)
    b. How could the toolkit itself be enhanced to better automate or streamline this kind of component self-evolution process? (e.g., dedicated 'EvolveComponentCodeTool', metrics-driven evolution triggers)
    
    Provide a comprehensive technical analysis and reasoning.
    """
    
    comparison_result_message = await system_agent.run(comparison_prompt)
    comparison_text = comparison_result_message.result.text if hasattr(comparison_result_message, 'result') and hasattr(comparison_result_message.result, 'text') else str(comparison_result_message)
    logger.info("Search comparison analysis complete.")
    
    final_output = {
        "phase1_analysis_of_smart_library": analysis_text,
        "phase2_system_agent_code_generation_response": evolution_response_text,
        "phase3_extracted_enhanced_smart_library_code": evolved_code if evolved_code else "Error: Code not extracted.",
        "phase5_comparison_analysis_of_enhanced_search": comparison_text,
        "enhanced_library_code_saved_to": "enhanced_smart_library.py" if evolved_code else "None (extraction failed)"
    }
    
    with open("self_improvement_results.json", "w") as f:
        f.write(safe_json_dumps(final_output, indent=2)) 
    
    logger.info("\nSelf-improvement demonstration complete! Results saved to self_improvement_results.json")
    return final_output

if __name__ == "__main__":
    dotenv_path = os.path.join(project_root, '.env') 
    if not os.path.exists(dotenv_path):
        logger.warning(f".env file not found at expected project root: {dotenv_path}. Trying current directory...")
        dotenv_path_cwd = os.path.join(os.getcwd(), '.env')
        if os.path.exists(dotenv_path_cwd):
            load_dotenv(dotenv_path_cwd)
            logger.info(f"Loaded .env from current working directory: {dotenv_path_cwd}")
        else:
            logger.error(f"No .env file found at {dotenv_path} or {dotenv_path_cwd}. Critical environment variables MONGODB_URI and OPENAI_API_KEY might be missing.")
    else:
        load_dotenv(dotenv_path)
        logger.info(f"Loaded .env from: {dotenv_path}")

    if not os.getenv("MONGODB_URI") or not os.getenv("OPENAI_API_KEY"):
        logger.error("CRITICAL: MONGODB_URI and/or OPENAI_API_KEY not found in environment after attempting to load .env.")
        logger.error(f"MONGODB_URI: {os.getenv('MONGODB_URI')}, OPENAI_API_KEY: {'SET' if os.getenv('OPENAI_API_KEY') else 'NOT SET'}")
    else:
        asyncio.run(evolve_smart_library())