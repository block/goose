import re

# Read the ChatInput file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

# Fix 1: Update actionPopover state to include cursorPosition
old_state = '''  const [actionPopover, setActionPopover] = useState<{
    isOpen: boolean;
    position: { x: number; y: number };
    selectedIndex: number;
  }>({
    isOpen: false,
    position: { x: 0, y: 0 },
    selectedIndex: 0,
  });'''

new_state = '''  const [actionPopover, setActionPopover] = useState<{
    isOpen: boolean;
    position: { x: number; y: number };
    selectedIndex: number;
    cursorPosition?: number;
  }>({
    isOpen: false,
    position: { x: 0, y: 0 },
    selectedIndex: 0,
    cursorPosition: 0,
  });'''

content = content.replace(old_state, new_state)

# Fix 2: Update the checkForMention function to set cursorPosition when opening action popover
# Find the section where action popover is opened
old_action_open = '''      setActionPopover({
        isOpen: true,
        position: {
          x: textAreaRect.left,
          y: textAreaRect.top,
        },
        selectedIndex: 0,
      });'''

new_action_open = '''      setActionPopover({
        isOpen: true,
        position: {
          x: textAreaRect.left,
          y: textAreaRect.top,
        },
        selectedIndex: 0,
        cursorPosition: cursorPosition,
      });'''

content = content.replace(old_action_open, new_action_open)

# Fix 3: Update handleActionButtonClick to include cursorPosition
old_button_click = '''    setActionPopover({
      isOpen: true,
      position: {
        x: buttonRect.left,
        y: buttonRect.top,
      },
      selectedIndex: 0,
    });'''

new_button_click = '''    setActionPopover({
      isOpen: true,
      position: {
        x: buttonRect.left,
        y: buttonRect.top,
      },
      selectedIndex: 0,
      cursorPosition: textAreaRef.current?.getBoundingClientRect ? 0 : 0, // Will be set by RichChatInput
    });'''

content = content.replace(old_button_click, new_button_click)

# Write the modified content back
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)

print("Fixed actionPopover state to include cursorPosition")
