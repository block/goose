import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing cursor positioning to align with text...")

# Fix 1: Update cursor styling to match text height and alignment
old_cursor_1 = 'className="border-l-2 border-black h-4 inline-block" style={{ animation: "blink 1s step-end infinite" }}'
new_cursor_1 = 'className="border-l-2 border-black inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.2em", marginLeft: "1px" }}'

content = content.replace(old_cursor_1, new_cursor_1)

# Fix 2: Also fix the other cursor instance
old_cursor_2 = '<span className="border-l-2 border-black h-4 inline-block" style={{ animation: "blink 1s step-end infinite" }} />'
new_cursor_2 = '<span className="border-l-2 border-black inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.2em", marginLeft: "1px" }} />'

content = content.replace(old_cursor_2, new_cursor_2)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed cursor positioning:")
print("   - Removed fixed h-4 height")
print("   - Added height: '1.2em' to match text line height")
print("   - Added marginLeft: '1px' for better spacing from text")
print("   - Cursor should now align properly with text")
