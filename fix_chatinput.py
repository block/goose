import re

# Read the ChatInput file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

# 1. Add RichTextInput import after ActionPill import
content = re.sub(
    r'(import ActionPill from \'\.\/ActionPill\';)',
    r'\1\nimport RichTextInput, { ActionData } from \'./RichTextInput\';',
    content
)

# 2. Replace selectedActions state with actions state - find the exact pattern
selectedactions_pattern = r'  // Action pills for visual display\s*const \[selectedActions, setSelectedActions\] = useState<Array<\{\s*id: string;\s*actionId: string;\s*label: string;\s*icon: React\.ReactNode;\s*\}>>.*?\[\]\);'
content = re.sub(
    selectedactions_pattern,
    '  // Action pills for visual display\n  const [actions, setActions] = useState<ActionData[]>([]);',
    content,
    flags=re.DOTALL
)

# 3. Remove the Selected Actions Pills section completely
selected_pills_pattern = r'\s*\{\/\* Selected Actions Pills \*\/\}\s*\{selectedActions\.length > 0 && \(\s*<div className="flex flex-wrap items-center gap-2 px-3 pt-2 pb-1 bg-bgSubtle\/20 border-b border-borderSubtle\/50">\s*<span className="text-xs text-textSubtle font-medium">Selected:<\/span>\s*\{selectedActions\.map\(\(action\) => \(\s*<ActionPill\s*key=\{action\.id\}\s*actionId=\{action\.actionId\}\s*label=\{action\.label\}\s*icon=\{action\.icon\}\s*onRemove=\{\(\) => handleRemoveAction\(action\.id\)\}\s*\/>\s*\)\)\}\s*<\/div>\s*\)\}'
content = re.sub(selected_pills_pattern, '', content, flags=re.DOTALL)

# 4. Replace all selectedActions with actions and setSelectedActions with setActions
content = content.replace('selectedActions', 'actions')
content = content.replace('setActions([]);', 'setActions([]);')  # Fix double replacement

# 5. Update handleActionSelect - find and replace the entire function
handle_action_select_old = r'  const handleActionSelect = \(actionId: string\) => \{[^}]*\};'
handle_action_select_new = '''  const handleActionSelect = (actionId: string) => {
    const actionInfo = getActionInfo(actionId);
    
    // Replace / with [Action] text inline
    const currentValue = displayValue;
    const cursorPosition = textAreaRef.current?.selectionStart || 0;
    const beforeCursor = currentValue.slice(0, cursorPosition);
    const lastSlashIndex = beforeCursor.lastIndexOf('/');
    
    if (lastSlashIndex !== -1) {
      const afterSlash = beforeCursor.slice(lastSlashIndex + 1);
      if (!afterSlash.includes(' ') && !afterSlash.includes('\\n')) {
        // Replace / with [Action] text
        const beforeSlash = currentValue.slice(0, lastSlashIndex);
        const afterCursor = currentValue.slice(cursorPosition);
        const actionText = `[${actionInfo.label}]`;
        const newValue = beforeSlash + actionText + afterCursor;
        
        setDisplayValue(newValue);
        setValue(newValue);
        
        // Track action for pill display
        const newAction: ActionData = {
          id: Date.now().toString() + Math.random().toString(36).substr(2, 9),
          actionId,
          label: actionInfo.label,
          icon: actionInfo.icon,
          position: lastSlashIndex,
        };
        
        setActions(prev => [...prev, newAction]);
        
        // Set cursor position after the action text
        setTimeout(() => {
          if (textAreaRef.current) {
            const newCursorPosition = lastSlashIndex + actionText.length;
            textAreaRef.current.setSelectionRange(newCursorPosition, newCursorPosition);
          }
        }, 0);
      }
    }
    
    console.log('Action selected:', actionId);
    setActionPopover(prev => ({ ...prev, isOpen: false }));
  };'''

# Find the function more carefully
lines = content.split('\n')
start_idx = -1
end_idx = -1
brace_count = 0
in_function = False

for i, line in enumerate(lines):
    if 'const handleActionSelect = (actionId: string) => {' in line:
        start_idx = i
        in_function = True
        brace_count = line.count('{') - line.count('}')
    elif in_function:
        brace_count += line.count('{') - line.count('}')
        if brace_count == 0:
            end_idx = i
            break

if start_idx != -1 and end_idx != -1:
    # Replace the function
    new_lines = lines[:start_idx] + handle_action_select_new.split('\n') + lines[end_idx + 1:]
    content = '\n'.join(new_lines)

print("ChatInput.tsx updated successfully")

# Write the modified content back
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)
