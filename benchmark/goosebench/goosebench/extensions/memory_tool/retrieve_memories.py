"""Test cases for the retrieve memories tool."""

# Prompts that should trigger valid retrieve memories tool usage
valid_prompts = [
    "Show all memories in the 'development' category",
    "Get my stored preferences from global memory",
    "Retrieve local project settings",
    "Show me what's stored in the 'workflow' category",
    "Get all global memories",
    "Retrieve everything from local storage",
    "Show memories tagged with #config",
    "Get all items from 'personal' category",
    "Retrieve project-specific memories",
    "Show what's saved in the 'build' category",
]

# Prompts that should not trigger retrieve memories tool usage based on tool description
invalid_prompts = [
    "Search across multiple categories at once",
    "Find memories without specifying scope",
    "Get memories with complex search criteria",
    "Retrieve memories by date range",
    "Search memories by content",
    "Get memories by partial category match",
    "Retrieve memories with regex patterns",
    "Find memories by size",
    "Get memories modified recently",
    "Search memories by author",
]