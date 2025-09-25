import re

# Read the ChatInput file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

print("Updating ChatInput to convert @filename to pills when selected...")

# Find the handleMentionFileSelect function and update it to create pill format
old_mention_select = '''  const handleMentionFileSelect = (filePath: string) => {
    const beforeMention = displayValue.slice(0, mentionPopover.mentionStart);
    const afterMention = displayValue.slice(
      textAreaRef.current?.selectionStart || displayValue.length
    );
    const newValue = `${beforeMention}${filePath}${afterMention}`;

    setDisplayValue(newValue);
    setValue(newValue);
    setMentionPopover((prev) => ({ ...prev, isOpen: false }));
    textAreaRef.current?.focus();

    // Set cursor position after the inserted file path
    setTimeout(() => {
      if (textAreaRef.current) {
        // Cursor positioning handled by RichChatInput
        // Focus handled by RichChatInput
      }
    }, 0);
  };'''

new_mention_select = '''  const handleMentionFileSelect = (filePath: string) => {
    const beforeMention = displayValue.slice(0, mentionPopover.mentionStart);
    const afterMention = displayValue.slice(
      textAreaRef.current?.selectionStart || displayValue.length
    );
    
    // Extract just the filename from the path for the pill
    const fileName = filePath.split('/').pop() || filePath;
    const mentionText = `@${fileName}`;
    const newValue = `${beforeMention}${mentionText} ${afterMention}`;

    setDisplayValue(newValue);
    setValue(newValue);
    setMentionPopover((prev) => ({ ...prev, isOpen: false }));
    textAreaRef.current?.focus();

    // Set cursor position after the inserted mention and space
    const newCursorPosition = beforeMention.length + mentionText.length + 1;
    setTimeout(() => {
      if (textAreaRef.current) {
        textAreaRef.current.setSelectionRange(newCursorPosition, newCursorPosition);
        textAreaRef.current.focus();
      }
    }, 0);
  };'''

content = content.replace(old_mention_select, new_mention_select)

# Write the modified content back
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Updated ChatInput mention handling:")
print("   - Modified handleMentionFileSelect to create @filename format")
print("   - Extracts filename from full path for cleaner pills")
print("   - Adds space after mention for better UX")
print("   - Sets cursor position properly after insertion")
