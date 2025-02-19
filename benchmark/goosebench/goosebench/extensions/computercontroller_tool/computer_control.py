"""Test cases for the computer control tool."""

# Prompts that should trigger valid computer control tool usage
valid_prompts = [
    "Launch Safari and open a specific URL",
    "Use AppleScript to automate Mail app",
    "Click a button in the current application",
    "Fill out a form in Safari",
    "Control system volume using AppleScript",
    "Organize files in a folder",
    "Add an event to Calendar",
    "Send an email using Mail app",
    "Manage iTunes playlist",
    "Automate document processing in Pages",
]

# Prompts that should not trigger computer control tool usage based on tool description
invalid_prompts = [
    "Control applications that don't support AppleScript",
    "Perform actions requiring root access",
    "Modify system files directly",
    "Access restricted system areas",
    "Control non-Apple applications without AppleScript support",
    "Perform actions requiring kernel modifications",
    "Execute privileged system commands",
    "Modify protected system settings",
    "Access hardware directly",
    "Control low-level system functions",
]