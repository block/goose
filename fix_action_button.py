import re

# Read the ChatInput.tsx file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

# Find and replace the handleActionButtonClick function
old_function = '''  const handleActionButtonClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    const buttonRect = event.currentTarget.getBoundingClientRect();
    
    setActionPopover({
      isOpen: true,
      position: {
        x: buttonRect.left,
        y: buttonRect.top,
      },
      selectedIndex: 0,
      cursorPosition: textAreaRef.current?.getBoundingClientRect ? 0 : 0, // Will be set by RichChatInput
    });
  };'''

new_function = '''  const handleActionButtonClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    const buttonRect = event.currentTarget.getBoundingClientRect();
    
    // Get the current cursor position from the RichChatInput
    const currentCursorPosition = textAreaRef.current?.getBoundingClientRect ? displayValue.length : 0;
    
    setActionPopover({
      isOpen: true,
      position: {
        x: buttonRect.left,
        y: buttonRect.top,
      },
      selectedIndex: 0,
      cursorPosition: currentCursorPosition,
    });
  };'''

# Replace the function
content = content.replace(old_function, new_function)

# Write back to file
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)

print("Updated handleActionButtonClick function to get cursor position")
