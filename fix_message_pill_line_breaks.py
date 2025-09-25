import re

# Read the MessageContent file
with open('ui/desktop/src/components/MessageContent.tsx', 'r') as f:
    content = f.read()

print("Fixing document pill line breaks in message display...")

# The issue is the same as before - using flex layout causes pills to break lines
# Change from flex to inline layout like we did for the input

old_container = '''  return (
    <span className={`inline-flex flex-wrap items-baseline gap-1 ${className || ''}`}>'''

new_container = '''  return (
    <span className={`inline ${className || ''}`}>'''

content = content.replace(old_container, new_container)

# Write the modified content back
with open('ui/desktop/src/components/MessageContent.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed message pill line breaks:")
print("   - Changed from inline-flex flex-wrap to inline")
print("   - Removed flex layout that was causing line breaks")
print("   - Pills should now flow inline with text in messages")
print("   - Same fix as applied to input field earlier")
