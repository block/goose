import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing cursor and placeholder styling...")

# Fix 1: Change cursor color from white to grey
old_cursor_1 = 'className="animate-pulse border-l border-textStandard h-4 inline-block"'
new_cursor_1 = 'className="animate-pulse border-l border-textSubtle h-4 inline-block"'

content = content.replace(old_cursor_1, new_cursor_1)

# Fix 2: Also fix the cursor at the end of renderContent
old_cursor_2 = '<span className="animate-pulse border-l border-textStandard h-4 inline-block" />'
new_cursor_2 = '<span className="animate-pulse border-l border-textSubtle h-4 inline-block" />'

content = content.replace(old_cursor_2, new_cursor_2)

# Fix 3: Ensure placeholder text is using textPlaceholder (should already be correct)
# Check if it's already using the right class
if 'text-textPlaceholder' in content:
    print("✅ Placeholder text already using textPlaceholder class")
else:
    print("⚠️  Placeholder text class might need fixing")

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed cursor styling:")
print("   - Changed from border-textStandard (white) to border-textSubtle (grey)")
print("✅ Placeholder text should already be subtle (text-textPlaceholder)")
