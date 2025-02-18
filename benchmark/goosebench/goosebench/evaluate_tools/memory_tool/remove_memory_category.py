"""Test cases for the remove memory category tool."""

# Prompts that should trigger valid remove memory category tool usage
valid_prompts = [
    "Delete all memories in the 'development' category",
    "Clear the 'workflow' category from global storage",
    "Remove all local project settings",
    "Delete everything in the 'personal' category",
    "Clear all global memories",
    "Remove all local memories",
    "Delete the 'build' category",
    "Clear project configuration category",
    "Remove the 'git' category memories",
    "Delete all items in 'credentials' category",
]

# Prompts that should not trigger remove memory category tool usage based on tool description
invalid_prompts = [
    "Delete memories across multiple categories",
    "Remove memories without specifying scope",
    "Clear memories by date range",
    "Delete memories by content",
    "Remove memories with specific tags",
    "Clear memories by partial category match",
    "Delete memories selectively",
    "Remove memories by size",
    "Clear recently modified memories",
    "Delete memories by author",
]