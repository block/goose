"""Test cases for the text editor tool."""

# Prompts that should trigger valid text editor tool usage
valid_prompts = [
    "View the contents of file.txt",
    "Show me what's in config.py",
    "Create a new file called test.txt with 'Hello World' content",
    "Write 'print(\"hello\")' to script.py",
    "Replace the string 'old_version' with 'new_version' in config.txt",
    "Change 'debug=True' to 'debug=False' in settings.py",
    "Undo the last edit made to main.py",
    "Revert the previous change in config.json",
    "Write this JSON content to data.json",
    "Update the version number in package.json",
]

# Prompts that should not trigger text editor tool usage based on tool description
invalid_prompts = [
    "Edit multiple sections of the file at once",
    "Replace all occurrences of a string in the file",
    "Make changes to multiple files simultaneously",
    "Modify a file that's larger than 400KB",
    "Edit a file with more than 400,000 characters",
    "Replace a string that appears multiple times in the file",
    "Make partial updates to specific sections without full file content",
    "Edit binary files",
    "Modify files without providing full path",
    "Replace text without exact string match",
]