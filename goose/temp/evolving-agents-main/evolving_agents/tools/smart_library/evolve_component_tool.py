# evolving_agents/tools/smart_library/evolve_component_tool.py

from typing import Dict, Any, List, Optional, Union
from pydantic import BaseModel, Field
import json # Keep for loading
from datetime import datetime, timezone # For timestamp

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.core.llm_service import LLMService
from evolving_agents.firmware.firmware import Firmware
# Import the corrected safe_json_dumps
from evolving_agents.workflow.process_workflow_tool import safe_json_dumps

class EvolveComponentInput(BaseModel):
    """Input schema for the EvolveComponentTool."""
    parent_id: str = Field(description="ID of the parent component to evolve")
    changes: str = Field(description="Description of changes to make")
    new_requirements: Optional[str] = Field(None, description="New requirements to incorporate")
    new_description: Optional[str] = Field(None, description="Updated description")
    new_version: Optional[str] = Field(None, description="Explicit version number (otherwise incremented)")
    target_domain: Optional[str] = Field(None, description="Target domain for adaptation")
    evolution_strategy: Optional[str] = Field("standard", description="Evolution strategy to use (standard, conservative, aggressive)")

class EvolveComponentTool(Tool[EvolveComponentInput, None, StringToolOutput]):
    """
    Tool for evolving existing components in the Smart Library.
    This tool handles various evolution strategies to adapt components to new requirements
    or different domains. It can preserve or radically change functionality based on
    the selected strategy, while maintaining compatibility with the original component.
    """
    name = "EvolveComponentTool"
    description = "Evolve existing agents and tools with various strategies to adapt them to new requirements or domains"
    input_schema = EvolveComponentInput
    
    def __init__(
        self, 
        smart_library: SmartLibrary, 
        llm_service: LLMService,
        firmware: Optional[Firmware] = None,
        options: Optional[Dict[str, Any]] = None
    ):
        super().__init__(options=options or {})
        self.library = smart_library
        self.llm = llm_service
        self.firmware = firmware or Firmware()
        
        # Define evolution strategies
        self.evolution_strategies = {
            "standard": {
                "description": "Balanced evolution that preserves core functionality while adding new features",
                "preservation_level": 0.7,  # 70% of original functionality preserved
                "prompt_modifier": "Evolve the code to implement the requested changes while preserving most of the original functionality."
            },
            "conservative": {
                "description": "Minimal changes to the original component, focusing on compatibility",
                "preservation_level": 0.9,  # 90% of original functionality preserved
                "prompt_modifier": "Make minimal changes to the code, preserving as much of the original functionality as possible while addressing the specific changes requested."
            },
            "aggressive": {
                "description": "Significant changes to optimize for the new requirements, less focus on preserving original functionality",
                "preservation_level": 0.4,  # 40% of original functionality preserved
                "prompt_modifier": "Reimagine the component with a focus on the new requirements, while maintaining only essential elements of the original code."
            },
            "domain_adaptation": {
                "description": "Specialized for adapting components to new domains",
                "preservation_level": 0.6,  # 60% of original functionality preserved
                "prompt_modifier": "Adapt the code to the new domain, adjusting domain-specific elements while preserving the core logic."
            }
        }
    
    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "library", "evolve"],
            creator=self,
        )
    
    async def _run(self, input: EvolveComponentInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        """
        Evolve an existing component in the Smart Library.
        
        Args:
            input: Evolution parameters
        
        Returns:
            StringToolOutput containing the evolution result in JSON format
        """
        try:
            # First, retrieve the parent record
            parent_record = await self.library.find_record_by_id(input.parent_id)
            if not parent_record:
                return StringToolOutput(safe_json_dumps({ # Use safe_json_dumps
                    "status": "error",
                    "message": f"Parent record with ID {input.parent_id} not found"
                }))
            
            # Select appropriate evolution strategy
            strategy = input.evolution_strategy or "standard"
            
            # For domain adaptation, override to the specialized strategy
            if input.target_domain and input.target_domain != parent_record.get("domain"):
                strategy = "domain_adaptation"
            
            # Get strategy details
            strategy_details = self.evolution_strategies.get(strategy, self.evolution_strategies["standard"])
            
            # Get firmware content for the appropriate domain
            target_domain = input.target_domain or parent_record.get("domain", "general")
            firmware_content = self.firmware.get_firmware_prompt(target_domain)
            
            # Generate evolved code with the selected strategy
            new_code = await self._generate_evolved_code(
                parent_record,
                input.changes,
                input.new_requirements,
                firmware_content,
                target_domain,
                strategy_details
            )
            
            # Create the evolved record
            evolved_record = await self.library.evolve_record(
                parent_id=input.parent_id,
                new_code_snippet=new_code,
                description=input.new_description or parent_record["description"],
                new_version=input.new_version
            )
            
            # Update metadata for the evolved record if it's a domain adaptation
            if input.target_domain and input.target_domain != parent_record.get("domain"):
                evolved_record["domain"] = input.target_domain
                
                # Update metadata to reflect domain adaptation
                if "metadata" not in evolved_record:
                    evolved_record["metadata"] = {}
                
                evolved_record["metadata"]["domain_adaptation"] = {
                    "original_domain": parent_record.get("domain", "unknown"),
                    "target_domain": input.target_domain,
                    "adaptation_timestamp": datetime.now(timezone.utc).isoformat() # Use consistent timestamp
                }
                
                await self.library.save_record(evolved_record)
            
            # Add evolution strategy to metadata
            if "metadata" not in evolved_record:
                evolved_record["metadata"] = {}
            
            evolved_record["metadata"]["evolution_strategy"] = {
                "strategy": strategy,
                "description": strategy_details["description"],
                "preservation_level": strategy_details["preservation_level"],
                "timestamp": datetime.now(timezone.utc).isoformat() # Use consistent timestamp
            }
            
            await self.library.save_record(evolved_record)
            
            # Ensure created_at is an ISO string for JSON dump
            created_at_iso = evolved_record["created_at"]
            if isinstance(created_at_iso, datetime):
                created_at_iso = created_at_iso.isoformat()

            # Return success response
            response_data = {
                "status": "success",
                "message": f"Successfully evolved {parent_record['record_type']} '{parent_record['name']}' using '{strategy}' strategy",
                "parent_id": input.parent_id,
                "evolved_id": evolved_record["id"],
                "strategy": {
                    "name": strategy,
                    "description": strategy_details["description"],
                    "preservation_level": strategy_details["preservation_level"]
                },
                "evolved_record": {
                    "name": evolved_record["name"],
                    "type": evolved_record["record_type"],
                    "domain": evolved_record["domain"],
                    "description": evolved_record["description"],
                    "version": evolved_record["version"],
                    "created_at": created_at_iso # Use ISO string
                },
                "next_steps": [
                    "Test the evolved component to verify it meets the new requirements",
                    "Register the evolved component with the Agent Bus if needed",
                    "Consider further evolution if it doesn't fully meet requirements"
                ]
            }
            return StringToolOutput(safe_json_dumps(response_data)) # Use safe_json_dumps
            
        except Exception as e:
            import traceback
            return StringToolOutput(safe_json_dumps({ # Use safe_json_dumps
                "status": "error",
                "message": f"Error evolving component: {str(e)}",
                "details": traceback.format_exc()
            }))
    
    def _get_current_timestamp(self) -> str: # No longer used, directly using datetime.now().isoformat()
        """Get current timestamp in ISO format."""
        return datetime.now(timezone.utc).isoformat()
    
    async def _generate_evolved_code(
        self,
        record: Dict[str, Any],
        changes: str,
        new_requirements: Optional[str] = None,
        firmware_content: str = "",
        target_domain: Optional[str] = None,
        strategy: Dict[str, Any] = None
    ) -> str:
        """
        Generate evolved code for a component using the specified strategy.
        
        This method contains the logic for evolving components, applying different
        strategies based on the evolution needs.
        
        Args:
            record: Original component record
            changes: Description of changes to make
            new_requirements: New requirements to incorporate
            firmware_content: Firmware content to inject
            target_domain: Target domain for adaptation
            strategy: Strategy details for evolution
            
        Returns:
            Evolved code snippet
        """
        # Use default strategy if none provided
        if not strategy:
            strategy = self.evolution_strategies["standard"]
        
        # Determine framework from metadata
        framework = record.get("metadata", {}).get("framework", "beeai")
        
        # Build the prompt for evolution
        prompt_modifier = strategy.get("prompt_modifier", "")
        domain_instruction = f"TARGET DOMAIN ADAPTATION: Adapt to the {target_domain} domain" if target_domain else ""
        
        evolution_prompt = f"""
        {firmware_content}

        ORIGINAL {record['record_type']} CODE:
        ```python
        {record['code_snippet']}
        ```

        REQUESTED CHANGES:
        {changes}

        {f'NEW REQUIREMENTS TO INCORPORATE: {new_requirements}' if new_requirements else ''}
        {domain_instruction}

        EVOLUTION STRATEGY:
        {strategy.get('description', 'Standard evolution')}
        
        INSTRUCTIONS:
        1. {prompt_modifier}
        2. Ensure the code follows {framework} framework standards
        3. Include appropriate error handling
        4. Follow all firmware guidelines
        5. Maintain compatibility with the original interface
        6. Focus particularly on:
           - Accurate implementation of the requested changes
           - Proper integration with existing functionality
           - Clear documentation of what has changed

        EVOLVED CODE:
        """
        
        # Generate the evolved code
        return await self.llm.generate(evolution_prompt)