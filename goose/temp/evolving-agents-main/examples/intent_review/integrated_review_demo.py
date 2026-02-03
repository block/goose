# examples/intent_review/integrated_review_demo.py

import asyncio
import logging
import os
import json
import uuid
from typing import Dict, Any, List, Optional
import re

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

# MongoDB connection parameters
# Fetch from environment variables with defaults
MONGODB_URI = os.environ.get("MONGODB_URI", "mongodb://localhost:27017")
MONGODB_DB_NAME = os.environ.get("MONGODB_DB_NAME", "evolving_agents_demo")

# Import toolkit components
from evolving_agents.core.dependency_container import DependencyContainer
from evolving_agents.core.llm_service import LLMService
from evolving_agents.core.mongodb_client import MongoDBClient
from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.agent_bus.smart_agent_bus import SmartAgentBus
from evolving_agents.core.system_agent import SystemAgentFactory
from evolving_agents.firmware.firmware import Firmware
from evolving_agents.agents.intent_review_agent import IntentReviewAgentInitializer
from evolving_agents.agents.architect_zero import create_architect_zero
# --- MODIFIED IMPORT: RunContextInput removed as it's not in beeai_framework.context ---
from beeai_framework.context import RunContext

# Import the input model for WorkflowDesignReviewTool
from evolving_agents.tools.intent_review.workflow_design_review_tool import WorkflowDesignInput
# Import the input model for ComponentSelectionReviewTool
from evolving_agents.tools.intent_review.component_selection_review_tool import ComponentSelectionInput


# Set up the environment variable to enable intent review mode
os.environ["INTENT_REVIEW_ENABLED"] = "true"
os.environ["INTENT_REVIEW_OUTPUT_PATH"] = "intent_plan_demo.json" # Corrected path name if desired

# Add this to handle long-running processes with progress indicators
class ProcessDisplay:
    def __init__(self, description: str):
        self.description = description
        self.spinner_chars = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
        self.current_index = 0
        self.running = False
        self._task: Optional[asyncio.Task] = None # Keep track of the task

    async def _spin(self):
        """The spinning animation loop."""
        while self.running:
            spinner = self.spinner_chars[self.current_index % len(self.spinner_chars)]
            print(f"\r{spinner} {self.description}...", end="", flush=True)
            self.current_index += 1
            await asyncio.sleep(0.1)

    async def start(self):
        """Start displaying the progress indicator."""
        if not self.running:
            self.running = True
            # Create and store the task
            self._task = asyncio.create_task(self._spin())
            # Give the task a moment to start printing
            await asyncio.sleep(0.01)

    async def stop(self, message: Optional[str] = None):
        """Stop displaying the progress indicator."""
        if self.running:
            self.running = False
            if self._task:
                self._task.cancel()
                try:
                    # Wait for the task to actually cancel
                    await self._task
                except asyncio.CancelledError:
                    pass # Expected cancellation
                self._task = None # Clear the task reference
            # Clear the line and print the final message
            print("\r" + " " * (len(self.description) + 5), end="\r", flush=True) # Clear line
            if message:
                print(f"✓ {message}")
            else:
                print(f"✓ {self.description} completed.")
            # Ensure the final message is flushed
            await asyncio.sleep(0.01)


# Replace long-running function calls with this pattern
async def with_progress(description: str, coro):
    """Run a coroutine with a progress indicator."""
    display = ProcessDisplay(description)
    await display.start() # Start display before awaiting coro

    try:
        result = await coro
        await display.stop() # Stop display on success
        return result
    except Exception as e:
        import traceback
        logger.error(f"Error during '{description}': {e}\n{traceback.format_exc()}")
        # Ensure stop is called even on error, potentially with an error message
        await display.stop(f"{description} failed: {e}")
        raise # Re-raise the exception after stopping the display


class ConsoleReader:
    """Simple console reader for interactive use."""

    def __iter__(self):
        return self

    def __next__(self) -> str:
        try:
            return input("\nYou 👤: ")
        except (KeyboardInterrupt, EOFError):
            raise StopIteration

    def write(self, prefix: str, message: str) -> None:
        """Print a message with a prefix."""
        # Ensure spinner is cleared before printing new lines
        print("\r" + " " * 80 + "\r", end="") # Clear line heuristically
        print(f"\n{prefix}{message}")


async def setup_environment():
    """Set up the environment for the demo."""
    # Create dependency container
    container = DependencyContainer()

    # Set up core services
    llm_service = LLMService(provider="openai", model="gpt-4o")
    container.register('llm_service', llm_service)

    mongodb_client = MongoDBClient(uri=MONGODB_URI, db_name=MONGODB_DB_NAME)
    container.register('mongodb_client', mongodb_client)

    smart_library = SmartLibrary(container=container)
    container.register('smart_library', smart_library)

    firmware = Firmware()
    container.register('firmware', firmware)

    agent_bus = SmartAgentBus(container=container)
    container.register('agent_bus', agent_bus)

    # Create system agent
    # Wrap individual creations if they are slow, or rely on the parent wrap
    system_agent = await SystemAgentFactory.create_agent(container=container)
    container.register('system_agent', system_agent)

    # Create architect agent
    architect_agent = await create_architect_zero(container=container)
    container.register('architect_agent', architect_agent)

    # Create intent review agent
    intent_review_agent = await IntentReviewAgentInitializer.create_agent(
        llm_service=llm_service,
        container=container
    )
    container.register('intent_review_agent', intent_review_agent)

    # Initialize components
    await smart_library.initialize()
    await agent_bus.initialize_from_library()

    logger.info("Environment setup complete")
    return container

async def seed_demo_data(container):
    """Seed the Smart Library with demo components."""
    smart_library = container.get('smart_library')

    # Create a basic document analyzer tool
    await smart_library.create_record(
        name="BasicDocumentAnalyzer",
        record_type="TOOL",
        domain="document_processing",
        description="A basic tool that analyzes documents to guess their type (invoice, receipt, etc.)",
        code_snippet="""
class BasicDocumentAnalyzer:
    def __init__(self):
        self.document_types = ["invoice", "receipt", "contract", "report"]

    def analyze(self, document_text):
        # Simplified analysis logic
        document_type = "unknown"
        confidence = 0.0

        for doc_type in self.document_types:
            if doc_type in document_text.lower():
                document_type = doc_type
                confidence = 0.7
                break

        return {"type": document_type, "confidence": confidence}
""",
        tags=["document", "analyzer", "text", "classification"]
    )

    # Create an invoice processor tool
    await smart_library.create_record(
        name="InvoiceProcessor",
        record_type="TOOL",
        domain="financial",
        description="A tool for extracting key information from invoice documents, including invoice number, date, vendor, and amounts",
        code_snippet="""
import re
import json

class InvoiceProcessor:
    def __init__(self):
        self.patterns = {
            "invoice_number": r"Invoice\\s*#?\\s*([\\w-]+)",
            "date": r"Date:?\\s*([\\d\\w\\s,/-]+)",
            "vendor": r"Vendor:?\\s*([^\\n]+)",
            "total": r"Total:?\\s*\\$?([\\d,.]+)",
        }

    def process_invoice(self, invoice_text):
        results = {}

        for field, pattern in self.patterns.items():
            match = re.search(pattern, invoice_text, re.IGNORECASE)
            if match:
                results[field] = match.group(1).strip()
            else:
                results[field] = "Not found"

        return results

    def to_json(self, invoice_text):
        results = self.process_invoice(invoice_text)
        return json.dumps(results, indent=2)
""",
        tags=["invoice", "extraction", "financial", "document"]
    )

    # Create a data validator tool
    await smart_library.create_record(
        name="DataValidator",
        record_type="TOOL",
        domain="data_processing",
        description="A tool for validating data formats and performing calculations verification",
        code_snippet="""
class DataValidator:
    def __init__(self):
        self.supported_types = ["invoice", "form", "financial_data"]

    def validate_calculations(self, data):
        # For invoices, verify subtotal + tax = total
        if "subtotal" in data and "tax" in data and "total" in data:
            try:
                subtotal = float(str(data["subtotal"]).replace(",", ""))
                tax = float(str(data["tax"]).replace(",", ""))
                total = float(str(data["total"]).replace(",", ""))

                expected_total = subtotal + tax
                if abs(expected_total - total) < 0.01:  # Allow for rounding errors
                    return True, "Calculations verified"
                else:
                    return False, f"Calculation error: {subtotal} + {tax} = {expected_total}, not {total}"
            except (ValueError, TypeError):
                return False, "Could not parse numerical values for validation"

        return None, "Insufficient data for calculation validation"

    def validate_format(self, data, expected_schema):
        missing_fields = []
        for field in expected_schema:
            if field not in data:
                missing_fields.append(field)

        if missing_fields:
            return False, f"Missing required fields: {', '.join(missing_fields)}"

        return True, "All required fields present"
""",
        tags=["validation", "calculation", "verification", "data_quality"]
    )

    logger.info("Demo data seeded successfully")

async def run_integrated_review_demo():
    """Run the integrated review demo with all review types."""
    print("\n" + "="*60)
    print("INTEGRATED REVIEW WORKFLOW DEMO")
    print("="*60)

    # Set up environment with progress
    container = await with_progress("Setting up environment", setup_environment())

    # Seed demo data with progress
    await with_progress("Seeding demo data", seed_demo_data(container))

    # Get the components we need
    system_agent = container.get('system_agent')
    architect_agent = container.get('architect_agent')
    intent_review_agent = container.get('intent_review_agent')
    reader = ConsoleReader()

    reader.write("🛠️ System: ", "Welcome to the Integrated Human-in-the-Loop Review Demo!")
    reader.write("🛠️ System: ", "This demo showcases three levels of review in the Evolving Agents Toolkit:")
    reader.write("🛠️ System: ", "1. Workflow Design Review - Review the high-level workflow design")
    reader.write("🛠️ System: ", "2. Component Selection Review - Review the components selected for the task")
    reader.write("🛠️ System: ", "3. Intent Plan Review - Review the specific steps to be executed")

    # Present a simple task example
    reader.write("🛠️ System: ", "Example task: 'Create a document processing system for invoices'")

    # Main interaction loop
    for prompt in reader:
        if not prompt.strip():
            continue

        if prompt.lower() in ["exit", "quit"]:
            reader.write("🛠️ System: ", "Exiting the demo. Goodbye!")
            break

        # START DEMO SEQUENCE
        reader.write("🤖 System: ", "Starting the integrated review workflow process...")

        try:
            # PHASE 1: Get a design from ArchitectZero
            reader.write("🤖 System: ", "Phase 1: Consulting ArchitectZero for a solution design...")

            design_request = f"""
            I need a solution design for the following task:

            {prompt}

            Please design a solution that includes:
            1. The components needed (new or existing)
            2. How these components should interact
            3. The workflow steps to achieve the goal

            Remember that we have these existing components:
            - BasicDocumentAnalyzer (TOOL): Analyzes documents to determine their type
            - InvoiceProcessor (TOOL): Extracts key data from invoices
            - DataValidator (TOOL): Validates data formats and calculations

            Return your design as a structured JSON object.
            """

            # Get the design from ArchitectZero with progress
            architect_response = await with_progress(
                "Consulting ArchitectZero for design",
                architect_agent.run(design_request)
            )

            # Extract the design from the response
            design_text = architect_response.result.text
            design_json = None # Initialize
            try:
                # Try to find and parse JSON in the response
                design_match = re.search(r'```json\s*([\s\S]*?)\s*```', design_text)
                if design_match:
                    design_json = json.loads(design_match.group(1))
                else:
                    # Try another approach to find JSON
                    start_idx = design_text.find('{')
                    end_idx = design_text.rfind('}')
                    if start_idx >= 0 and end_idx > start_idx:
                        design_json = json.loads(design_text[start_idx:end_idx+1])
                    else:
                         # Fallback: Attempt to parse the whole string if it looks like JSON
                        try:
                            design_json = json.loads(design_text)
                            logger.warning("Parsed entire ArchitectZero response as JSON.")
                        except json.JSONDecodeError:
                           raise ValueError("Could not extract JSON design from ArchitectZero response")

                reader.write("🤖 ArchitectZero: ", "I've designed a solution. Let's review it together.")

                # PHASE 2: Review the workflow design
                reader.write("🤖 System: ", "Phase 2: Reviewing the workflow design...")

                # Use the WorkflowDesignReviewTool to review the design
                workflow_review_tool = intent_review_agent.tools_map["workflow_review"]

                # Use the correct input model from the tool itself
                workflow_review_input = workflow_review_tool.input_schema(
                    design=design_json,
                    interactive=True
                )

                # Run the review with progress
                reader.write("🤖 IntentReviewAgent: ", "Let's review the proposed workflow design.")
                workflow_review_result = await with_progress(
                    "Running workflow design review",
                    workflow_review_tool._run(tool_input=workflow_review_input) # Pass 'tool_input'
                )

                # Parse the review result
                workflow_review_data = json.loads(workflow_review_result.get_text_content())

                if workflow_review_data.get("status") != "approved":
                    reader.write("🤖 System: ", "The workflow design was rejected. Please try again with a different request.")
                    continue

                reader.write("🤖 System: ", "The workflow design was approved. Proceeding to component selection.")

                # PHASE 3: Component Selection Review
                reader.write("🤖 System: ", "Phase 3: Reviewing component selections...")

                # Simulate component search results based on the design
                components_to_review = []

                # Add existing components from our library
                components_to_review.append({
                    "id": "doc_analyzer_1",
                    "name": "BasicDocumentAnalyzer",
                    "record_type": "TOOL",
                    "description": "A basic tool that analyzes documents to guess their type (invoice, receipt, etc.)",
                    "similarity_score": 0.85,
                    "recommendation": "reuse"
                })

                components_to_review.append({
                    "id": "invoice_proc_1",
                    "name": "InvoiceProcessor",
                    "record_type": "TOOL",
                    "description": "A tool for extracting key information from invoice documents, including invoice number, date, vendor, and amounts",
                    "similarity_score": 0.92,
                    "recommendation": "reuse"
                })

                components_to_review.append({
                    "id": "data_validator_1",
                    "name": "DataValidator",
                    "record_type": "TOOL",
                    "description": "A tool for validating data formats and performing calculations verification",
                    "similarity_score": 0.78,
                    "recommendation": "reuse"
                })

                # Add some simulated "new" components that might be created
                components_to_review.append({
                    "id": "new_component_1",
                    "name": "AdvancedInvoiceAnalyzer",
                    "record_type": "AGENT",
                    "description": "A more advanced agent for comprehensive invoice analysis and processing",
                    "similarity_score": 0.65,
                    "recommendation": "create"
                })

                # Use the ComponentSelectionReviewTool to review the components
                component_review_tool = intent_review_agent.tools_map["component_review"]

                # Use the correct input model from the tool itself
                component_review_input = component_review_tool.input_schema(
                    query="invoice processing components",
                    task_context="Building an invoice processing system",
                    components=components_to_review,
                    interactive=True,
                    allow_none=True
                )

                # Run the review with progress
                reader.write("🤖 IntentReviewAgent: ", "Now let's review the components we'll use for this task.")
                component_review_result = await with_progress(
                    "Running component selection review",
                    component_review_tool._run(tool_input=component_review_input) # Pass 'tool_input'
                )

                # Parse the review result
                component_review_data = json.loads(component_review_result.get_text_content())

                if component_review_data.get("status") == "none_selected":
                    reader.write("🤖 System: ", "No components were selected. Please try again with a different request.")
                    continue

                selected_components = component_review_data.get("selected_components", [])
                reader.write("🤖 System: ", f"Selected {len(selected_components)} components. Proceeding to intent plan generation.")

                # PHASE 4: Generate and review the intent plan
                reader.write("🤖 System: ", "Phase 4: Generating and reviewing the intent plan...")

                # Create a simplified intent plan based on the design and selected components
                # Use selected component names where appropriate
                # This plan generation is simplified for the demo
                plan_intents = []
                intent_deps = []
                # Define initial input variable name consistently
                current_input_var = "input_document"

                # Dynamically build intents based on selected components (simple example)
                for i, comp in enumerate(selected_components):
                    intent_id = f"intent_{uuid.uuid4().hex[:8]}"
                    output_var = f"step_{i+1}_output" # Generate consistent output variable name

                    # Determine action based on component (simplified mapping)
                    action = "analyze" # Default action
                    params = {}
                    if comp.get("name") == "BasicDocumentAnalyzer":
                         action = "analyze"
                         params = {"document_text": f"{{{{{current_input_var}}}}}"}
                         output_var = "document_type" # More specific output
                    elif comp.get("name") == "InvoiceProcessor":
                        action = "process_invoice"
                        params = {"invoice_text": f"{{{{{current_input_var}}}}}"}
                        output_var = "invoice_data" # More specific output
                    elif comp.get("name") == "DataValidator":
                         action = "validate_format_and_calculations" # Assume combined action
                         params = {
                             "data": f"{{{{{current_input_var}}}}}",
                             "expected_schema": ["invoice_number", "date", "vendor", "total"] # Example
                         }
                         output_var = "validation_result" # More specific output
                    elif comp.get("name") == "AdvancedInvoiceAnalyzer": # Handle potential new agent
                         action = "analyze_invoice_comprehensively" # Hypothetical action
                         params = {"invoice_document": f"{{{{{current_input_var}}}}}"}
                         output_var = "advanced_analysis"
                    else: # Fallback for unknown components
                         action = "process"
                         params = {"input_data": f"{{{{{current_input_var}}}}}"}


                    plan_intents.append({
                        "intent_id": intent_id,
                        "step_type": "EXECUTE",
                        "component_type": comp["record_type"],
                        "component_name": comp["name"],
                        "action": action,
                        "params": params,
                        "output_var": output_var, # Include the output var
                        "justification": f"Execute {comp['name']} to {action.replace('_', ' ')}",
                        "depends_on": list(intent_deps), # Copy dependencies from previous step
                        "status": "PENDING"
                    })
                    intent_deps.append(intent_id) # Add current intent as dependency for next
                    current_input_var = output_var # Chain output to next input


                # Add the final return step
                final_return_intent_id = f"intent_{uuid.uuid4().hex[:8]}"
                plan_intents.append({
                    "intent_id": final_return_intent_id,
                    "step_type": "RETURN",
                    "component_name": "SystemAgent", # Or whatever handles final output
                    "action": "return_result",
                    "params": {"value": f"{{{{{current_input_var}}}}}"}, # Return the last output variable
                    "justification": "Return the final result of the workflow",
                    "depends_on": list(intent_deps), # Depends on the last processing step
                    "status": "PENDING"
                })

                intent_plan = {
                    "plan_id": f"plan_{uuid.uuid4().hex[:8]}",
                    "title": "Invoice Processing Workflow",
                    "description": f"Process and validate based on request: {prompt}",
                    "objective": prompt,
                    "intents": plan_intents,
                    "status": "PENDING_REVIEW"
                }


                # Convert to JSON string and back to ensure it's clean
                intent_plan_json = json.dumps(intent_plan, indent=2)

                # Use the ApprovePlanTool to review the intent plan
                plan_approval_tool = intent_review_agent.tools_map["plan_approval"]

                # Create the input for the tool
                plan_approval_input = plan_approval_tool.input_schema(
                    plan_id=intent_plan["plan_id"],
                    interactive_mode=True,
                    use_agent_reviewer=False,  # Use human review
                    output_path="integrated_demo_intent_plan.json" # Use corrected path name
                )

                # --- CORRECTED CONTEXT CREATION ---
                # RunContextInput is removed. Parameters are passed directly to RunContext.
                # Create RunContext with required args
                # Use the plan_approval_tool instance as the 'owner' of this context
                # The tool instance has an emitter attribute.
                # Pass run_params directly, and signal as None for now.
                run_context = RunContext(instance=plan_approval_tool, signal=None, run_params={})

                # Set the intent plan value DIRECTLY in the context dictionary
                run_context.context["intent_plan"] = intent_plan_json
                # --- END CORRECTION ---

                # Run the review with progress
                reader.write("🤖 IntentReviewAgent: ", "Finally, let's review the detailed intent plan before execution.")
                plan_approval_result = await with_progress(
                    "Running intent plan review",
                    # Pass tool_input argument name correctly, and the context
                    plan_approval_tool._run(tool_input=plan_approval_input, context=run_context)
                )

                # Parse the review result
                plan_approval_data = json.loads(plan_approval_result.get_text_content())

                if plan_approval_data.get("status") == "approved":
                    reader.write("🤖 System: ", "The intent plan was approved! Now I can execute it to complete your task.")
                    reader.write("🤖 System: ", "In a real implementation, SystemAgent would now execute each step in the approved plan.")
                    reader.write("🤖 System: ", "Plan execution complete! Your invoice processing system is ready.")
                else:
                    reader.write("🤖 System: ", "The intent plan was rejected. I won't proceed with execution.")
                    reader.write("🤖 System: ", "Please provide a new request or try again with different requirements.")

                reader.write("🤖 System: ", "This completes the integrated human-in-the-loop review demonstration.")

            except json.JSONDecodeError as json_e:
                 logger.error(f"Failed to parse JSON: {json_e}")
                 logger.error(f"Problematic text: {design_text}")
                 reader.write("⚠️ Error: ", f"Could not process the response from ArchitectZero (invalid JSON).")
                 reader.write("🤖 System: ", "Please try again or rephrase your request.")
            except ValueError as ve:
                 logger.error(f"Value error during processing: {ve}")
                 reader.write("⚠️ Error: ", f"Problem during processing: {ve}")
                 reader.write("🤖 System: ", "Let's try again with a different request.")


        except Exception as e:
            # General exception catch moved outside the inner try-except for JSON
            import traceback
            logger.error(f"Error in integrated demo: {str(e)}")
            logger.error(traceback.format_exc())
            reader.write("⚠️ Error: ", f"Something went wrong: {str(e)}")
            reader.write("🤖 System: ", "Let's try again with a different request.")


if __name__ == "__main__":
    # Ensure output directory exists if needed for logs/plans
    output_dir = os.path.dirname(os.environ.get("INTENT_REVIEW_OUTPUT_PATH", "."))
    if output_dir and not os.path.exists(output_dir):
        os.makedirs(output_dir, exist_ok=True)

    try:
        asyncio.run(run_integrated_review_demo())
    except KeyboardInterrupt:
        print("\nDemo interrupted by user. Exiting.")