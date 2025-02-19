"""Test cases for the cache tool."""

# Prompts that should trigger valid cache tool usage
valid_prompts = [
    "List all cached files",
    "Show me what's in the cache",
    "View the content of this cached file",
    "Delete this specific cached file",
    "Clear all cached data",
    "Show the contents of a cached file",
    "Remove this file from cache",
    "List the cache directory contents",
    "View a cached text file",
    "Delete everything from the cache",
]

# Prompts that should not trigger cache tool usage based on tool description
invalid_prompts = [
    "Modify a cached file directly",
    "Search within cached files",
    "Compress the cache directory",
    "Move cached files to another location",
    "Change cache directory permissions",
    "Reorganize cached files",
    "Filter cache by file type",
    "Sort cached files by size",
    "Archive old cached files",
    "Backup the cache directory",
]