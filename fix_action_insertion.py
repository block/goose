import re

# Read the ChatInput.tsx file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

# Find and replace the handleActionSelect function to handle both / trigger and button click
old_function = '''  const handleActionSelect = (actionId: string) => {
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
      if (!afterSlash.includes(' ') && !afterSlash.includes('\n')) {
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

new_function = '''  const handleActionSelect = (actionId: string) => {
    const actionInfo = getActionInfo(actionId);
    
    // Get current cursor position from the RichChatInput
    const currentValue = displayValue;
    const cursorPosition = actionPopover.cursorPosition || 0;
    const beforeCursor = currentValue.slice(0, cursorPosition);
    const afterCursor = currentValue.slice(cursorPosition);
    const lastSlashIndex = beforeCursor.lastIndexOf('/');
    
    // Check if this was triggered by a / command (slash exists and no space after it)
    if (lastSlashIndex !== -1) {
      const afterSlash = beforeCursor.slice(lastSlashIndex + 1);
      // Check if we're still in the same "word" after the slash
      if (!afterSlash.includes(' ') && !afterSlash.includes('\n')) {
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
        
        console.log('Action selected via slash command:', actionId, 'at position:', cursorPosition);
        setActionPopover(prev => ({ ...prev, isOpen: false }));
        return;
      }
    }
    
    // If not a slash command, insert action at current cursor position (button click)
    const actionText = `[${actionInfo.label}]`;
    const newValue = beforeCursor + actionText + " " + afterCursor;
    
    setDisplayValue(newValue);
    setValue(newValue);
    
    // Set cursor position after the action text and space
    const newCursorPosition = cursorPosition + actionText.length + 1;
    setTimeout(() => {
      if (textAreaRef.current) {
        textAreaRef.current.setSelectionRange(newCursorPosition, newCursorPosition);
        textAreaRef.current.focus();
      }
    }, 0);
    
    console.log('Action selected via button click:', actionId, 'at position:', cursorPosition);
    setActionPopover(prev => ({ ...prev, isOpen: false }));
  };'''

# Replace the function
content = content.replace(old_function, new_function)

# Write back to file
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)

print("Updated handleActionSelect function to handle both slash commands and button clicks")
