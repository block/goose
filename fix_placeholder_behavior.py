import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing placeholder behavior to stay visible when focused...")

# Find the renderContent function and fix the placeholder logic
old_placeholder_logic = '''  const renderContent = useCallback(() => {
    if (!value.trim() && !isFocused) {
      return (
        <span className="text-textPlaceholder pointer-events-none select-none">
          {placeholder}
        </span>
      );
    }'''

new_placeholder_logic = '''  const renderContent = useCallback(() => {
    // Show placeholder when there's no text content (regardless of focus state)
    if (!value.trim()) {
      return (
        <div className="relative">
          <span className="text-textSubtle pointer-events-none select-none absolute">
            {placeholder}
          </span>
          {isFocused && (
            <span className="border-l-2 border-black inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.2em", marginLeft: "1px" }} />
          )}
        </div>
      );
    }'''

content = content.replace(old_placeholder_logic, new_placeholder_logic)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed placeholder behavior:")
print("   - Changed from text-textPlaceholder to text-textSubtle (matches icon color)")
print("   - Placeholder now stays visible when input is focused")
print("   - Only disappears when user actually types text")
print("   - Cursor appears alongside placeholder when focused")
print("   - Uses absolute positioning to overlay cursor on placeholder")
