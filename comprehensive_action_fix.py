import re

# Read the ChatInput file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

print("Creating comprehensive fix for action pill population...")

# 1. Clean up the RichChatInput onChange handler
# Find the current messy onChange and replace with clean version
old_onchange_pattern = r'onChange=\{\(newValue, cursorPos\) => \{[^}]*\}\}'
new_onchange = '''onChange={(newValue, cursorPos) => {
              setDisplayValue(newValue);
              updateValue(newValue);
              debouncedSaveDraft(newValue);
              setHasUserTyped(true);
              
              // Check for @ mention and / action triggers
              if (cursorPos !== undefined) {
                const syntheticTarget = {
                  getBoundingClientRect: () => textAreaRef.current?.getBoundingClientRect?.() || new DOMRect(),
                  selectionStart: cursorPos,
                  selectionEnd: cursorPos,
                  value: newValue,
                };
                checkForMention(newValue, cursorPos, syntheticTarget as HTMLTextAreaElement);
              }
            }}'''

# Use a more targeted replacement
lines = content.split('\n')
start_idx = -1
end_idx = -1
brace_count = 0
in_onchange = False

for i, line in enumerate(lines):
    if 'onChange={(newValue, cursorPos) => {' in line:
        start_idx = i
        in_onchange = True
        brace_count = line.count('{') - line.count('}')
    elif in_onchange:
        brace_count += line.count('{') - line.count('}')
        if '}}' in line and brace_count <= 0:
            end_idx = i
            break

if start_idx != -1 and end_idx != -1:
    # Replace the onChange handler
    new_lines = lines[:start_idx] + ['            ' + new_onchange] + lines[end_idx + 1:]
    content = '\n'.join(new_lines)
    print(f"Fixed onChange handler (lines {start_idx}-{end_idx})")

# 2. Ensure the actionPopover state has cursorPosition
if 'cursorPosition?: number;' not in content:
    content = content.replace(
        'selectedIndex: number;\n  }>({',
        'selectedIndex: number;\n    cursorPosition?: number;\n  }>({'
    )
    content = content.replace(
        'selectedIndex: 0,\n  });',
        'selectedIndex: 0,\n    cursorPosition: 0,\n  });'
    )
    print("Added cursorPosition to actionPopover state")

# 3. Make sure checkForMention sets cursorPosition when opening action popover
content = content.replace(
    '''      setActionPopover({
        isOpen: true,
        position: {
          x: textAreaRect.left,
          y: textAreaRect.top,
        },
        selectedIndex: 0,
      });''',
    '''      setActionPopover({
        isOpen: true,
        position: {
          x: textAreaRect.left,
          y: textAreaRect.top,
        },
        selectedIndex: 0,
        cursorPosition: cursorPosition,
      });'''
)

print("Updated checkForMention to set cursorPosition")

# Write the modified content back
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)

print("Comprehensive fix applied successfully!")
print("\nKey changes made:")
print("1. Cleaned up RichChatInput onChange handler")
print("2. Ensured actionPopover state includes cursorPosition")
print("3. Updated checkForMention to pass cursorPosition to actionPopover")
print("4. handleActionSelect should now work correctly with proper cursor position")
