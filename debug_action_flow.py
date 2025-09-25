import re

# Read the ChatInput file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

# Add debugging to handleActionSelect function
old_function_pattern = r'  const handleActionSelect = \(actionId: string\) => \{[^}]*console\.log\([^)]*\);\s*setActionPopover[^}]*\};'

new_function = '''  const handleActionSelect = (actionId: string) => {
    console.log('🎯 handleActionSelect called with:', actionId);
    const actionInfo = getActionInfo(actionId);
    console.log('📋 Action info:', actionInfo);
    
    // Get current cursor position from the actionPopover state
    const currentValue = displayValue;
    const cursorPosition = actionPopover.cursorPosition || 0;
    console.log('📍 Current state:', { currentValue, cursorPosition, displayValue });
    
    const beforeCursor = currentValue.slice(0, cursorPosition);
    const afterCursor = currentValue.slice(cursorPosition);
    const lastSlashIndex = beforeCursor.lastIndexOf('/');
    
    console.log('🔍 Cursor analysis:', { beforeCursor, afterCursor, lastSlashIndex });
    
    if (lastSlashIndex !== -1) {
      const afterSlash = beforeCursor.slice(lastSlashIndex + 1);
      console.log('⚡ After slash:', afterSlash);
      
      if (!afterSlash.includes(' ') && !afterSlash.includes('\\n')) {
        // Replace the /query with [Action] text
        const beforeSlash = currentValue.slice(0, lastSlashIndex);
        const actionText = `[${actionInfo.label}]`;
        const newValue = beforeSlash + actionText + " " + afterCursor;
        
        console.log('🔄 Text replacement:', { 
          beforeSlash, 
          actionText, 
          afterCursor, 
          newValue,
          oldValue: currentValue 
        });
        
        setDisplayValue(newValue);
        setValue(newValue);
        
        console.log('✅ Values set, new text should be:', newValue);
        
        // Set cursor position after the action text and space
        const newCursorPosition = lastSlashIndex + actionText.length + 1;
        setTimeout(() => {
          if (textAreaRef.current) {
            textAreaRef.current.setSelectionRange(newCursorPosition, newCursorPosition);
            textAreaRef.current.focus();
          }
        }, 0);
      } else {
        console.log('❌ After slash contains space/newline, not replacing');
      }
    } else {
      console.log('❌ No slash found in text before cursor');
    }
    
    console.log('🎯 Action selected:', actionId, 'at position:', cursorPosition);
    setActionPopover(prev => ({ ...prev, isOpen: false }));
  };'''

# Find and replace the function using regex
content = re.sub(old_function_pattern, new_function, content, flags=re.DOTALL)

# Write the modified content back
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)

print("Added comprehensive debugging to handleActionSelect")
