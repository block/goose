"""Test cases for the automation script tool."""

# Prompts that should trigger valid automation script tool usage
valid_prompts = [
    "Create a shell script to sort unique lines in a file",
    "Write a Ruby script to process some text data",
    "Make a script to extract the second column from a CSV",
    "Create a script to find pattern matches in a file",
    "Write a shell script to process log files",
    "Create a Ruby script for text manipulation",
    "Make a script to analyze data in a text file",
    "Write a script to format JSON data",
    "Create a script to clean up file names",
    "Write a script to extract specific data from files",
]

# Prompts that should not trigger automation script tool usage based on tool description
invalid_prompts = [
    "Create a complex application with multiple files",
    "Write a script that requires external dependencies",
    "Create a script that needs a database",
    "Write a GUI application",
    "Create a web server application",
    "Write a script that needs special system access",
    "Create a script that requires third-party libraries",
    "Write a script that needs network services",
    "Create a distributed processing script",
    "Write a script that requires system installation",
]