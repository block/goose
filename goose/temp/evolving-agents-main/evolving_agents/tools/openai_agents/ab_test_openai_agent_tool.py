# evolving_agents/tools/openai_agents/ab_test_openai_agent_tool.py

from typing import Dict, Any, List, Optional, Union
from pydantic import BaseModel, Field
import json
import time
import asyncio
import statistics

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.core.llm_service import LLMService
from evolving_agents.agents.agent_factory import AgentFactory
from evolving_agents.providers.registry import ProviderRegistry
from evolving_agents.tools.openai_agents.openai_agent_logger import OpenAIAgentLogger

class ABTestInput(BaseModel):
    """Input schema for the ABTestOpenAIAgentTool."""
    agent_a_id: str = Field(description="ID of the first agent to test")
    agent_b_id: str = Field(description="ID of the second agent to test")
    test_inputs: List[str] = Field(description="List of test inputs to use for comparison")
    domain: str = Field(description="Domain for the test")
    evaluation_criteria: Optional[List[str]] = Field(None, description="List of criteria to evaluate (e.g., accuracy, speed, completeness)")

class ABTestOpenAIAgentTool(Tool[ABTestInput, None, StringToolOutput]):
    """
    Tool for A/B testing two OpenAI agents to compare their performance.
    This tool can help determine if an evolved agent performs better than its predecessor.
    """
    name = "ABTestOpenAIAgentTool"
    description = "Compare two OpenAI agents on the same tasks to measure performance differences"
    input_schema = ABTestInput
    
    def __init__(
        self, 
        smart_library: SmartLibrary, 
        llm_service: LLMService,
        agent_factory: AgentFactory,
        agent_logger: Optional[OpenAIAgentLogger] = None,
        options: Optional[Dict[str, Any]] = None
    ):
        super().__init__(options=options or {})
        self.library = smart_library
        self.llm = llm_service
        self.agent_factory = agent_factory
        self.agent_logger = agent_logger or OpenAIAgentLogger()
    
    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "openai", "ab_test"],
            creator=self,
        )
    
    async def _run(self, input: ABTestInput, options: Optional[Dict[str, Any]] = None, context: Optional[RunContext] = None) -> StringToolOutput:
        """
        Run an A/B test comparing two OpenAI agents.
        
        Args:
            input: The test parameters
        
        Returns:
            StringToolOutput containing the test results in JSON format
        """
        try:
            # Get agent records
            agent_a_record = await self.library.find_record_by_id(input.agent_a_id)
            agent_b_record = await self.library.find_record_by_id(input.agent_b_id)
            
            if not agent_a_record or not agent_b_record:
                return StringToolOutput(json.dumps({
                    "status": "error",
                    "message": "One or both agents not found"
                }, indent=2))
            
            # Create agent instances using the agent factory
            agent_a = await self.agent_factory.create_agent(agent_a_record)
            agent_b = await self.agent_factory.create_agent(agent_b_record)
            
            # Set up evaluation criteria
            criteria = input.evaluation_criteria or ["accuracy", "response_time", "completeness"]
            
            # Run tests
            results_a = []
            results_b = []
            
            for test_input in input.test_inputs:
                # Test Agent A
                start_time_a = time.time()
                result_a = await self.agent_factory.execute_agent(agent_a, test_input)
                response_time_a = time.time() - start_time_a
                
                # Test Agent B
                start_time_b = time.time()
                result_b = await self.agent_factory.execute_agent(agent_b, test_input)
                response_time_b = time.time() - start_time_b
                
                # Evaluate results
                evaluation = await self._evaluate_responses(
                    test_input,
                    result_a["result"],
                    result_b["result"],
                    criteria,
                    input.domain
                )
                
                # Record results
                results_a.append({
                    "input": test_input,
                    "output": result_a["result"],
                    "response_time": response_time_a,
                    "scores": evaluation["scores_a"]
                })
                
                results_b.append({
                    "input": test_input,
                    "output": result_b["result"],
                    "response_time": response_time_b,
                    "scores": evaluation["scores_b"]
                })
                
                # Log the results for future evolution
                if self.agent_logger:
                    self.agent_logger.record_invocation(
                        agent_a_record["id"],
                        agent_a_record["name"],
                        input.domain,
                        test_input,
                        evaluation["winner"] == "A" or evaluation["winner"] == "Tie",
                        response_time_a
                    )
                    
                    self.agent_logger.record_invocation(
                        agent_b_record["id"],
                        agent_b_record["name"],
                        input.domain,
                        test_input,
                        evaluation["winner"] == "B" or evaluation["winner"] == "Tie",
                        response_time_b
                    )
            
            # Calculate aggregate scores
            agent_a_scores = {
                criterion: statistics.mean([result["scores"].get(criterion, 0) for result in results_a])
                for criterion in criteria
            }
            agent_a_scores["average_response_time"] = statistics.mean([result["response_time"] for result in results_a])
            
            agent_b_scores = {
                criterion: statistics.mean([result["scores"].get(criterion, 0) for result in results_b])
                for criterion in criteria
            }
            agent_b_scores["average_response_time"] = statistics.mean([result["response_time"] for result in results_b])
            
            # Determine overall winner
            a_wins = sum(1 for a, b in zip(results_a, results_b) 
                       if sum(a["scores"].values()) > sum(b["scores"].values()))
            b_wins = sum(1 for a, b in zip(results_a, results_b) 
                       if sum(a["scores"].values()) < sum(b["scores"].values()))
            ties = len(results_a) - a_wins - b_wins
            
            # Determine performance difference
            total_a_score = sum(agent_a_scores.values())
            total_b_score = sum(agent_b_scores.values())
            percentage_difference = ((total_b_score - total_a_score) / total_a_score) * 100
            
            # Generate improvement recommendations
            recommendations = await self._generate_improvement_recommendations(
                agent_a_record,
                agent_b_record,
                agent_a_scores,
                agent_b_scores,
                results_a,
                results_b
            )
            
            # Return the results
            return StringToolOutput(json.dumps({
                "status": "success",
                "test_summary": {
                    "agent_a": {
                        "id": agent_a_record["id"],
                        "name": agent_a_record["name"],
                        "version": agent_a_record["version"]
                    },
                    "agent_b": {
                        "id": agent_b_record["id"],
                        "name": agent_b_record["name"],
                        "version": agent_b_record["version"]
                    },
                    "total_tests": len(input.test_inputs),
                    "agent_a_wins": a_wins,
                    "agent_b_wins": b_wins,
                    "ties": ties,
                    "percentage_difference": f"{percentage_difference:.2f}%",
                    "overall_winner": "Agent A" if a_wins > b_wins else "Agent B" if b_wins > a_wins else "Tie"
                },
                "agent_a_scores": agent_a_scores,
                "agent_b_scores": agent_b_scores,
                "detailed_results": {
                    "agent_a": results_a,
                    "agent_b": results_b
                },
                "improvement_recommendations": recommendations
            }, indent=2))
            
        except Exception as e:
            import traceback
            return StringToolOutput(json.dumps({
                "status": "error",
                "message": f"Error running A/B test: {str(e)}",
                "details": traceback.format_exc()
            }, indent=2))
    
    async def _evaluate_responses(
        self,
        test_input: str,
        response_a: str,
        response_b: str,
        criteria: List[str],
        domain: str
    ) -> Dict[str, Any]:
        """Evaluate and compare two responses based on the given criteria."""
        # Use LLM to evaluate the responses
        evaluation_prompt = f"""
        You are an impartial judge evaluating the performance of two AI assistants.
        
        TASK INPUT:
        {test_input}
        
        RESPONSE FROM ASSISTANT A:
        {response_a}
        
        RESPONSE FROM ASSISTANT B:
        {response_b}
        
        DOMAIN: {domain}
        
        Please evaluate both responses on the following criteria (score from 0-10):
        {', '.join(criteria)}
        
        For each criterion, provide:
        1. A score for Assistant A
        2. A score for Assistant B
        3. A brief explanation for the scores
        
        Finally, declare a winner or a tie based on overall performance.
        
        Return your evaluation in JSON format with this structure:
        {{
          "scores_a": {{"criterion1": score, "criterion2": score, ...}},
          "scores_b": {{"criterion1": score, "criterion2": score, ...}},
          "explanations": {{"criterion1": "explanation", ...}},
          "winner": "A", "B", or "Tie",
          "reasoning": "explanation for the overall winner decision"
        }}
        """
        
        evaluation_response = await self.llm.generate(evaluation_prompt)
        
        try:
            # Parse the JSON response
            evaluation = json.loads(evaluation_response)
            return evaluation
        except json.JSONDecodeError:
            # If parsing fails, return a default evaluation
            default_scores = {criterion: 5.0 for criterion in criteria}
            return {
                "scores_a": default_scores,
                "scores_b": default_scores,
                "explanations": {criterion: "Evaluation parsing failed" for criterion in criteria},
                "winner": "Tie",
                "reasoning": "Could not determine a winner due to evaluation parsing failure"
            }
    
    async def _generate_improvement_recommendations(
        self,
        agent_a_record: Dict[str, Any],
        agent_b_record: Dict[str, Any],
        agent_a_scores: Dict[str, float],
        agent_b_scores: Dict[str, float],
        results_a: List[Dict[str, Any]],
        results_b: List[Dict[str, Any]]
    ) -> List[str]:
        """Generate recommendations for further improvements based on test results."""
        # Identify strengths and weaknesses
        a_strengths = [criterion for criterion, score in agent_a_scores.items() 
                      if score > agent_b_scores.get(criterion, 0)]
        b_strengths = [criterion for criterion, score in agent_b_scores.items() 
                      if score > agent_a_scores.get(criterion, 0)]
        
        a_weaknesses = [criterion for criterion, score in agent_a_scores.items() 
                       if score < agent_b_scores.get(criterion, 0)]
        b_weaknesses = [criterion for criterion, score in agent_b_scores.items() 
                       if score < agent_a_scores.get(criterion, 0)]
        
        # Check if one is an evolution of the other
        is_a_evolved_from_b = agent_a_record.get("parent_id") == agent_b_record["id"]
        is_b_evolved_from_a = agent_b_record.get("parent_id") == agent_a_record["id"]
        
        evolution_relationship = None
        if is_a_evolved_from_b:
            evolution_relationship = f"{agent_a_record['name']} is evolved from {agent_b_record['name']}"
        elif is_b_evolved_from_a:
            evolution_relationship = f"{agent_b_record['name']} is evolved from {agent_a_record['name']}"
        
        # Build a recommendation prompt
        recommendation_prompt = f"""
        Based on A/B testing results between two AI agents, generate specific recommendations for further improvements.
        
        AGENT A: {agent_a_record['name']} (Version: {agent_a_record['version']})
        AGENT B: {agent_b_record['name']} (Version: {agent_b_record['version']})
        
        {evolution_relationship if evolution_relationship else "The agents are separate implementations."}
        
        STRENGTHS:
        - Agent A excels in: {', '.join(a_strengths) if a_strengths else "No clear strengths"}
        - Agent B excels in: {', '.join(b_strengths) if b_strengths else "No clear strengths"}
        
        WEAKNESSES:
        - Agent A is weaker in: {', '.join(a_weaknesses) if a_weaknesses else "No clear weaknesses"}
        - Agent B is weaker in: {', '.join(b_weaknesses) if b_weaknesses else "No clear weaknesses"}
        
        SAMPLE TEST ITEMS THAT SHOWED LARGEST DIFFERENCES:
        {chr(10).join([f"- Input: {a['input'][:100]}..." for a, b in zip(results_a[:3], results_b[:3]) if abs(sum(a['scores'].values()) - sum(b['scores'].values())) > 3])}
        
        Provide 3-5 specific, actionable recommendations for further agent evolution that would:
        1. Combine strengths of both agents
        2. Address key weaknesses
        3. Represent a clear improvement over both existing agents
        
        Format each recommendation as a clear instruction that could be used in an evolution strategy.
        """
        
        recommendation_response = await self.llm.generate(recommendation_prompt)
        
        # Parse the recommendations - assume one per line
        recommendations = [line.strip().lstrip('-').strip() 
                          for line in recommendation_response.split('\n') 
                          if line.strip() and line.strip().startswith('-')]
        
        # If no clear recommendations parsed, return the whole response
        if not recommendations:
            recommendations = [recommendation_response.strip()]
            
        return recommendations