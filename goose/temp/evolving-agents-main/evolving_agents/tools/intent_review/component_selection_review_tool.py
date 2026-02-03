# evolving_agents/tools/intent_review/component_selection_review_tool.py

import json
import logging # Added logging import
from typing import Dict, Any, List, Optional

from pydantic import BaseModel, Field

# BeeAI Framework imports for Tool structure
from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

logger = logging.getLogger(__name__) # Added logger

class ComponentResult(BaseModel):
    """A component search result with metadata."""
    id: str
    name: str
    record_type: str
    description: str
    similarity_score: float
    recommendation: Optional[str] = None

class ComponentSelectionInput(BaseModel):
    """Input schema for the ComponentSelectionReviewTool."""
    query: str = Field(description="The original search query")
    task_context: Optional[str] = Field(None, description="The task context for the search")
    components: List[Dict[str, Any]] = Field(description="The component search results to review")
    interactive: bool = Field(True, description="Whether to use interactive review mode")
    allow_none: bool = Field(False, description="Whether 'none of these' is a valid option")

class ComponentSelectionReviewTool(Tool[ComponentSelectionInput, None, StringToolOutput]):
    """
    Tool for reviewing and selecting components from search results.
    Implements a human-in-the-loop approval process for component selection.
    """
    name = "ComponentSelectionReviewTool"
    description = "Get human approval for component selection before using them in workflows"
    input_schema = ComponentSelectionInput

    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "intent_review", "component_selection"],
            creator=self,
        )

    async def _run(self, tool_input: ComponentSelectionInput, options: Optional[Dict[str, Any]] = None,
                  context: Optional[RunContext] = None) -> StringToolOutput: # Changed 'input' to 'tool_input'
        """
        Present component options and get human selection/approval.
        """
        # Format the components for better presentation
        components = []
        for i, comp in enumerate(tool_input.components): # Use 'tool_input'
            try:
                component = ComponentResult(
                    id=comp.get("id", f"unknown_{i}"),
                    name=comp.get("name", f"Component {i}"),
                    record_type=comp.get("record_type", comp.get("type", "unknown")),
                    description=comp.get("description", "No description available"),
                    # Handle potential missing score field gracefully
                    similarity_score=float(comp.get("similarity_score", comp.get("similarity", 0.0))),
                    recommendation=comp.get("recommendation", None)
                )
                components.append(component)
            except Exception as e:
                 logger.warning(f"Skipping component due to formatting error: {comp}. Error: {e}")


        print("\n" + "="*60)
        print("🔍 COMPONENT SELECTION REVIEW 🔍")
        print("="*60)

        print(f"\nSearch Query: {tool_input.query}") # Use 'tool_input'
        if tool_input.task_context: # Use 'tool_input'
            print(f"Task Context: {tool_input.task_context}") # Use 'tool_input'

        if not components:
             print("\nNo components provided for review.")
             # Return a specific status if no components were passed in
             return StringToolOutput(json.dumps({
                 "status": "no_components_provided",
                 "message": "No components were available for selection review.",
                 "selected_components": []
             }, indent=2))

        print(f"\nFound {len(components)} potential components:")

        # Display components with their details
        for i, comp in enumerate(components, 1):
            print(f"\n{i}. [{comp.record_type}] {comp.name}")
            print(f"   Score: {comp.similarity_score:.2f}")
            print(f"   Description: {comp.description[:100]}...")
            if comp.recommendation:
                print(f"   Recommendation: {comp.recommendation}")

        if tool_input.interactive: # Use 'tool_input'
            # Interactive selection
            print("\nPlease select which component(s) to use.")
            print("You can select multiple by entering comma-separated numbers (e.g., '1,3').")
            if tool_input.allow_none: # Use 'tool_input'
                print("Or enter 'none' if none of these components are suitable.")

            valid_selection = False
            selected_indices = []

            while not valid_selection:
                # Use the built-in input() function correctly
                selection = input("\nYour selection: ").strip().lower()

                if selection == 'none' and tool_input.allow_none: # Use 'tool_input'
                    return StringToolOutput(json.dumps({
                        "status": "none_selected",
                        "message": "No components were selected",
                        "selected_components": []
                    }, indent=2))

                try:
                    # Parse comma-separated values
                    if ',' in selection:
                        indices = [int(idx.strip()) for idx in selection.split(',')]
                    elif selection: # Check if selection is not empty before trying int()
                        indices = [int(selection)]
                    else: # Handle empty input
                        print("Please enter a selection.")
                        continue

                    # Validate indices
                    current_valid_indices = []
                    invalid_found = False
                    for idx in indices:
                        if 1 <= idx <= len(components):
                            current_valid_indices.append(idx - 1)  # Convert to 0-based
                        else:
                            print(f"Invalid selection: {idx}. Must be between 1 and {len(components)}.")
                            invalid_found = True
                            break # Stop checking if one invalid index found

                    if not invalid_found and current_valid_indices:
                        selected_indices = current_valid_indices
                        valid_selection = True
                    elif not invalid_found and not current_valid_indices: # e.g., user entered only invalid indices
                         pass # Loop again, error message already printed


                except ValueError:
                    print("Invalid input. Please enter numbers separated by commas, or 'none'.")

            # Get the selected components
            selected_components = [components[idx] for idx in selected_indices]

            # Get any additional comments
            comments = input("Any comments or instructions for using these components? ").strip()

            return StringToolOutput(json.dumps({
                "status": "components_selected",
                "message": f"Selected {len(selected_components)} components",
                "selected_components": [comp.dict() for comp in selected_components],
                "comments": comments
            }, indent=2))

        else:
            # Non-interactive mode (Simplified)
            print("\nNon-interactive mode: Please provide selection via external mechanism.")
            # In a real non-interactive scenario, you'd fetch the selection differently.
            # For this demo, we'll simulate an automatic selection of the top result if available.
            if components:
                selected_components = [components[0]] # Auto-select top result
                logger.info(f"Non-interactive mode: Auto-selected top component: {selected_components[0].name}")
                return StringToolOutput(json.dumps({
                    "status": "components_selected",
                    "message": f"Auto-selected top component in non-interactive mode",
                    "selected_components": [comp.dict() for comp in selected_components]
                }, indent=2))
            else:
                 # This case is unlikely given the check above, but for safety
                 return StringToolOutput(json.dumps({
                    "status": "no_components_provided",
                    "message": "No components were available for auto-selection.",
                    "selected_components": []
                }, indent=2))