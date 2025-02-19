"""Test cases for the Google Drive read tool."""

# Prompts that should trigger valid read tool usage
valid_prompts = [
    "Read the file with URI gdrive:///abc123",
    "Show me the contents of gdrive:///xyz789",
    "Get the text from gdrive:///doc456",
    "Read this Google Doc gdrive:///123abc",
    "Show the contents of spreadsheet gdrive:///789xyz",
    "Get the text of presentation gdrive:///456def",
    "Read file gdrive:///def123 and include images",
    "Show me gdrive:///789abc without images",
    "Get the content of document gdrive:///xyz456",
    "Read text file gdrive:///123xyz",
]

# Prompts that should not trigger read tool usage based on tool description
invalid_prompts = [
    "Edit the file gdrive:///abc123",
    "Write to document gdrive:///xyz789",
    "Modify spreadsheet gdrive:///123def",
    "Update presentation gdrive:///def789",
    "Delete file gdrive:///789xyz",
    "Create new document gdrive:///456abc",
    "Share file gdrive:///xyz123",
    "Move document gdrive:///789def",
    "Copy file gdrive:///abc789",
    "Rename document gdrive:///def456",
]