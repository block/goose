# evolving_agents/tools/openai_agents/openai_agent_logger.py

import json
import time
from typing import Dict, Any, List, Optional
from dataclasses import dataclass, field

@dataclass
class OpenAIAgentExperience:
    """Tracks an OpenAI agent's experiences and performance over time."""
    
    agent_id: str
    agent_name: str
    total_invocations: int = 0
    successful_invocations: int = 0
    average_response_time: float = 0.0
    domain_performance: Dict[str, float] = field(default_factory=dict)
    input_patterns: Dict[str, int] = field(default_factory=dict)
    recent_failures: List[Dict[str, Any]] = field(default_factory=list)
    evolution_history: List[Dict[str, Any]] = field(default_factory=list)

class OpenAIAgentLogger:
    """Records and analyzes OpenAI agent performance and experiences."""
    
    def __init__(self, storage_path: str = "openai_agent_experiences.json"):
        self.storage_path = storage_path
        self.experiences = {}
        self._load_experiences()
    
    def _load_experiences(self) -> None:
        """Load agent experiences from storage."""
        try:
            with open(self.storage_path, 'r') as f:
                experiences_data = json.load(f)
                for agent_id, exp_data in experiences_data.items():
                    self.experiences[agent_id] = OpenAIAgentExperience(**exp_data)
        except (FileNotFoundError, json.JSONDecodeError):
            pass
    
    def _save_experiences(self) -> None:
        """Save agent experiences to storage."""
        experiences_data = {agent_id: vars(exp) for agent_id, exp in self.experiences.items()}
        with open(self.storage_path, 'w') as f:
            json.dump(experiences_data, f, indent=2)
    
    def record_invocation(self, agent_id: str, agent_name: str, 
                         domain: str, input_text: str, 
                         success: bool, response_time: float) -> None:
        """Record an agent invocation and its outcome."""
        # Get or create agent experience record
        if agent_id not in self.experiences:
            self.experiences[agent_id] = OpenAIAgentExperience(agent_id=agent_id, agent_name=agent_name)
        
        exp = self.experiences[agent_id]
        
        # Update statistics
        exp.total_invocations += 1
        if success:
            exp.successful_invocations += 1
        
        # Update average response time
        exp.average_response_time = (
            (exp.average_response_time * (exp.total_invocations - 1) + response_time) / 
            exp.total_invocations
        )
        
        # Update domain performance
        if domain not in exp.domain_performance:
            exp.domain_performance[domain] = 1.0 if success else 0.0
        else:
            # Weighted average: recent performance counts more
            exp.domain_performance[domain] = 0.7 * exp.domain_performance[domain] + 0.3 * (1.0 if success else 0.0)
        
        # Track input patterns (simplified)
        input_pattern = self._extract_pattern(input_text)
        exp.input_patterns[input_pattern] = exp.input_patterns.get(input_pattern, 0) + 1
        
        # Record failure if applicable
        if not success:
            exp.recent_failures.append({
                "timestamp": time.time(),
                "domain": domain,
                "input_pattern": input_pattern
            })
            # Keep only recent failures
            if len(exp.recent_failures) > 10:
                exp.recent_failures.pop(0)
        
        # Save updated experiences
        self._save_experiences()
    
    def record_evolution(self, agent_id: str, evolution_type: str, 
                         changes: Dict[str, Any]) -> None:
        """Record an evolution event for an agent."""
        if agent_id not in self.experiences:
            return
        
        self.experiences[agent_id].evolution_history.append({
            "timestamp": time.time(),
            "evolution_type": evolution_type,
            "changes": changes
        })
        
        self._save_experiences()
    
    def get_experience(self, agent_id: str) -> Optional[OpenAIAgentExperience]:
        """Get the experience record for an agent."""
        return self.experiences.get(agent_id)
    
    def _extract_pattern(self, text: str) -> str:
        """Extract a simplified pattern from text to group similar inputs."""
        # Simplified implementation - in practice, use NLP or embeddings
        words = text.lower().split()
        if len(words) <= 5:
            return "_".join(words)
        return "_".join(words[:3]) + "_" + "_".join(words[-2:])