from typing import Dict, Any, List, Optional, Tuple
from pydantic import BaseModel, Field
import json
import logging

from beeai_framework.tools.tool import Tool, StringToolOutput
from beeai_framework.context import RunContext
from beeai_framework.emitter.emitter import Emitter

from evolving_agents.smart_library.smart_library import SmartLibrary
from evolving_agents.tools.intent_review.component_selection_review_tool import (
    ComponentSelectionReviewTool,
    ComponentSelectionInput
)

logger = logging.getLogger(__name__)

class SearchInput(BaseModel):
    """Input schema for the SearchComponentTool."""
    query: str = Field(description="Query string to search for")
    task_context: Optional[str] = Field(
        None,
        description="Task context describing how the results will be used"
    )
    record_type: Optional[str] = Field(
        None,
        description="Type of record to search for (AGENT or TOOL)"
    )
    domain: Optional[str] = Field(
        None,
        description="Domain to search within"
    )
    limit: int = Field(
        5,
        description="Maximum number of results to return"
    )
    threshold: float = Field(
        0.0,
        description="Minimum similarity threshold (0.0 to 1.0)"
    )
    with_recommendation: bool = Field(
        True,
        description="Include a recommendation on whether to reuse, evolve, or create based on similarity score"
    )

class SearchComponentTool(Tool[SearchInput, None, StringToolOutput]):
    """
    Tool for searching components in the Smart Library by query, similarity, or name.
    Uses semantic search to find the most relevant components and provides recommendations
    based on similarity scores. Now supports task-specific context for more relevant results.
    """
    name = "SearchComponentTool"
    description = (
        "Search for agents and tools in the library using natural language queries, "
        "task context, and get recommendations based on similarity"
    )
    input_schema = SearchInput

    def __init__(self, smart_library: SmartLibrary, options: Optional[Dict[str, Any]] = None):
        super().__init__(options=options or {})
        self.library = smart_library

    def _create_emitter(self) -> Emitter:
        return Emitter.root().child(
            namespace=["tool", "library", "search"],
            creator=self,
        )

    async def _run(
        self,
        input: SearchInput,
        options: Optional[Dict[str, Any]] = None,
        context: Optional[RunContext] = None
    ) -> StringToolOutput:
        """
        Search the Smart Library for components matching the query and task context.

        Args:
            input: The search input parameters with query and optional task context

        Returns:
            StringToolOutput containing the search results in JSON format with recommendations
        """
        try:
            # Perform semantic search with optional task context
            search_results = await self.library.semantic_search(
                query=input.query,
                task_context=input.task_context,
                record_type=input.record_type,
                domain=input.domain,
                limit=input.limit,
                threshold=input.threshold
            )

            # Format results with similarity details
            formatted_results: List[Dict[str, Any]] = []
            for i, result_tuple in enumerate(search_results):
                record, final_score, content_score, task_score = result_tuple
                result = {
                    "rank": i + 1,
                    "id": record["id"],
                    "name": record["name"],
                    "type": record["record_type"],
                    "domain": record.get("domain", "general"),
                    "description": record["description"],
                    "similarity_score": final_score,
                    "content_score": content_score,
                    "task_score": task_score,
                    "version": record.get("version", "1.0.0")
                }
                if input.with_recommendation:
                    result["recommendation"] = self._get_recommendation(final_score)
                    result["recommendation_reason"] = self._get_recommendation_reason(final_score)
                formatted_results.append(result)

            # Determine overall recommendation based on top result or lack thereof
            recommendation: Dict[str, Any] = {}
            if len(formatted_results) == 0 and input.with_recommendation:
                recommendation = {
                    "overall_recommendation": "create",
                    "reason": "No similar components found. Consider creating a new component."
                }
            elif formatted_results and input.with_recommendation:
                top = formatted_results[0]
                recommendation = {
                    "overall_recommendation": top["recommendation"],
                    "reason": top["recommendation_reason"],
                    "based_on": f"{top['name']} (similarity: {top['similarity_score']:.2f})"
                }

            # Human review integration
            intent_review_enabled = config.INTENT_REVIEW_ENABLED
            if intent_review_enabled and context and hasattr(context, "get_value"):
                human_review = context.get_value("human_review_components", True)
                if human_review and formatted_results:
                    selection_review_tool = ComponentSelectionReviewTool()
                    logger.info("Component search results intercepted for human review")
                    review_input = ComponentSelectionInput(
                        query=input.query,
                        task_context=input.task_context,
                        components=formatted_results,
                        interactive=True,
                        allow_none=True
                    )
                    try:
                        review_result = await selection_review_tool._run(review_input, options, context)
                        review_data = json.loads(review_result.get_text_content())
                        status = review_data.get("status")
                        if status == "components_selected":
                            selected = review_data.get("selected_components", [])
                            if selected:
                                sel_ids = [c["id"] for c in selected]
                                filtered = [r for r in formatted_results if r["id"] in sel_ids]
                                for r in filtered:
                                    r["human_selected"] = True
                                response = {
                                    "query": input.query,
                                    "task_context": input.task_context or "None provided",
                                    "result_count": len(filtered),
                                    "results": filtered,
                                    "human_reviewed": True,
                                    "review_comments": review_data.get("comments", "")
                                }
                                if input.with_recommendation:
                                    response["recommendation"] = {
                                        "overall_recommendation": "human_selected",
                                        "reason": "Components were manually selected by human reviewer",
                                        "comment": review_data.get("comments", "")
                                    }
                                return StringToolOutput(json.dumps(response, indent=2))
                        elif status == "none_selected":
                            response = {
                                "query": input.query,
                                "task_context": input.task_context or "None provided",
                                "result_count": 0,
                                "results": [],
                                "human_reviewed": True,
                                "review_message": "No components were selected by human reviewer"
                            }
                            if input.with_recommendation:
                                response["recommendation"] = {
                                    "overall_recommendation": "create",
                                    "reason": "Human reviewer rejected all suggested components"
                                }
                            return StringToolOutput(json.dumps(response, indent=2))
                    except Exception as err:
                        logger.error(f"Error during component selection review: {err}")
                        # Fallback to normal response

            # Default response if no review or after review fallback
            response = {
                "query": input.query,
                "task_context": input.task_context if input.task_context else "None provided",
                "result_count": len(formatted_results),
                "results": formatted_results,
            }
            if input.with_recommendation:
                response["recommendation"] = recommendation
            return StringToolOutput(json.dumps(response, indent=2))

        except Exception as e:
            import traceback
            return StringToolOutput(json.dumps({
                "error": f"Error searching components: {str(e)}",
                "details": traceback.format_exc()
            }, indent=2))

    def _get_recommendation(self, similarity: float) -> str:
        """Get a recommendation based on similarity score."""
        if similarity >= 0.7:
            return "reuse"
        elif similarity >= 0.3:
            return "evolve"
        return "create"

    def _get_recommendation_reason(self, similarity: float) -> str:
        """Get a reason for the recommendation."""
        if similarity >= 0.8:
            return f"The component is highly similar (score: {similarity:.2f}) to the request. Reuse as-is for efficiency."
        elif similarity >= 0.4:
            return f"The component is moderately similar (score: {similarity:.2f}) to the request. Evolve it to better match the requirements."
        return f"No sufficiently similar component found (best score: {similarity:.2f}). Create a new component."
