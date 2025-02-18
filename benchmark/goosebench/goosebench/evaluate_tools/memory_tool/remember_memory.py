"""Test cases for the remember memory tool."""

# Prompts that should trigger valid remember memory tool usage
valid_prompts = [
    "Remember this development preference in the 'development' category",
    "Store this setting globally with tags #config #setup",
    "Save this workflow detail locally in 'workflow' category",
    "Remember my name and email in the 'personal' category globally",
    "Store project configuration locally with #settings tag",
    "Save this formatting preference in development category",
    "Remember this shortcut in 'keyboard' category with #shortcuts tag",
    "Store build instructions locally in 'build' category",
    "Save API credentials globally in 'credentials' category",
    "Remember git configuration in 'git' category with #config tag",
]

# Prompts that should not trigger remember memory tool usage based on tool description
invalid_prompts = [
    "Save this without specifying a category",
    "Store this without indicating global or local scope",
    "Remember this with invalid tags format",
    "Save empty content in a category",
    "Store this in multiple categories at once",
    "Remember this with system-level access",
    "Save this in a protected category",
    "Store this with special file permissions",
    "Remember this in a non-existent directory",
    "Save this with binary content",
]