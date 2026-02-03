from fastapi import APIRouter
from pydantic import BaseModel

from src.agents import get_agent
from src.agents.base import AgentResult

router = APIRouter()


class RuleEvaluationRequest(BaseModel):
    rule_text: str
    event_data: dict | None = None  # Advanced: pass extra event data for edge cases.


# ... existing code ...
@router.post("/rules/evaluate", response_model=AgentResult)
async def evaluate_rule(request: RuleEvaluationRequest) -> AgentResult:
    # Agent: uses central config—change here affects all rule evals.
    agent = get_agent("feasibility")

    # Async call—agent may throw if rule malformed.
    result = await agent.execute(rule_description=request.rule_text)

    # Re-wrap the result to match the expected AgentResult structure for the response
    return AgentResult(
        success=result.data.get("is_feasible", False),
        message=result.message,
        data={
            "supported": result.data.get("is_feasible", False),
            "rule_yaml": result.data.get("yaml_content", ""),
            "snippet": result.data.get("yaml_content", ""),
        },
    )
