# evolving_agents/tools/openai_agents/evolve_openai_agent_tool.py

from typing import Dict, Any, List, Optional, Union
from pydantic import BaseModel, Field
import json

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.core.llm_service import LLMService
from evolving_agents.firmware.firmware import Firmware
from evolving_agents.tools.openai_agents.openai_agent_logger import OpenAIAgentLogger

class EvolveOpenAIAgentInput(BaseModel):
    """Input schema for the EvolveOpenAIAgentTool."""
    agent_id_or_name: str = Field(description="ID or name of the OpenAI agent to evolve")
    evolution_type: str = Field(description="Type of evolution to perform (standard, conservative, aggressive, domain_adaptation)")
    changes: str = Field(description="Description of the changes to make or improvements needed")
    target_domain: Optional[str] = Field(None, description="Target domain if adapting to a new domain")
    learning_from_experience: bool = Field(True, description="Whether to analyze the agent's past experiences to guide evolution")

class EvolveOpenAIAgentTool(Tool[EvolveOpenAIAgentInput, None, StringToolOutput]):
    """
    Tool for evolving OpenAI agents based on experiences and requirements.
    This tool uses the agent's performance history and user-specified changes to
    create an improved version of the agent.
    """
    name = "EvolveOpenAIAgentTool"
    description = "Evolves OpenAI agents through different strategies to adapt to new requirements or improve performance"
    input_schema = EvolveOpenAIAgentInput
    
    def __init__(
        self, 
        smart_library: SmartLibrary, 
        llm_service: LLMService,
        firmware: Optional[Firmware] = None,
        agent_logger: Optional[OpenAIAgentLogger] = None,
        options: Optional[Dict[str, Any]] = None
    ):
        super().__init__(options=options or {})
        self.library = smart_library
        self.llm = llm_service
        self.firmware = firmware or Firmware()
        self.agent_logger = agent_logger or OpenAIAgentLogger()
        
        # Define evolution strategies specific to OpenAI agents
        self.evolution_strategies = {
            "standard": {
                "description": "Balanced evolution that enhances instructions while maintaining core behavior",
                "instruction_preservation": 0.7,  # 70% of original instructions preserved
                "prompt_modifier": "Enhance the agent's instructions with new capabilities while preserving its core behavior."
            },
            "conservative": {
                "description": "Minimal changes focused on fixing specific issues while maintaining compatibility",
                "instruction_preservation": 0.9,  # 90% of original instructions preserved
                "prompt_modifier": "Make minor adjustments to the agent's instructions to address specific issues while preserving its behavior."
            },
            "aggressive": {
                "description": "Significant changes to optimize for new requirements",
                "instruction_preservation": 0.4,  # 40% of original instructions preserved
                "prompt_modifier": "Substantially revise the agent's instructions to optimize for the new requirements."
            },
            "domain_adaptation": {
                "description": "Specialized adaptation to a new domain",
                "instruction_preservation": 0.6,  # 60% of original instructions preserved
                "prompt_modifier": "Adapt the agent's instructions for the new domain while maintaining its core capabilities."
            }
        }
    
    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "openai", "evolve_agent"],
            creator=self,
        )
    
    async def _run(self, input: EvolveOpenAIAgentInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        """
        Evolve an OpenAI agent based on the specified strategy and changes.
        
        Args:
            input: The evolution parameters
        
        Returns:
            StringToolOutput containing the evolution result in JSON format
        """
        try:
            # Get the agent record
            agent_record = await self._get_agent_record(input.agent_id_or_name)
            if not agent_record:
                return StringToolOutput(json.dumps({
                    "status": "error",
                    "message": f"Agent '{input.agent_id_or_name}' not found"
                }, indent=2))
            
            # Verify it's an OpenAI agent
            metadata = agent_record.get("metadata", {})
            framework = metadata.get("framework", "").lower()
            
            if framework != "openai-agents":
                return StringToolOutput(json.dumps({
                    "status": "error",
                    "message": f"Agent '{input.agent_id_or_name}' is not an OpenAI agent"
                }, indent=2))
            
            # Get evolution strategy
            strategy = input.evolution_type.lower()
            if strategy not in self.evolution_strategies:
                strategy = "standard"  # Default to standard if invalid
            
            strategy_details = self.evolution_strategies[strategy]
            
            # Extract the current instructions from the agent code
            current_instructions = self._extract_instructions(agent_record["code_snippet"])
            
            # Get the agent's experience if available and requested
            experience_data = {}
            if input.learning_from_experience and self.agent_logger:
                experience = self.agent_logger.get_experience(agent_record["id"])
                if experience:
                    experience_data = {
                        "total_invocations": experience.total_invocations,
                        "success_rate": experience.successful_invocations / max(1, experience.total_invocations),
                        "average_response_time": experience.average_response_time,
                        "domain_performance": experience.domain_performance,
                        "common_inputs": sorted(experience.input_patterns.items(), key=lambda x: x[1], reverse=True)[:5],
                        "recent_failures": experience.recent_failures
                    }
            
            # Get domain-specific firmware content if adapting to a new domain
            target_domain = input.target_domain or agent_record.get("domain", "general")
            firmware_content = self.firmware.get_firmware_prompt(target_domain)
            
            # Generate evolved instructions
            evolved_instructions = await self._generate_evolved_instructions(
                current_instructions,
                input.changes,
                strategy_details,
                experience_data,
                firmware_content,
                target_domain
            )
            
            # Generate the complete evolved code
            evolved_code = self._generate_evolved_code(
                agent_record["code_snippet"],
                evolved_instructions
            )
            
            # Create the evolved record
            evolved_record = await self.library.evolve_record(
                parent_id=agent_record["id"],
                new_code_snippet=evolved_code,
                description=f"{agent_record['description']} (Evolved with {strategy} strategy)",
                status="active"
            )
            
            # Update metadata for the evolved record
            if "metadata" not in evolved_record:
                evolved_record["metadata"] = {}
            
            evolved_record["metadata"].update({
                "framework": "openai-agents",
                "evolution_strategy": strategy,
                "evolution_timestamp": self._get_current_timestamp()
            })
            
            # If adapting to a new domain, update the domain
            if input.target_domain:
                evolved_record["domain"] = input.target_domain
                evolved_record["metadata"]["previous_domain"] = agent_record.get("domain", "general")
            
            # Save the updated record
            await self.library.save_record(evolved_record)
            
            # Record the evolution in the logger
            if self.agent_logger:
                self.agent_logger.record_evolution(
                    agent_record["id"],
                    strategy,
                    {
                        "new_agent_id": evolved_record["id"],
                        "changes": input.changes,
                        "target_domain": target_domain
                    }
                )
            
            # Return success response
            return StringToolOutput(json.dumps({
                "status": "success",
                "message": f"Successfully evolved OpenAI agent '{agent_record['name']}' using '{strategy}' strategy",
                "original_agent_id": agent_record["id"],
                "evolved_agent_id": evolved_record["id"],
                "evolution_strategy": {
                    "name": strategy,
                    "description": strategy_details["description"]
                },
                "evolved_agent": {
                    "name": evolved_record["name"],
                    "version": evolved_record["version"],
                    "domain": evolved_record["domain"]
                }
            }, indent=2))
            
        except Exception as e:
            import traceback
            return StringToolOutput(json.dumps({
                "status": "error",
                "message": f"Error evolving OpenAI agent: {str(e)}",
                "details": traceback.format_exc()
            }, indent=2))
    
    async def _get_agent_record(self, agent_id_or_name: str) -> Optional[Dict[str, Any]]:
        """Get an agent record by ID or name."""
        # Try by ID first
        record = await self.library.find_record_by_id(agent_id_or_name)
        if record:
            return record
        
        # Try by name for AGENT type
        return await self.library.find_record_by_name(agent_id_or_name, "AGENT")
    
    def _extract_instructions(self, code_snippet: str) -> str:
        """Extract the instructions from an OpenAI agent's code snippet."""
        import re
        
        # Look for instructions in triple quotes
        instruction_pattern = r'instructions=(?:"""|\'\'\')(.*?)(?:"""|\'\'\')' 
        match = re.search(instruction_pattern, code_snippet, re.DOTALL)
        
        if match:
            return match.group(1).strip()
        
        # Alternative pattern
        alt_pattern = r'instructions=[\'"](.*?)[\'"]'
        match = re.search(alt_pattern, code_snippet, re.DOTALL)
        
        if match:
            return match.group(1).strip()
        
        return ""
    
    async def _generate_evolved_instructions(
        self,
        current_instructions: str,
        changes: str,
        strategy_details: Dict[str, Any],
        experience_data: Dict[str, Any],
        firmware_content: str,
        target_domain: str
    ) -> str:
        """Generate evolved instructions for the OpenAI agent."""
        prompt_modifier = strategy_details.get("prompt_modifier", "")
        instruction_preservation = strategy_details.get("instruction_preservation", 0.7)
        
        # Prepare experience data string if available
        experience_text = ""
        if experience_data:
            experience_text = f"""
AGENT EXPERIENCE DATA:
- Total Invocations: {experience_data.get('total_invocations', 0)}
- Success Rate: {experience_data.get('success_rate', 0) * 100:.1f}%
- Average Response Time: {experience_data.get('average_response_time', 0):.2f} seconds
- Domain Performance: {', '.join([f"{domain}: {score:.2f}" for domain, score in experience_data.get('domain_performance', {}).items()])}

Common Input Patterns:
{chr(10).join([f"- {pattern}: {count} times" for pattern, count in experience_data.get('common_inputs', [])])}

Recent Failures:
{chr(10).join([f"- Domain: {failure.get('domain')}, Pattern: {failure.get('input_pattern')}" for failure in experience_data.get('recent_failures', [])])}
"""
        
        # Build the prompt for evolving instructions
        evolution_prompt = f"""
{firmware_content}

CURRENT OPENAI AGENT INSTRUCTIONS:
```
{current_instructions}
```

REQUESTED CHANGES:
{changes}

TARGET DOMAIN: {target_domain}

{experience_text}

EVOLUTION STRATEGY:
{strategy_details.get('description', 'Standard evolution')}
Instruction Preservation Level: {instruction_preservation * 100:.0f}%

INSTRUCTIONS:
1. {prompt_modifier}
2. Maintain the agent's role and primary purpose
3. Follow all firmware guidelines for the target domain
4. Focus on improving clarity, effectiveness, and addressing the requested changes
5. Create a clear and well-structured set of instructions for the OpenAI agent

EVOLVED INSTRUCTIONS:
"""
        
        # Generate the evolved instructions
        return await self.llm.generate(evolution_prompt)
    
    def _generate_evolved_code(self, original_code: str, evolved_instructions: str) -> str:
        """Generate the evolved code by replacing the instructions in the original code."""
        import re
        
        # First try to replace triple-quoted instructions
        instruction_pattern = r'(instructions=)(?:"""|\'\'\')(.*?)(?:"""|\'\'\')' 
        if re.search(instruction_pattern, original_code, re.DOTALL):
            return re.sub(
                instruction_pattern,
                f'\\1"""\n{evolved_instructions}\n"""',
                original_code,
                flags=re.DOTALL
            )
        
        # Fall back to replacing single-quoted instructions
        alt_pattern = r'(instructions=)[\'"](.*?)[\'"]'
        if re.search(alt_pattern, original_code, re.DOTALL):
            # For single quotes, escape any internal quotes
            escaped_instructions = evolved_instructions.replace('"', '\\"').replace("'", "\\'")
            return re.sub(
                alt_pattern,
                f'\\1"{escaped_instructions}"',
                original_code,
                flags=re.DOTALL
            )
        
        # If no pattern matches, return original with a warning comment
        return f"""
# WARNING: Could not update instructions automatically
# Please manually replace the instructions with the following:
'''
{evolved_instructions}
'''

{original_code}
"""
    
    def _get_current_timestamp(self) -> str:
        """Get current timestamp in ISO format."""
        from datetime import datetime
        return datetime.utcnow().isoformat()