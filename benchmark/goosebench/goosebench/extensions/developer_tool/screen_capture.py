"""Test cases for the screen capture tool."""

# Prompts that should trigger valid screen capture tool usage
valid_prompts = [
    "Take a screenshot of the main display",
    "Capture the window titled 'Terminal'",
    "Screenshot the current display",
    "Take a picture of display 0",
    "Capture a screenshot of the browser window",
    "Take a screenshot of the active window",
    "Capture display 1",
    "Screenshot the window named 'Settings'",
    "Take a capture of the main screen",
    "Screenshot the specified window",
]

# Prompts that should not trigger screen capture tool usage based on tool description
invalid_prompts = [
    "Capture multiple windows at once",
    "Take a screenshot of all displays",
    "Record a video of the screen",
    "Capture a region of the screen",
    "Take a partial screenshot",
    "Screenshot a specific area",
    "Capture screen with mouse cursor",
    "Take a timed screenshot",
    "Screenshot with specific dimensions",
    "Capture screen without window decorations",
]