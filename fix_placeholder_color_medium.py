import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Changing placeholder to a medium grey color...")

# Find the placeholder text and change it to text-muted (neutral-400)
old_placeholder_color = 'className="text-textPlaceholder pointer-events-none select-none"'
new_placeholder_color = 'className="text-textMuted pointer-events-none select-none"'

content = content.replace(old_placeholder_color, new_placeholder_color)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed placeholder color:")
print("   - Changed from text-textPlaceholder (very light) to text-textMuted (medium grey)")
print("   - Uses --color-neutral-400 which should be more visible")
print("   - Should be a good balance between subtle and readable")
