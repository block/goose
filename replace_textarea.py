import re

# Read the ChatInput file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

# Find and replace the textarea element with RichTextInput
textarea_start = content.find('<textarea')
if textarea_start == -1:
    print("Textarea not found")
    exit(1)

# Find the end of the textarea element
textarea_end = content.find('</textarea>', textarea_start) + len('</textarea>')
if textarea_end == -1:
    print("Textarea end not found")
    exit(1)

# Extract the old textarea
old_textarea = content[textarea_start:textarea_end]
print(f"Found textarea from {textarea_start} to {textarea_end}")

# Create the new RichTextInput component
new_richtextinput = '''<RichTextInput
            ref={textAreaRef}
            value={displayValue}
            onChange={(newValue, newActions) => {
              setDisplayValue(newValue);
              setValue(newValue);
              setActions(newActions);
              debouncedSaveDraft(newValue);
              setHasUserTyped(true);
              checkForMention(newValue, textAreaRef.current?.selectionStart || 0, textAreaRef.current!);
            }}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            onFocus={() => setIsFocused(true)}
            onBlur={() => setIsFocused(false)}
            onCompositionStart={handleCompositionStart}
            onCompositionEnd={handleCompositionEnd}
            placeholder={isRecording ? '' : '⌘↑/⌘↓ to navigate messages'}
            disabled={isUserInputDisabled}
            style={{
              maxHeight: `${maxHeight}px`,
              overflowY: 'auto',
              opacity: isRecording ? 0 : 1,
            }}
            className="w-full outline-none border-none focus:ring-0 bg-transparent px-3 pt-3 pb-1.5 pr-20 text-sm resize-none text-textStandard placeholder:text-textPlaceholder"
            actions={actions}
            maxHeight={maxHeight}
          />'''

# Replace the textarea with RichTextInput
content = content[:textarea_start] + new_richtextinput + content[textarea_end:]

# Write the modified content back
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)

print("Textarea replaced with RichTextInput successfully")
