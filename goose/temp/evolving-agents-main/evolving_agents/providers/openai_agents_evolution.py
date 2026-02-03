# evolving_agents/providers/openai_agents_evolution.py

from typing import Dict, Any, List, Optional, Union
import logging

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.core.llm_service import LLMService
from evolving_agents.agents.agent_factory import AgentFactory
from evolving_agents.tools.openai_agents.openai_agent_logger import OpenAIAgentLogger, OpenAIAgentExperience
from evolving_agents.tools.openai_agents.evolve_openai_agent_tool import EvolveOpenAIAgentTool
from evolving_agents.tools.openai_agents.ab_test_openai_agent_tool import ABTestOpenAIAgentTool

logger = logging.getLogger(__name__)

class OpenAIAgentsEvolutionManager:
    """
    Manager for OpenAI agent evolution processes.
    This class integrates the evolution tools with the core framework.
    """
    
    def __init__(
        self,
        smart_library: SmartLibrary,
        llm_service: LLMService,
        agent_factory: AgentFactory
    ):
        self.library = smart_library
        self.llm = llm_service
        self.agent_factory = agent_factory
        self.agent_logger = OpenAIAgentLogger()
        
        # Initialize tools
        self.evolve_tool = EvolveOpenAIAgentTool(
            smart_library=smart_library,
            llm_service=llm_service,
            agent_logger=self.agent_logger
        )
        
        self.ab_test_tool = ABTestOpenAIAgentTool(
            smart_library=smart_library,
            llm_service=llm_service,
            agent_factory=agent_factory,
            agent_logger=self.agent_logger
        )
        
        logger.info("OpenAI Agents Evolution Manager initialized")
    
    async def analyze_evolution_candidates(self, domain: Optional[str] = None) -> List[Dict[str, Any]]:
        """
        Analyze agents in the library to identify candidates for evolution.
        
        Args:
            domain: Optional domain to filter agents
            
        Returns:
            List of agents with evolution recommendations
        """
        # Get all OpenAI agents
        if domain:
            domain_records = await self.library.find_records_by_domain(domain, "AGENT")
        else:
            # Get all agents
            domain_records = []
            for record in self.library.records:
                if record["record_type"] == "AGENT":
                    domain_records.append(record)
        
        # Filter to OpenAI agents
        openai_agents = [
            record for record in domain_records 
            if record.get("metadata", {}).get("framework", "").lower() == "openai-agents"
        ]
        
        candidates = []
        
        for agent in openai_agents:
            # Get the agent's experience
            experience = self.agent_logger.get_experience(agent["id"])
            
            if not experience:
                continue
                
            # Check if the agent might benefit from evolution
            evolution_recommendation = await self._analyze_agent_for_evolution(agent, experience)
            
            if evolution_recommendation:
                candidates.append({
                    "agent_id": agent["id"],
                    "agent_name": agent["name"],
                    "domain": agent["domain"],
                    "version": agent["version"],
                    "evolution_recommendation": evolution_recommendation
                })
        
        return candidates
    
    async def evolve_agent(
        self,
        agent_id: str,
        changes: str,
        evolution_type: str = "standard",
        target_domain: Optional[str] = None
    ) -> Dict[str, Any]:
        """
        Evolve an OpenAI agent using the EvolveOpenAIAgentTool.
        
        Args:
            agent_id: ID of the agent to evolve
            changes: Description of changes to make
            evolution_type: Type of evolution to perform
            target_domain: Optional target domain if adapting
            
        Returns:
            Result of the evolution operation
        """
        # Create input for the evolution tool
        evolution_input = self.evolve_tool.input_schema(
            agent_id_or_name=agent_id,
            evolution_type=evolution_type,
            changes=changes,
            target_domain=target_domain,
            learning_from_experience=True
        )
        
        # Run the evolution tool
        result = await self.evolve_tool._run(evolution_input)
        
        # Parse and return the result
        try:
            return json.loads(result.get_text_content())
        except json.JSONDecodeError:
            return {
                "status": "error",
                "message": "Failed to parse evolution result",
                "raw_result": result.get_text_content()
            }
    
    async def compare_agents(
        self,
        agent_a_id: str,
        agent_b_id: str,
        test_inputs: List[str],
        domain: str
    ) -> Dict[str, Any]:
        """
        Compare two OpenAI agents using the ABTestOpenAIAgentTool.
        
        Args:
            agent_a_id: ID of the first agent
            agent_b_id: ID of the second agent
            test_inputs: List of test inputs
            domain: Domain for the test
            
        Returns:
            Result of the A/B test
        """
        # Create input for the A/B test tool
        test_input = self.ab_test_tool.input_schema(
            agent_a_id=agent_a_id,
            agent_b_id=agent_b_id,
            test_inputs=test_inputs,
            domain=domain
        )
        
        # Run the A/B test tool
        result = await self.ab_test_tool._run(test_input)
        
        # Parse and return the result
        try:
            return json.loads(result.get_text_content())
        except json.JSONDecodeError:
            return {
                "status": "error",
                "message": "Failed to parse A/B test result",
                "raw_result": result.get_text_content()
            }
    
    async def _analyze_agent_for_evolution(
        self,
        agent: Dict[str, Any],
        experience: OpenAIAgentExperience
    ) -> Optional[Dict[str, Any]]:
        """
        Analyze an agent's experience to determine if it would benefit from evolution.
        
        Args:
            agent: The agent record
            experience: The agent's experience data
            
        Returns:
            Evolution recommendation if applicable, None otherwise
        """
        # Check if the agent has enough experience to warrant evolution
        if experience.total_invocations < 10:
            return None
            
        # Check the success rate
        success_rate = experience.successful_invocations / max(1, experience.total_invocations)
        
        # If success rate is low, recommend evolution
        if success_rate < 0.8:
            return {
                "reason": "Low success rate",
                "suggested_strategy": "aggressive",
                "details": f"Success rate is {success_rate:.2f}, below the 0.8 threshold",
                "changes": "Improve the agent's instructions to handle more cases correctly."
            }
        
        # Check domain performance
        if experience.domain_performance:
            worst_domain = min(experience.domain_performance.items(), key=lambda x: x[1])
            if worst_domain[1] < 0.7:
                return {
                    "reason": "Poor domain performance",
                    "suggested_strategy": "domain_adaptation",
                    "target_domain": worst_domain[0],
                    "details": f"Performance in domain '{worst_domain[0]}' is {worst_domain[1]:.2f}, below the 0.7 threshold",
                    "changes": f"Adapt the agent to handle tasks in the '{worst_domain[0]}' domain more effectively."
                }
        
        # Check for recent failures
        if experience.recent_failures and len(experience.recent_failures) > 3:
            failure_pattern = self._identify_failure_pattern(experience.recent_failures)
            if failure_pattern:
                return {
                    "reason": "Recurring failure pattern",
                    "suggested_strategy": "standard",
                    "details": f"Recurring failure pattern: {failure_pattern}",
                    "changes": f"Improve handling of inputs related to: {failure_pattern}"
                }
        
        # Check if it's been a long time since last evolution
        if (experience.evolution_history and 
            experience.total_invocations > 100 and 
            len(experience.evolution_history) == 0):
            return {
                "reason": "High usage without evolution",
                "suggested_strategy": "standard",
                "details": f"Agent has {experience.total_invocations} invocations but has never been evolved",
                "changes": "General improvements based on usage patterns and common inputs."
            }
            
        return None
    
    def _identify_failure_pattern(self, failures: List[Dict[str, Any]]) -> Optional[str]:
        """Identify common patterns in recent failures."""
        # Simple implementation - check for repeating patterns
        domains = [f.get("domain") for f in failures if "domain" in f]
        patterns = [f.get("input_pattern") for f in failures if "input_pattern" in f]
        
        # Count occurrences
        domain_counts = {}
        for domain in domains:
            domain_counts[domain] = domain_counts.get(domain, 0) + 1
            
        pattern_counts = {}
        for pattern in patterns:
            pattern_counts[pattern] = pattern_counts.get(pattern, 0) + 1
        
        # Find the most common
        if domain_counts:
            most_common_domain = max(domain_counts.items(), key=lambda x: x[1])
            if most_common_domain[1] >= 2:
                return f"domain: {most_common_domain[0]}"
                
        if pattern_counts:
            most_common_pattern = max(pattern_counts.items(), key=lambda x: x[1])
            if most_common_pattern[1] >= 2:
                return f"input pattern: {most_common_pattern[0]}"
                
        return None