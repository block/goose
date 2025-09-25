import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing cursor positioning for multi-line text...")

# The issue is with the container layout. We need to change from flex to inline layout
# Find the return statement with the flex container
old_container = '''    return (
      <div className="flex flex-wrap items-center gap-1 min-h-[1.5em]">
        {parts.length > 0 ? parts : (
          isFocused && (
            <span className="border-l-2 border-black inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.2em", marginLeft: "1px" }} />
          )
        )}
      </div>
    );'''

new_container = '''    return (
      <div className="whitespace-pre-wrap min-h-[1.5em] leading-relaxed">
        {parts.length > 0 ? parts : (
          isFocused && (
            <span className="border-l-2 border-black inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.2em", marginLeft: "1px" }} />
          )
        )}
      </div>
    );'''

content = content.replace(old_container, new_container)

# Also need to fix how text parts are rendered to be truly inline
# Find the text rendering sections and make sure they're inline
old_text_span = 'className="whitespace-pre-wrap"'
new_text_span = 'className="inline whitespace-pre-wrap"'

content = content.replace(old_text_span, new_text_span)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed multi-line cursor positioning:")
print("   - Changed container from flex to whitespace-pre-wrap")
print("   - Added leading-relaxed for better line height")
print("   - Made text spans explicitly inline")
print("   - Cursor should now stay in correct position on line breaks")
