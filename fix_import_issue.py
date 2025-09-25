import re

# Read the ChatInput file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

# Fix the import - change from default import to named import
old_import = "import RichChatInput, { RichChatInputRef } from './RichChatInput';"
new_import = "import { RichChatInput, RichChatInputRef } from './RichChatInput';"

content = content.replace(old_import, new_import)

print("Fixed RichChatInput import to use named export")

# Write the modified content back
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)

# Also check if we need to fix the RichChatInput export
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    rich_content = f.read()

# Make sure it has both named and default exports
if 'export default RichChatInput;' not in rich_content:
    # Add default export if missing
    rich_content = rich_content.replace(
        'RichChatInput.displayName = \'RichChatInput\';',
        'RichChatInput.displayName = \'RichChatInput\';\n\nexport default RichChatInput;'
    )
    
    with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
        f.write(rich_content)
    
    print("Added default export to RichChatInput")
else:
    print("RichChatInput already has default export")

print("Import/export issue fixed!")
