"""Test cases for the web search tool."""

# Prompts that should trigger valid web search tool usage
valid_prompts = [
    "Search for information about 'Tesla'",
    "Look up what 'Bitcoin' is",
    "Find details about 'SpaceX'",
    "Search for 'Python' programming language",
    "What is 'Docker'?",
    "Look up the company 'Microsoft'",
    "Search for information about 'Linux'",
    "Find out about 'AWS'",
    "What is 'Kubernetes'?",
    "Search for 'React' framework",
]

# Prompts that should not trigger web search tool usage based on tool description
invalid_prompts = [
    "Search for multiple words at once",
    "Look up a complex query with multiple terms",
    "Search for a long phrase",
    "Find results for this entire sentence",
    "Search for 'word1 word2 word3'",
    "Look up multiple topics at once",
    "Search for a paragraph of text",
    "Find results for multiple questions",
    "Search for a list of items",
    "Look up several different topics",
]