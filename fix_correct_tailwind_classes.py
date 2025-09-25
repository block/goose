import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing Tailwind class names to match the CSS variables...")

# Looking at the CSS, the correct classes should be:
# - text-text-default (maps to --color-text-default)
# - text-text-muted (maps to --color-text-muted) 
# - border-text-default (maps to --color-text-default)

# Fix 1: Change placeholder text class
old_placeholder_class = 'className="text-textMuted pointer-events-none select-none"'
new_placeholder_class = 'className="text-text-muted pointer-events-none select-none"'

content = content.replace(old_placeholder_class, new_placeholder_class)

# Fix 2: Change cursor border class
old_cursor_border = 'border-textDefault'
new_cursor_border = 'border-text-default'

content = content.replace(old_cursor_border, new_cursor_border)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed Tailwind class names:")
print("   - Changed text-textMuted to text-text-muted")
print("   - Changed border-textDefault to border-text-default")
print("   - These should now map to the correct CSS variables")
print("   - Should work properly in both light and dark modes")
