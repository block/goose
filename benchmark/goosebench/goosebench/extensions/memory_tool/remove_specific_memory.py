"""Test cases for the remove specific memory tool."""

# Prompts that should trigger valid remove specific memory tool usage
valid_prompts = [
    "Delete the memory about code formatting from development category",
    "Remove the git configuration memory from global storage",
    "Delete project API key from credentials category",
    "Remove my email setting from personal category",
    "Delete the build instruction memory from local storage",
    "Remove specific workflow step from workflow category",
    "Delete keyboard shortcut memory from shortcuts category",
    "Remove specific project setting from local config",
    "Delete specific credential from global storage",
    "Remove particular preference from settings category",
]

# Prompts that should not trigger remove specific memory tool usage based on tool description
invalid_prompts = [
    "Delete multiple memories at once",
    "Remove memories by pattern matching",
    "Delete memories without exact content",
    "Remove memories by tag only",
    "Delete memories by date",
    "Remove partial memory content",
    "Delete memories by regex",
    "Remove memories without category",
    "Delete memories by approximate match",
    "Remove memories without scope",
]