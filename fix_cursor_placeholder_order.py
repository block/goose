import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing cursor position to appear before placeholder text...")

# Find the placeholder rendering and fix the cursor positioning
old_placeholder_render = '''    // Show placeholder when there's no text content (regardless of focus state)
    if (!value.trim()) {
      return (
        <div className="relative min-h-[1.5em] flex items-center">
          <span className="text-textSubtle pointer-events-none select-none">
            {placeholder}
          </span>
          {isFocused && (
            <span className="border-l-2 border-black inline-block align-baseline ml-0.5" style={{ animation: "blink 1s step-end infinite", height: "1em", verticalAlign: "baseline" }} />
          )}
        </div>
      );
    }'''

new_placeholder_render = '''    // Show placeholder when there's no text content (regardless of focus state)
    if (!value.trim()) {
      return (
        <div className="relative min-h-[1.5em] flex items-center">
          {isFocused && (
            <span className="border-l-2 border-black inline-block align-baseline mr-0.5" style={{ animation: "blink 1s step-end infinite", height: "1em", verticalAlign: "baseline" }} />
          )}
          <span className="text-textSubtle pointer-events-none select-none">
            {placeholder}
          </span>
        </div>
      );
    }'''

content = content.replace(old_placeholder_render, new_placeholder_render)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed cursor and placeholder order:")
print("   - Moved cursor before placeholder text in the DOM")
print("   - Changed ml-0.5 to mr-0.5 (margin-right instead of margin-left)")
print("   - Cursor now appears at the beginning of the line")
print("   - Placeholder text appears after the cursor")
