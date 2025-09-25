import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

# Add debugging to the renderContent function
# Find the while loop that processes actions
old_while_section = '''    while ((match = actionRegex.exec(value)) !== null) {
      const beforeAction = value.slice(lastIndex, match.index);
      const actionLabel = match[1];
      
      // Add text before action with potential cursor
      if (beforeAction) {'''

new_while_section = '''    console.log('🎨 RichChatInput renderContent called with value:', value);
    console.log('🔍 Looking for action patterns with regex:', actionRegex);
    
    while ((match = actionRegex.exec(value)) !== null) {
      console.log('✅ Found action match:', match);
      const beforeAction = value.slice(lastIndex, match.index);
      const actionLabel = match[1];
      console.log('📝 Processing action:', { beforeAction, actionLabel, matchIndex: match.index });
      
      // Add text before action with potential cursor
      if (beforeAction) {'''

content = content.replace(old_while_section, new_while_section)

# Add debugging to the action pill creation
old_pill_creation = '''      // Add the action pill
      addCursorIfNeeded(currentPos);
      const actionInfo = ACTION_MAP[actionLabel.toLowerCase().replace(/\\s+/g, '-') as keyof typeof ACTION_MAP] || 
                        { label: actionLabel, icon: <Zap size={12} /> };
      
      parts.push(
        <ActionPill
          key={`action-${keyCounter++}`}
          actionId={actionLabel.toLowerCase().replace(/\\s+/g, '-')}
          label={actionLabel}
          icon={actionInfo.icon}
          variant="default"
          size="sm"
          onRemove={() => handleRemoveAction(actionLabel)}
        />
      );'''

new_pill_creation = '''      // Add the action pill
      addCursorIfNeeded(currentPos);
      const actionInfo = ACTION_MAP[actionLabel.toLowerCase().replace(/\\s+/g, '-') as keyof typeof ACTION_MAP] || 
                        { label: actionLabel, icon: <Zap size={12} /> };
      
      console.log('🏷️ Creating action pill:', { actionLabel, actionInfo });
      
      parts.push(
        <ActionPill
          key={`action-${keyCounter++}`}
          actionId={actionLabel.toLowerCase().replace(/\\s+/g, '-')}
          label={actionLabel}
          icon={actionInfo.icon}
          variant="default"
          size="sm"
          onRemove={() => handleRemoveAction(actionLabel)}
        />
      );
      
      console.log('✅ Action pill added to parts array');'''

content = content.replace(old_pill_creation, new_pill_creation)

# Add debugging to the onChange handler
old_onchange = '''  const handleTextareaChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    const newCursorPos = e.target.selectionStart;
    
    console.log('RichChatInput: onChange', { newValue, newCursorPos });
    onChange(newValue, newCursorPos);
    setCursorPosition(newCursorPos);
  }, [onChange]);'''

new_onchange = '''  const handleTextareaChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    const newCursorPos = e.target.selectionStart;
    
    console.log('🔄 RichChatInput: onChange', { newValue, newCursorPos });
    console.log('🎨 Will trigger re-render with new value:', newValue);
    onChange(newValue, newCursorPos);
    setCursorPosition(newCursorPos);
  }, [onChange]);'''

content = content.replace(old_onchange, new_onchange)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("Added comprehensive debugging to RichChatInput")
