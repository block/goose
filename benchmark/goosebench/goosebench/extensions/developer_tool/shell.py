"""Test cases for the shell tool."""

# Prompts that should trigger valid shell tool usage
valid_prompts = [
    "Run the command 'ls' to list files",
    "Execute 'pwd' to show current directory",
    "Use ripgrep to search for files containing 'example'",
    "Find all Python files using 'rg --files | rg .py'",
    "Search for the string 'class Example' in files using ripgrep",
    "Show the contents of a file using cat",
    "Count lines in a file using wc -l",
    "Check disk space with df -h",
    "List processes with ps",
    "Create a directory with mkdir test",
]

# Prompts that should not trigger shell tool usage based on tool description
invalid_prompts = [
    "Run a command that will produce gigabytes of output",
    "Start a long-running server without backgrounding it",
    "Use find to recursively search for files",
    "Use ls -R to list all files recursively",
    "Execute a command that will run indefinitely",
    "Run a command that streams continuous output",
    "Use grep recursively to search files",
    "Start a process that needs to be manually terminated",
    "Run a command that generates unlimited output",
    "Execute ls -la on the entire filesystem",
]