# File: src/agents/repository_analysis_agent/agent.py
from typing import Any

import structlog
from langgraph.graph import END, StateGraph

from src.agents.base import AgentResult, BaseAgent
from src.agents.repository_analysis_agent import nodes
from src.agents.repository_analysis_agent.models import AnalysisState

logger = structlog.get_logger()


class RepositoryAnalysisAgent(BaseAgent):
    """
    Agent responsible for inspecting a repository and suggesting Watchflow rules.

    This agent uses a graph-based orchestration (LangGraph) to:
    1. Fetch repository metadata (file tree, languages, etc.).
    2. Analyze PR history for hygiene signals (AI detection, test coverage).
    3. Generate governance rules using an LLM based on gathered context.
    """

    def __init__(self) -> None:
        super().__init__(agent_name="repository_analysis")

    def _build_graph(self) -> Any:
        """
        Constructs the state graph for the analysis workflow.

        Flow:
        1. `fetch_metadata`: Gathers static repo facts (languages, file structure).
        2. `fetch_pr_signals`: Analyzes dynamic history (PR hygiene, AI usage).
        3. `generate_rules`: Synthesizes data into governance recommendations.

        Returns:
            Compiled StateGraph ready for execution.
        """
        workflow: StateGraph[AnalysisState] = StateGraph(AnalysisState)

        # Register Nodes
        workflow.add_node("fetch_metadata", nodes.fetch_repository_metadata)
        workflow.add_node("fetch_pr_signals", nodes.fetch_pr_signals)
        workflow.add_node("generate_report", nodes.generate_analysis_report)
        workflow.add_node("generate_rules", nodes.generate_rule_recommendations)
        workflow.add_node("generate_reasonings", nodes.generate_rule_reasonings)

        # Define Edges (Linear Flow)
        # 1. Gather data → 2. Diagnose problems (report) → 3. Prescribe solutions (rules) → 4. Explain prescriptions (reasonings)
        workflow.set_entry_point("fetch_metadata")
        workflow.add_edge("fetch_metadata", "fetch_pr_signals")
        workflow.add_edge("fetch_pr_signals", "generate_report")
        workflow.add_edge("generate_report", "generate_rules")
        workflow.add_edge("generate_rules", "generate_reasonings")
        workflow.add_edge("generate_reasonings", END)

        return workflow.compile()

    async def execute(self, **kwargs: Any) -> AgentResult:
        """
        Executes the repository analysis workflow.

        Args:
            **kwargs: Must contain `repo_full_name` (str) and optionally:
                - `is_public` (bool): Whether the repo is public
                - `user_token` (str | None): Optional GitHub Personal Access Token for authenticated requests

        Returns:
            AgentResult: Contains the list of recommended rules or error details.

        Raises:
            TimeoutError: If analysis exceeds the 60-second safety limit.
        """
        repo_full_name: str | None = kwargs.get("repo_full_name")
        is_public: bool = kwargs.get("is_public", False)
        user_token: str | None = kwargs.get("user_token")

        if not repo_full_name:
            return AgentResult(success=False, message="repo_full_name is required")

        initial_state = AnalysisState(
            repo_full_name=repo_full_name,
            is_public=is_public,
            user_token=user_token,
            codeowners_content=None,
        )

        try:
            # Execute Graph with 60-second hard timeout
            result = await self._execute_with_timeout(self.graph.ainvoke(initial_state), timeout=60.0)

            # LangGraph returns dict, convert back to AnalysisState
            final_state = AnalysisState(**result) if isinstance(result, dict) else result

            if final_state.error:
                return AgentResult(success=False, message=final_state.error)

            return AgentResult(
                success=True,
                message="Analysis complete",
                data={
                    "recommendations": final_state.recommendations,
                    "hygiene_summary": final_state.hygiene_summary,
                    "rule_reasonings": final_state.rule_reasonings,
                    "analysis_report": final_state.analysis_report,
                },
            )

        except TimeoutError:
            logger.error("agent_execution_timeout", agent="repository_analysis", repo=repo_full_name)
            return AgentResult(success=False, message="Analysis timed out after 60 seconds")
        except Exception as e:
            # Catching Exception here is only for the top-level orchestration safety
            logger.exception("agent_execution_failed", agent="repository_analysis", error=str(e))
            return AgentResult(success=False, message=str(e))
