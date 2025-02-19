"""Test cases for the Google Drive search tool."""

# Prompts that should trigger valid search tool usage
valid_prompts = [
    "Search for files named 'budget'",
    "Find documents containing 'report'",
    "Look for files with 'presentation' in the name",
    "Search my drive for 'meeting notes'",
    "Find files named 'project plan'",
    "Search for 'invoice' in my files",
    "Look up documents named 'proposal'",
    "Find spreadsheets with 'data' in the name",
    "Search for files containing 'schedule'",
    "Find documents with 'summary' in the title",
]

# Prompts that should not trigger search tool usage based on tool description
invalid_prompts = [
    "Search for files modified in the last week",
    "Find files larger than 1MB",
    "Search for files shared with me",
    "Look for files in a specific folder",
    "Find files by type",
    "Search for files by owner",
    "Look for recently modified files",
    "Find files with specific permissions",
    "Search for files by date",
    "Find files in trash",
]