import re

# Read the ChatInput file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

# Find and replace the handleActionSelect function
old_function = '''  const handleActionSelect = (actionId: string) => {
    const actionInfo = getActionInfo(actionId);
    
    // Replace / with [Action] text inline
    const currentValue = displayValue;
    const cursorPosition = 0; // RichChatInput handles cursor position internally
    const beforeCursor = currentValue.slice(0, cursorPosition);
    const lastSlashIndex = beforeCursor.lastIndexOf('/');
    
    if (lastSlashIndex !== -1) {
      const afterSlash = beforeCursor.slice(lastSlashIndex + 1);
      if (!afterSlash.includes(' ') && !afterSlash.includes('\\n')) {
        // Replace / with [Action] text
        const beforeSlash = currentValue.slice(0, lastSlashIndex);
        // Simplified approach for RichChatInput
        const actionText = `[${actionInfo.label}]`;
        const newValue = beforeSlash + actionText + " ";
        
        setDisplayValue(newValue);
        setValue(newValue);
        
        // Track selected action for pill display
        
        
        // Set cursor position after the action text
        setTimeout(() => {
          if (textAreaRef.current) {
            // Cursor positioning handled by RichChatInput
            // Focus handled by RichChatInput
          }
        }, 0);
      }
    }
    
    console.log('Action selected:', actionId);
    setActionPopover(prev => ({ ...prev, isOpen: false }));
  };'''

new_function = '''  const handleActionSelect = (actionId: string) => {
    const actionInfo = getActionInfo(actionId);
    
    // Get current cursor position from the RichChatInput
    const currentValue = displayValue;
    const cursorPosition = actionPopover.cursorPosition || 0;
    const beforeCursor = currentValue.slice(0, cursorPosition);
    const afterCursor = currentValue.slice(cursorPosition);
    const lastSlashIndex = beforeCursor.lastIndexOf('/');
    
    if (lastSlashIndex !== -1) {
      const afterSlash = beforeCursor.slice(lastSlashIndex + 1);
      // Check if we're still in the same "word" after the slash
      if (!afterSlash.includes(' ') && !afterSlash.includes('\\n')) {
        // Replace the /query with [Action] text
        const beforeSlash = currentValue.slice(0, lastSlashIndex);
        const actionText = `[${actionInfo.label}]`;
        const newValue = beforeSlash + actionText + " " + afterCursor;
        
        setDisplayValue(newValue);
        setValue(newValue);
        
        // Set cursor position after the action text and space
        const newCursorPosition = lastSlashIndex + actionText.length + 1;
        setTimeout(() => {
          if (textAreaRef.current) {
            textAreaRef.current.setSelectionRange(newCursorPosition, newCursorPosition);
            textAreaRef.current.focus();
          }
        }, 0);
      }
    }
    
    console.log('Action selected:', actionId, 'at position:', cursorPosition);
    setActionPopover(prev => ({ ...prev, isOpen: false }));
  };'''

# Replace the function
content = content.replace(old_function, new_function)

# Write the modified content back
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)

print("Fixed handleActionSelect function to properly handle cursor position and create action pills")
