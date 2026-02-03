# evolving_agents/tools/intent_review/workflow_design_review_tool.py

import json
import logging
import asyncio
import time # Added for non-interactive timeout
from typing import Dict, Any, List, Optional

from pydantic import BaseModel, Field

# BeeAI Framework imports for Tool structure
from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

logger = logging.getLogger(__name__)

class WorkflowDesignInput(BaseModel):
    """Input schema for the WorkflowDesignReviewTool."""
    design: Dict[str, Any] = Field(description="The workflow design to review")
    interactive: bool = Field(True, description="Whether to use interactive review mode")
    timeout: Optional[int] = Field(600, description="Maximum time to wait for review in seconds")

class WorkflowDesignReviewTool(Tool[WorkflowDesignInput, None, StringToolOutput]):
    """
    Tool for reviewing workflow designs before they're processed into executable steps.
    This allows human oversight at the design stage, before detailed intents are created.
    """
    name = "WorkflowDesignReviewTool"
    description = "Get human approval for a workflow design before generating executable steps"
    input_schema = WorkflowDesignInput

    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "intent_review", "workflow_design"],
            creator=self,
        )

    async def _run(self, tool_input: WorkflowDesignInput, options: Optional[Dict[str, Any]] = None,
                  context: Optional[RunContext] = None) -> StringToolOutput: # Changed 'input' to 'tool_input'
        """
        Get human approval for a workflow design.
        """
        design = tool_input.design # Use 'tool_input'

        # Pretty-print the design for better readability
        design_str = json.dumps(design, indent=2)

        print("\n" + "="*60)
        print("🔍 WORKFLOW DESIGN REVIEW 🔍")
        print("="*60)

        # Display design information
        print(f"\nTitle: {design.get('title', design.get('name', 'Unnamed Workflow'))}")
        print(f"Description: {design.get('description', 'No description provided')}")

        # Display components
        if "components" in design:
            print("\nComponents:")
            for i, component in enumerate(design.get("components", []), 1): # Use .get() for safety
                component_type = component.get("type", "unknown")
                component_name = component.get("name", f"Component {i}")
                component_purpose = component.get("purpose", "No purpose specified")
                print(f"  {i}. [{component_type}] {component_name}: {component_purpose}")

        # Display workflow steps if available
        workflow_data = design.get("workflow") # Get workflow data safely
        if workflow_data:
            sequence = None
            if isinstance(workflow_data, dict):
                sequence = workflow_data.get("sequence")
            elif isinstance(workflow_data, list): # Handle case where workflow itself is the list
                sequence = workflow_data

            if isinstance(sequence, list):
                print("\nWorkflow Sequence/Steps:")
                for i, step in enumerate(sequence, 1):
                    # Try to format step nicely, handle non-string steps
                    step_str = str(step)
                    if isinstance(step, dict):
                        step_type = step.get("type", "EXECUTE") # Default assumption
                        step_name = step.get("name", step.get("item_type", "Unknown"))
                        step_str = f"[{step_type}] {step_name}"
                        if "description" in step: step_str += f": {step['description']}"
                    elif not isinstance(step, str):
                         step_str = json.dumps(step) # Fallback for complex types

                    print(f"  {i}. {step_str}")
            else:
                 logger.warning(f"Workflow data found but 'sequence' is not a list: {type(sequence)}")


        if tool_input.interactive: # Use 'tool_input'
            # Interactive console-based review
            print("\nPlease review this workflow design.")
            print("Does this design meet the requirements for the task?")

            while True:
                # Use the built-in input() function correctly
                choice = input("\nApprove this workflow design? (y/n/d for details): ").strip().lower()

                if choice == 'd':
                    # Show more details
                    print("\nDetailed Workflow Design:")
                    print(design_str)
                    continue

                if choice == 'y':
                    comments = input("Optional comments or suggestions: ").strip()
                    return StringToolOutput(json.dumps({
                        "status": "approved",
                        "message": "Workflow design approved by reviewer",
                        "comments": comments
                    }, indent=2)) # Added indent

                elif choice == 'n':
                    reason = input("Reason for rejection: ").strip()
                    return StringToolOutput(json.dumps({
                        "status": "rejected",
                        "message": "Workflow design rejected by reviewer",
                        "reason": reason
                    }, indent=2)) # Added indent

                else:
                    print("Invalid choice. Please enter 'y' (approve), 'n' (reject), or 'd' (details).")
        else:
            # Non-interactive mode
            print("\nPlease review the workflow design and respond with 'approve' or 'reject [reason]'")

            # This is a simplified version for demonstration - in a production system
            # this might interact with an API or UI
            max_wait_time = tool_input.timeout # Use 'tool_input'
            start_time = time.time()

            while time.time() - start_time < max_wait_time:
                # In a real implementation, this would check an external approval source
                # Here we'll just wait for console input
                try:
                    # Use the built-in input() function correctly
                    response = input("\nResponse (approve/reject): ").strip().lower()

                    if response.startswith("approve"):
                        return StringToolOutput(json.dumps({
                            "status": "approved",
                            "message": "Workflow design approved by reviewer"
                        }, indent=2)) # Added indent
                    elif response.startswith("reject"):
                        reason = response[6:].strip() if len(response) > 6 else "No reason provided"
                        return StringToolOutput(json.dumps({
                            "status": "rejected",
                            "message": "Workflow design rejected by reviewer",
                            "reason": reason
                        }, indent=2)) # Added indent
                    else:
                        print("Invalid response. Please enter 'approve' or 'reject [reason]'.")

                except KeyboardInterrupt:
                    return StringToolOutput(json.dumps({
                        "status": "cancelled",
                        "message": "Review cancelled by user"
                    }, indent=2)) # Added indent

                await asyncio.sleep(0.1)

            # Timeout
            return StringToolOutput(json.dumps({
                "status": "timeout",
                "message": f"Review timed out after {max_wait_time} seconds"
            }, indent=2)) # Added indent