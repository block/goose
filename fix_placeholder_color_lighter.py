import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Changing placeholder color back to lighter original color...")

# Find the placeholder text and change it back to the original lighter color
old_placeholder_color = 'className="text-textSubtle pointer-events-none select-none"'
new_placeholder_color = 'className="text-textPlaceholder pointer-events-none select-none"'

content = content.replace(old_placeholder_color, new_placeholder_color)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed placeholder color:")
print("   - Changed from text-textSubtle (darker grey) back to text-textPlaceholder (lighter grey)")
print("   - Now uses the same light color as the original empty text")
print("   - Should be more subtle and less prominent")
