"""Test cases for the list windows tool."""

# Prompts that should trigger valid list windows tool usage
valid_prompts = [
    "Show me all available windows",
    "List the windows that can be captured",
    "What windows are currently open?",
    "Display available window titles",
    "Get a list of windows for screen capture",
    "Show window titles that I can screenshot",
    "What windows can I take screenshots of?",
    "List all window titles",
    "Show me what windows are available for capture",
    "Get available window names",
]

# Prompts that should not trigger list windows tool usage based on tool description
invalid_prompts = [
    "Close all windows",
    "Minimize the current window",
    "Maximize the browser window",
    "Move window to another display",
    "Resize the current window",
    "Change window focus",
    "Arrange windows on screen",
    "Hide inactive windows",
    "Show desktop",
    "Switch between windows",
]