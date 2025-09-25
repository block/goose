import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing placeholder text shift when cursor appears...")

# Find the placeholder rendering and fix it to prevent text shifting
old_placeholder_render = '''    // Show placeholder when there's no text content (regardless of focus state)
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

new_placeholder_render = '''    // Show placeholder when there's no text content (regardless of focus state)
    if (!value.trim()) {
      return (
        <div className="relative min-h-[1.5em] flex items-center">
          <span className="text-textSubtle pointer-events-none select-none">
            {placeholder}
          </span>
          {isFocused && (
            <span 
              className="border-l-2 border-black inline-block align-baseline absolute left-0" 
              style={{ 
                animation: "blink 1s step-end infinite", 
                height: "1em", 
                verticalAlign: "baseline",
                top: "50%",
                transform: "translateY(-50%)"
              }} 
            />
          )}
        </div>
      );
    }'''

content = content.replace(old_placeholder_render, new_placeholder_render)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed placeholder text shifting:")
print("   - Made cursor absolutely positioned so it doesn't affect layout")
print("   - Added left-0 to position cursor at the start")
print("   - Added top: 50% and transform: translateY(-50%) for vertical centering")
print("   - Placeholder text now stays in exact same position when cursor appears")
