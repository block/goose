import re

# Read the ActionPopover file
with open('ui/desktop/src/components/ActionPopover.tsx', 'r') as f:
    content = f.read()

# Fix the order of operations in handleItemClick - onSelect should happen BEFORE onClose
old_handleItemClick = '''  const handleItemClick = (index: number) => {
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
    
    // Close popover after a small delay to allow text replacement to complete
    console.log('🚪 ActionPopover: Closing popover after delay');
    setTimeout(() => {
      onClose();
    }, 10);
  };'''

content = content.replace(old_handleItemClick, new_handleItemClick)

# Also fix the selectAction method
old_selectAction = '''      selectAction: (index: number) => {
        console.log('⌨️ ActionPopover: selectAction called via keyboard', { index, actionId: actions[index]?.id });
        if (actions[index]) {
          console.log('🔄 ActionPopover: Calling onSelect from selectAction:', actions[index].id);
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
          setTimeout(() => {
            onClose();
          }, 10);
        }
      },'''

content = content.replace(old_selectAction, new_selectAction)

# Write the modified content back
with open('ui/desktop/src/components/ActionPopover.tsx', 'w') as f:
    f.write(content)

print("Fixed ActionPopover to delay onClose() call")
