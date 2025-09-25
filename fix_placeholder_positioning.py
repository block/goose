import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing placeholder positioning and visibility...")

# Find the placeholder rendering logic and fix the positioning
old_placeholder_render = '''    // Show placeholder when there's no text content (regardless of focus state)
    if (!value.trim()) {
      return (
        <div className="relative">
          <span className="text-textSubtle pointer-events-none select-none absolute">
            {placeholder}
          </span>
          {isFocused && (
            <span className="border-l-2 border-black inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
          )}
        </div>
      );
    }'''

new_placeholder_render = '''    // Show placeholder when there's no text content (regardless of focus state)
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

content = content.replace(old_placeholder_render, new_placeholder_render)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed placeholder positioning:")
print("   - Removed absolute positioning that was causing cutoff")
print("   - Added min-h-[1.5em] to ensure proper container height")
print("   - Added flex items-center for proper vertical alignment")
print("   - Used ml-0.5 instead of marginLeft for cursor spacing")
print("   - Placeholder should now be visible and properly positioned")
