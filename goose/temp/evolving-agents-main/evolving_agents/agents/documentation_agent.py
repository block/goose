# evolving_agents/agents/documentation_agent.py

import logging
import json
from typing import Optional, Dict, Any

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.core.llm_service import LLMService

logger = logging.getLogger(__name__)

class DocumentationAgent:
    """
    Generates documentation for evolved components by comparing them to their
    parent versions and stores this documentation in the SmartLibrary.
    """

    def __init__(self, smart_library: SmartLibrary, llm_service: LLMService):
        if smart_library is None:
            raise ValueError("SmartLibrary instance is required for DocumentationAgent.")
        if llm_service is None:
            raise ValueError("LLMService instance is required for DocumentationAgent.")
            
        self.smart_library = smart_library
        self.llm_service = llm_service
        self.logger = logging.getLogger(__name__)
        self.logger.info("DocumentationAgent initialized.")

    async def generate_and_update_documentation(self, component_id: str) -> bool:
        """
        Generates documentation for the specified component and updates its record
        in the SmartLibrary.

        Args:
            component_id: The ID of the (new) component for which to generate documentation.

        Returns:
            True if documentation was successfully generated and stored, False otherwise.
        """
        self.logger.info(f"Starting documentation generation for component_id: {component_id}")

        try:
            # 1. Fetch Component Data
            component_record = await self.smart_library.find_record_by_id(component_id)
            if not component_record:
                self.logger.error(f"Component {component_id} not found in SmartLibrary.")
                return False
            
            self.logger.debug(f"Successfully fetched component {component_id}.")

            parent_id = component_record.get("parent_id") or component_record.get("metadata", {}).get("evolved_from")
            parent_record = None
            if parent_id:
                parent_record = await self.smart_library.find_record_by_id(parent_id)
                if parent_record:
                    self.logger.debug(f"Successfully fetched parent component {parent_id} for {component_id}.")
                else:
                    self.logger.warning(f"Parent component {parent_id} for {component_id} not found, proceeding without parent context.")
            else:
                self.logger.info(f"Component {component_id} has no parent_id. Generating documentation without parent context.")

            # 2. Prepare Input for LLM
            prompt = self._build_llm_prompt(component_record, parent_record)
            self.logger.debug(f"Generated LLM prompt for {component_id}:\n{prompt}")

            # 3. Generate Documentation with LLM
            llm_response_str = await self.llm_service.generate(prompt)
            if not llm_response_str:
                self.logger.error(f"LLM failed to generate documentation for {component_id}.")
                return False
            
            self.logger.debug(f"LLM response for {component_id}: {llm_response_str}")
            
            try:
                # Expecting JSON output from the LLM
                llm_output = json.loads(llm_response_str)
                change_summary = llm_output.get("change_summary")
                updated_description = llm_output.get("updated_description")

                if not updated_description: # Description is mandatory
                    self.logger.error(f"LLM output for {component_id} is missing 'updated_description'. Output: {llm_response_str}")
                    return False
                if not change_summary and parent_record: # Change summary is expected if parent exists
                    self.logger.warning(f"LLM output for {component_id} is missing 'change_summary' even though parent exists. Output: {llm_response_str}")


            except json.JSONDecodeError:
                self.logger.error(f"Failed to parse LLM JSON response for {component_id}. Response: {llm_response_str}", exc_info=True)
                # Fallback: use the whole response as description if it's not JSON
                updated_description = llm_response_str 
                change_summary = "Could not parse LLM response for change summary." if parent_record else None


            # 4. Store Documentation
            if "metadata" not in component_record:
                component_record["metadata"] = {}
            
            component_record["metadata"]["generated_documentation"] = {
                "change_summary": change_summary,
                "updated_description": updated_description,
                "generated_at": self.llm_service.get_utc_timestamp() # Add timestamp
            }
            
            # Also update the main description field if the LLM provided a good one
            component_record["description"] = updated_description
            
            await self.smart_library.save_record(component_record)
            self.logger.info(f"Successfully generated and stored documentation for component {component_id}.")
            return True

        except Exception as e:
            self.logger.error(f"An error occurred during documentation generation for {component_id}: {e}", exc_info=True)
            return False

    def _build_llm_prompt(self, component_record: Dict[str, Any], parent_record: Optional[Dict[str, Any]] = None) -> str:
        """
        Constructs the prompt for the LLM to generate documentation.
        """
        new_code = component_record.get("code_snippet", "")
        new_desc = component_record.get("description", "") # Current description, might be basic
        new_version = component_record.get("version", "N/A")
        evolution_details = component_record.get("metadata", {}).get("evolution_reason", "") or \
                            component_record.get("metadata", {}).get("changes_summary", "") # From strategist/evolver

        prompt_parts = [
            "You are an expert technical writer tasked with documenting a software component.",
            "The component has been evolved, and I need your help to generate its documentation.",
            "Please provide the output as a single JSON object with two keys: \"change_summary\" and \"updated_description\".",
            "Ensure `updated_description` is comprehensive and accurately reflects the component's functionality based on its code.",
            "If there's no parent component, `change_summary` can be null or a brief note stating it's a new component.",
            "\n--- New Component (Version: " + new_version + ") ---",
            "Code Snippet:",
            "```python\n" + new_code + "\n```",
            f"Current Description (may need improvement): {new_desc}",
        ]
        if evolution_details:
            prompt_parts.append(f"Evolution Context/Reason Provided: {evolution_details}")

        if parent_record:
            old_code = parent_record.get("code_snippet", "")
            old_desc = parent_record.get("description", "")
            old_version = parent_record.get("version", "N/A")
            prompt_parts.extend([
                f"\n--- Parent Component (Version: {old_version}) ---",
                "Code Snippet:",
                "```python\n" + old_code + "\n```",
                f"Description: {old_desc}",
                "\n--- Task ---",
                "1. Analyze the differences between the parent component and the new component (code, description, version, evolution context).",
                "2. Generate a concise 'change_summary' detailing these differences. Focus on what a user or developer of this component would need to know about the evolution.",
                "3. Generate an 'updated_description' for the NEW component. This description should be comprehensive, stand-alone, and accurately reflect its capabilities and purpose based on its new code. If the current description is good, refine it; if not, write a new one. Do NOT just state the changes; provide a full description of the new component."
            ])
        else:
            prompt_parts.extend([
                "\n--- Task ---",
                "1. This is a new component (no parent provided).",
                "2. Set 'change_summary' to a brief note indicating it's a new component or initial version.",
                "3. Generate a comprehensive 'updated_description' for this new component based on its code and existing description. Ensure it's suitable for a developer or user to understand its purpose and functionality."
            ])
        
        prompt_parts.append("\nRemember to provide your response as a single JSON object with keys \"change_summary\" and \"updated_description\".")
        return "\n".join(prompt_parts)

# Example usage (conceptual, would be run in an async context)
async def main():
    # This is a conceptual example.
    # In a real scenario, SmartLibrary and LLMService would be properly initialized
    # with a running MongoDB, and actual LLM API keys/endpoints.

    class MockLLMService:
        async def generate(self, prompt: str) -> str:
            print(f"MockLLMService received prompt:\n{prompt[:300]}...\n")
            # Simulate LLM generating JSON output
            if "no parent provided" in prompt:
                 return json.dumps({
                    "change_summary": "Initial version of the component.",
                    "updated_description": "This is a newly generated, comprehensive description for a component that performs X, Y, and Z based on its provided code. It is designed for efficient processing of data streams."
                })
            else:
                return json.dumps({
                    "change_summary": "Refactored core logic for 15% performance improvement and added support for new data type 'XYZ'. Deprecated method 'old_method'.",
                    "updated_description": "This component now offers enhanced performance for task A due to refactoring. It processes inputs of type P, Q, and new type XYZ. Key methods include 'new_method_a' and 'process_data'. Method 'old_method' has been deprecated."
                })
        def get_utc_timestamp(self):
            from datetime import datetime, timezone
            return datetime.now(timezone.utc).isoformat()


    class MockSmartLibrary:
        async def find_record_by_id(self, record_id: str) -> Optional[Dict[str, Any]]:
            print(f"MockSmartLibrary: find_record_by_id called for {record_id}")
            if record_id == "comp_new_v2":
                return {
                    "id": "comp_new_v2", "name": "MyComponent", "version": "2.0.0",
                    "code_snippet": "def new_feature():\n  return True\n\ndef main_logic_v2():\n  # improved logic\n  pass",
                    "description": "Initial simple description for v2.",
                    "parent_id": "comp_old_v1",
                    "metadata": {"evolution_reason": "Needed new_feature and performance boost."}
                }
            elif record_id == "comp_old_v1":
                return {
                    "id": "comp_old_v1", "name": "MyComponent", "version": "1.0.0",
                    "code_snippet": "def main_logic_v1():\n  # original logic\n  pass",
                    "description": "Original component description for v1."
                }
            elif record_id == "comp_new_v1_no_parent":
                 return {
                    "id": "comp_new_v1_no_parent", "name": "NewComponent", "version": "1.0.0",
                    "code_snippet": "def initial_code():\n  return 'hello'",
                    "description": "Basic new component.",
                    "metadata": {}
                }
            return None

        async def save_record(self, record: Dict[str, Any]) -> str:
            print(f"MockSmartLibrary: save_record called for {record['id']}")
            print(f"  Updated description: {record['description']}")
            print(f"  Generated documentation in metadata: {json.dumps(record.get('metadata', {}).get('generated_documentation'), indent=2)}")
            return record["id"]

    # Setup
    llm_service_mock = MockLLMService()
    smart_library_mock = MockSmartLibrary()
    doc_agent = DocumentationAgent(smart_library=smart_library_mock, llm_service=llm_service_mock)

    # Test case 1: Component with a parent
    print("\n--- Test Case 1: Component with Parent ---")
    success_with_parent = await doc_agent.generate_and_update_documentation("comp_new_v2")
    print(f"Documentation generation successful (with parent): {success_with_parent}")

    # Test case 2: Component without a parent
    print("\n--- Test Case 2: Component without Parent ---")
    success_no_parent = await doc_agent.generate_and_update_documentation("comp_new_v1_no_parent")
    print(f"Documentation generation successful (no parent): {success_no_parent}")
    
    # Test case 3: Component not found
    print("\n--- Test Case 3: Component Not Found ---")
    success_not_found = await doc_agent.generate_and_update_documentation("comp_does_not_exist")
    print(f"Documentation generation successful (not found): {success_not_found}")


if __name__ == "__main__":
    import asyncio
    # To run this example, you might need to adjust imports or run it within your project's async context.
    # Example: asyncio.run(main())
    pass
