import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing dark mode cursor color and text color...")

# Fix 1: Change cursor from black to theme-aware color
# Find all cursor instances and replace border-black with border-textDefault
old_cursor_class = 'className="border-l-2 border-black inline-block align-baseline absolute left-0"'
new_cursor_class = 'className="border-l-2 border-textDefault inline-block align-baseline absolute left-0"'

content = content.replace(old_cursor_class, new_cursor_class)

# Also fix other cursor instances in the text processing
content = content.replace('border-l-2 border-black inline-block', 'border-l-2 border-textDefault inline-block')

# Fix 2: Change placeholder text color to match text-textMuted properly
old_placeholder_color = 'className="text-textMuted pointer-events-none select-none"'
new_placeholder_color = 'className="text-textMuted pointer-events-none select-none"'

# The placeholder color is already correct, but let's make sure it's using the right class

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed dark mode styling:")
print("   - Changed cursor from border-black to border-textDefault")
print("   - Cursor will now be white in dark mode, black in light mode")
print("   - Placeholder text already uses text-textMuted which adapts to theme")
print("   - Both cursor and text should now work properly in dark mode")
