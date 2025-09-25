import re

# Read the ActionPopover file
with open('ui/desktop/src/components/ActionPopover.tsx', 'r') as f:
    content = f.read()

# Add debugging to handleItemClick
old_handleItemClick = '''  const handleItemClick = (index: number) => {
    onSelectedIndexChange(index);
    onSelect(actions[index].id);
    actions[index].action();
    onClose();
  };'''

new_handleItemClick = '''  const handleItemClick = (index: number) => {
    console.log('🎯 ActionPopover: handleItemClick called', { index, actionId: actions[index].id });
    console.log('📋 ActionPopover: onSelect function:', onSelect);
    console.log('🔄 ActionPopover: About to call onSelect with:', actions[index].id);
    
    onSelectedIndexChange(index);
    
    // Call onSelect first - this should trigger handleActionSelect in ChatInput
    onSelect(actions[index].id);
    
    console.log('✅ ActionPopover: onSelect called successfully');
    
    // Call the local action (just for logging)
    actions[index].action();
    
    console.log('🚪 ActionPopover: Closing popover');
    onClose();
  };'''

content = content.replace(old_handleItemClick, new_handleItemClick)

# Also add debugging to the selectAction method in useImperativeHandle
old_selectAction = '''      selectAction: (index: number) => {
        if (actions[index]) {
          onSelect(actions[index].id);
          actions[index].action();
          onClose();
        }
      },'''

new_selectAction = '''      selectAction: (index: number) => {
        console.log('⌨️ ActionPopover: selectAction called via keyboard', { index, actionId: actions[index]?.id });
        if (actions[index]) {
          console.log('🔄 ActionPopover: Calling onSelect from selectAction:', actions[index].id);
          onSelect(actions[index].id);
          actions[index].action();
          onClose();
        }
      },'''

content = content.replace(old_selectAction, new_selectAction)

# Write the modified content back
with open('ui/desktop/src/components/ActionPopover.tsx', 'w') as f:
    f.write(content)

print("Added comprehensive debugging to ActionPopover")
