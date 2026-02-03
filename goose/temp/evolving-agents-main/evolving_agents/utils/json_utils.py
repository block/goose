# evolving_agents/utils/json_utils.py
import json
import logging
from datetime import datetime, timezone
from enum import Enum # For handling enums like IntentStatus, PlanStatus

logger = logging.getLogger(__name__)

def safe_json_dumps(data: any, indent: int = 2) -> str:
    """Safely dump data to JSON, handling common non-serializable types."""
    def default_serializer(obj):
        if isinstance(obj, (datetime, timezone)):
            return obj.isoformat()
        if isinstance(obj, set):
            return list(obj)
        if isinstance(obj, Enum): # General Enum handling
            return obj.value
        if hasattr(obj, 'to_dict') and callable(obj.to_dict): # For Pydantic models or dataclasses with to_dict
            return obj.to_dict()
        
        # Fallback: Convert to string if no specific handler matched
        # This should be used cautiously as it can hide serialization issues.
        logger.warning(f"Object of type {obj.__class__.__name__} is not directly JSON serializable or handled by custom rules, using str(). Value: {str(obj)[:100]}")
        return str(obj)

    try:
        return json.dumps(data, indent=indent, default=default_serializer)
    except TypeError as e:
        logger.error(f"JSON serialization error even after custom default_serializer: {e}", exc_info=True)
        # Fallback to a basic error message within JSON
        return json.dumps({"error": f"Data not fully serializable: {str(e)}", "original_type": str(type(data))})