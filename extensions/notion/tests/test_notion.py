"""Tests for the Notion extension."""

import pytest
from unittest.mock import AsyncMock, MagicMock
from goose_notion import NotionExtension

@pytest.fixture
def notion_extension():
    """Create a Notion extension instance for testing."""
    return NotionExtension()

@pytest.fixture
async def initialized_extension(notion_extension):
    """Create an initialized Notion extension instance."""
    config = {"notion_token": "test-token"}
    await notion_extension.initialize(config)
    return notion_extension

@pytest.mark.asyncio
async def test_query_database(initialized_extension):
    """Test database querying functionality."""
    # Mock the Notion client response
    mock_response = {
        "results": [
            {"id": "test-id", "properties": {"Name": {"title": [{"text": {"content": "Test"}}]}}}
        ]
    }
    initialized_extension.client.databases.query = AsyncMock(return_value=mock_response)
    
    result = await initialized_extension.query_database(
        database_id="test-db",
        filter={"property": "Name", "text": {"equals": "Test"}}
    )
    
    assert result == mock_response
    initialized_extension.client.databases.query.assert_called_once()

@pytest.mark.asyncio
async def test_create_page(initialized_extension):
    """Test page creation functionality."""
    mock_response = {
        "id": "test-page-id",
        "properties": {"Name": {"title": [{"text": {"content": "Test Page"}}]}}
    }
    initialized_extension.client.pages.create = AsyncMock(return_value=mock_response)
    
    result = await initialized_extension.create_page(
        parent_id="test-db",
        properties={"Name": {"title": [{"text": {"content": "Test Page"}}]}}
    )
    
    assert result == mock_response
    initialized_extension.client.pages.create.assert_called_once()

@pytest.mark.asyncio
async def test_initialization(notion_extension):
    """Test extension initialization."""
    config = {"notion_token": "test-token"}
    await notion_extension.initialize(config)
    assert notion_extension.client is not None

@pytest.mark.asyncio
async def test_error_handling(initialized_extension):
    """Test error handling in extension functions."""
    initialized_extension.client.databases.query = AsyncMock(side_effect=Exception("API Error"))
    
    with pytest.raises(Exception) as exc_info:
        await initialized_extension.query_database("test-db")
    
    assert "Failed to query database" in str(exc_info.value)