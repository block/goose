"""Test cases for the JetBrains IDE integration tools."""

# Prompts that should trigger valid JetBrains tool usage
valid_prompts = [
    "Open the current file in the IDE",
    "Navigate to line 42 in the active file",
    "Find usages of this class",
    "Go to the definition of this method",
    "Show documentation for this symbol",
    "Run the current test file",
    "Debug this application",
    "Show project structure",
    "Open recent files",
    "Search everywhere in the project",
]

# Prompts that should not trigger JetBrains tool usage based on tool description
invalid_prompts = [
    "Create a new IDE instance",
    "Modify IDE settings",
    "Install new plugins",
    "Change IDE theme",
    "Update the IDE version",
    "Configure version control",
    "Modify IDE keymap",
    "Change project settings",
    "Install new IDE features",
    "Uninstall IDE components",
]