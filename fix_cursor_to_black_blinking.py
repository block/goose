import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Making cursor black and blinking like a regular cursor...")

# Fix 1: Change cursor color from grey to black and use proper blinking animation
old_cursor_1 = 'className="animate-pulse border-l border-textSubtle h-4 inline-block"'
new_cursor_1 = 'className="animate-blink border-l-2 border-black h-4 inline-block"'

content = content.replace(old_cursor_1, new_cursor_1)

# Fix 2: Also fix the cursor at the end of renderContent
old_cursor_2 = '<span className="animate-pulse border-l border-textSubtle h-4 inline-block" />'
new_cursor_2 = '<span className="animate-blink border-l-2 border-black h-4 inline-block" />'

content = content.replace(old_cursor_2, new_cursor_2)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Updated cursor styling:")
print("   - Changed from border-textSubtle (grey) to border-black")
print("   - Changed from animate-pulse to animate-blink")
print("   - Made border thicker (border-l-2) for better visibility")
print("   - Now behaves like a regular text cursor")

# Now we need to add the custom blink animation to the CSS
print("\n📝 Note: We need to add the animate-blink class to Tailwind config")
print("   This will make the cursor blink on/off like a regular text cursor")
