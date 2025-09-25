import re

# Read the ChatInput file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing handleMentionFileSelect to create @filename format...")

# Find the current handleMentionFileSelect function and replace it
old_function = '''  const handleMentionFileSelect = (filePath: string) => {
    // Replace the @ mention with the file path
    const beforeMention = displayValue.slice(0, mentionPopover.mentionStart);
    const afterMention = displayValue.slice(
      mentionPopover.mentionStart + 1 + mentionPopover.query.length
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

new_function = '''  const handleMentionFileSelect = (filePath: string) => {
    console.log('📁 handleMentionFileSelect called with:', filePath);
    
    // Extract just the filename from the full path for the pill
    const fileName = filePath.split('/').pop() || filePath;
    console.log('📁 Extracted filename:', fileName);
    
    // Create @filename format for pill detection
    const mentionText = `@${fileName}`;
    console.log('📁 Creating mention text:', mentionText);
    
    // Replace the @ mention with @filename format
    const beforeMention = displayValue.slice(0, mentionPopover.mentionStart);
    const afterMention = displayValue.slice(
      mentionPopover.mentionStart + 1 + mentionPopover.query.length
    );
    const newValue = `${beforeMention}${mentionText} ${afterMention}`;
    
    console.log('📁 New value will be:', newValue);

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

content = content.replace(old_function, new_function)

# Write the modified content back
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed handleMentionFileSelect function:")
print("   - Now extracts filename from full path")
print("   - Creates @filename format instead of full path")
print("   - Adds space after mention for better UX")
print("   - Added comprehensive debugging logs")
print("   - Should now create text that matches the mention regex")
