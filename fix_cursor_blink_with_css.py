import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing cursor blink by using inline styles instead of Tailwind class...")

# Replace animate-blink with inline style
old_cursor_1 = 'className="animate-blink border-l-2 border-black h-4 inline-block"'
new_cursor_1 = 'className="border-l-2 border-black h-4 inline-block" style={{ animation: "blink 1s step-end infinite" }}'

content = content.replace(old_cursor_1, new_cursor_1)

# Also fix the other cursor instance
old_cursor_2 = '<span className="animate-blink border-l-2 border-black h-4 inline-block" />'
new_cursor_2 = '<span className="border-l-2 border-black h-4 inline-block" style={{ animation: "blink 1s step-end infinite" }} />'

content = content.replace(old_cursor_2, new_cursor_2)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed cursor blinking by using inline CSS animation")
print("   - Removed animate-blink class (not recognized by Tailwind)")
print("   - Added inline style: animation: 'blink 1s step-end infinite'")
print("   - Uses the @keyframes blink defined in main.css")
print("   - Should now blink properly!")
