import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing cursor vertical alignment with text...")

# Find all cursor instances and fix their styling
old_cursor_style = 'style={{ animation: "blink 1s step-end infinite", height: "1.2em", marginLeft: "1px" }}'
new_cursor_style = 'style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }}'

content = content.replace(old_cursor_style, new_cursor_style)

# Also need to ensure the cursor aligns with the text line height
# Let's also adjust the className to help with alignment
old_cursor_class = 'className="border-l-2 border-black inline-block"'
new_cursor_class = 'className="border-l-2 border-black inline-block align-baseline"'

content = content.replace(old_cursor_class, new_cursor_class)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed cursor vertical alignment:")
print("   - Changed height from 1.2em to 1em (matches text height)")
print("   - Added verticalAlign: 'baseline' to align with text baseline")
print("   - Added align-baseline class for consistent alignment")
print("   - Cursor should now be centered with the text line")
