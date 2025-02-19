"""Test cases for the load tutorial tool."""

# Prompts that should trigger valid load tutorial tool usage
valid_prompts = [
    "Show me the getting-started tutorial",
    "Load the developer-mcp tutorial",
    "I need help getting started, show the tutorial",
    "Can you load the tutorial about development?",
    "Show me how to use Goose with the tutorial",
    "Load the beginner's guide tutorial",
    "I'm new here, can you show me the introduction tutorial?",
    "Display the tutorial for developers",
    "Show the tutorial about MCP development",
    "Load the basic usage tutorial",
]

# Prompts that should not trigger load tutorial tool usage based on tool description
invalid_prompts = [
    "Create a new tutorial",
    "Edit the existing tutorial",
    "Delete this tutorial",
    "Modify tutorial content",
    "Save this as a tutorial",
    "Update the tutorial text",
    "Remove old tutorials",
    "Change tutorial format",
    "Add new tutorial section",
    "Merge multiple tutorials",
]