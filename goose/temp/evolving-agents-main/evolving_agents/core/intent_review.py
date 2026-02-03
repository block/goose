# evolving_agents/core/intent_review.py
from enum import Enum
from typing import Dict, List, Optional, Any, Union
from dataclasses import dataclass, field
from datetime import datetime, timezone # Keep timezone for consistency

class IntentStatus(str, Enum):
    """Status of an intent in a plan."""
    PENDING = "PENDING"
    APPROVED = "APPROVED"
    REJECTED = "REJECTED"
    EXECUTING = "EXECUTING"
    COMPLETE = "COMPLETE"
    FAILED = "FAILED"

@dataclass
class Intent:
    """Represents a single intent in the plan."""
    intent_id: str
    step_type: str  # DEFINE, CREATE, EXECUTE, or RETURN
    component_type: str  # AGENT, TOOL, etc.
    component_name: str
    action: str
    params: Dict[str, Any]
    justification: str
    depends_on: List[str] = field(default_factory=list)
    status: IntentStatus = IntentStatus.PENDING
    result: Optional[Dict[str, Any]] = None
    errors: Optional[List[str]] = None
    review_comments: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert intent to dictionary."""
        return {
            "intent_id": self.intent_id,
            "step_type": self.step_type,
            "component_type": self.component_type,
            "component_name": self.component_name,
            "action": self.action,
            "params": self.params,
            "justification": self.justification,
            "depends_on": self.depends_on,
            "status": self.status.value if isinstance(self.status, Enum) else self.status, # Ensure enum is converted
            "result": self.result,
            "errors": self.errors,
            "review_comments": self.review_comments
        }
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'Intent':
        """Create intent from dictionary."""
        return cls(
            intent_id=data["intent_id"],
            step_type=data["step_type"],
            component_type=data.get("component_type", "UNKNOWN"),
            component_name=data["component_name"],
            action=data["action"],
            params=data["params"],
            justification=data.get("justification", ""),
            depends_on=data.get("depends_on", []),
            status=IntentStatus(data.get("status", "PENDING")),
            result=data.get("result"),
            errors=data.get("errors"),
            review_comments=data.get("review_comments")
        )

class PlanStatus(str, Enum):
    """Status of an intent plan."""
    PENDING_REVIEW = "PENDING_REVIEW"
    APPROVED = "APPROVED"
    REJECTED = "REJECTED"
    EXECUTING = "EXECUTING"
    COMPLETE = "COMPLETE"
    FAILED = "FAILED"

@dataclass
class IntentPlan:
    """Represents a plan of intents to be executed."""
    plan_id: str
    title: str
    description: str
    objective: str
    intents: List[Intent]
    status: PlanStatus = PlanStatus.PENDING_REVIEW
    review_timestamp: Optional[str] = None
    reviewer_comments: Optional[str] = None
    rejection_reason: Optional[str] = None
    created_at: Optional[str] = field(default_factory=lambda: datetime.now(timezone.utc).isoformat()) # Added field

    def to_dict(self) -> Dict[str, Any]:
        """Convert plan to dictionary."""
        return {
            "plan_id": self.plan_id,
            "title": self.title,
            "description": self.description,
            "objective": self.objective,
            "intents": [intent.to_dict() for intent in self.intents],
            "status": self.status.value if isinstance(self.status, Enum) else self.status, # Ensure enum is converted
            "review_timestamp": self.review_timestamp,
            "reviewer_comments": self.reviewer_comments,
            "rejection_reason": self.rejection_reason,
            "created_at": self.created_at
        }
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'IntentPlan':
        """Create plan from dictionary."""
        return cls(
            plan_id=data["plan_id"],
            title=data["title"],
            description=data["description"],
            objective=data["objective"],
            intents=[Intent.from_dict(intent_data) for intent_data in data.get("intents", [])], # Handle missing intents gracefully
            status=PlanStatus(data.get("status", "PENDING_REVIEW")),
            review_timestamp=data.get("review_timestamp"),
            reviewer_comments=data.get("reviewer_comments"),
            rejection_reason=data.get("rejection_reason"),
            created_at=data.get("created_at") # Allow created_at to be loaded
        )