"""Notion integration for Goose."""

from typing import Any, Dict, List, Optional
from datetime import datetime, timedelta
from goose.extension import Extension, extension_function

class NotionExtension(Extension):
    """Extension providing Notion API integration for Goose."""
    
    name = "notion"
    version = "0.1.0"
    
    def __init__(self) -> None:
        super().__init__()
        self.client = None
        
    async def initialize(self, config: Dict[str, Any]) -> None:
        """Initialize the Notion client."""
        from notion_client import Client
        self.client = Client(auth=config["notion_token"])
    
    @extension_function(
        description="Query a Notion database with filters and sorting",
        parameters={
            "database_id": {
                "type": "string",
                "description": "The ID of the database to query"
            },
            "filter": {
                "type": "object",
                "description": "Filter criteria for the query",
                "required": False
            },
            "sorts": {
                "type": "array",
                "description": "Sort criteria for the query",
                "required": False
            }
        }
    )
    async def query_database(self, database_id: str, 
                           filter: Optional[Dict] = None,
                           sorts: Optional[List] = None) -> Dict[str, Any]:
        """Query a Notion database with the given parameters."""
        try:
            return await self.client.databases.query(
                database_id=database_id,
                filter=filter,
                sorts=sorts
            )
        except Exception as e:
            raise Exception(f"Failed to query database: {str(e)}")
    
    @extension_function(
        description="Create a new page in Notion",
        parameters={
            "parent_id": {
                "type": "string",
                "description": "ID of the parent database or page"
            },
            "properties": {
                "type": "object",
                "description": "Page properties to set"
            },
            "children": {
                "type": "array",
                "description": "Page content blocks",
                "required": False
            }
        }
    )
    async def create_page(self, parent_id: str,
                         properties: Dict[str, Any],
                         children: Optional[List] = None) -> Dict[str, Any]:
        """Create a new page in Notion."""
        try:
            page_data = {
                "parent": {"database_id": parent_id},
                "properties": properties
            }
            if children:
                page_data["children"] = children
            
            return await self.client.pages.create(**page_data)
        except Exception as e:
            raise Exception(f"Failed to create page: {str(e)}")

# Additional functions would follow the same pattern