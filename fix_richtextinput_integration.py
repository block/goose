import re

# Read the ChatInput file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

# Find the RichChatInput onChange handler and update it to handle cursor position
old_onchange = '''            onChange={(newValue, cursorPos) => {
              setDisplayValue(newValue);
              updateValue(newValue);
              debouncedSaveDraft(newValue);
              setHasUserTyped(true);
              
              // Check for mention/action triggers with the new cursor position
              if (cursorPos !== undefined) {
                checkForMention(newValue, cursorPos, textAreaRef.current!);
              }
            }}'''

new_onchange = '''            onChange={(newValue, cursorPos) => {
              setDisplayValue(newValue);
              updateValue(newValue);
              debouncedSaveDraft(newValue);
              setHasUserTyped(true);
              
              // Check for mention/action triggers with the new cursor position
              if (cursorPos !== undefined) {
                checkForMention(newValue, cursorPos, {
                  getBoundingClientRect: () => textAreaRef.current?.getBoundingClientRect?.() || new DOMRect(),
                  selectionStart: cursorPos,
                  selectionEnd: cursorPos,
                  value: newValue,
                } as HTMLTextAreaElement);
              }
            }}'''

content = content.replace(old_onchange, new_onchange)

# Write the modified content back
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)

print("Fixed RichChatInput onChange integration with cursor position handling")
