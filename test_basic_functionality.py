# Let's create a simple test to verify if the basic text replacement is working
print("🧪 Testing Basic Text Replacement")
print("=" * 50)

# Read the handleActionSelect function
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

# Check if the function exists and has the right structure
if 'handleActionSelect' in content:
    print("✅ handleActionSelect function found")
    
    # Check for key components
    if 'setDisplayValue(newValue)' in content:
        print("✅ setDisplayValue call found")
    if 'setValue(newValue)' in content:
        print("✅ setValue call found")
    if 'actionInfo.label' in content:
        print("✅ actionInfo.label usage found")
    if 'lastSlashIndex' in content:
        print("✅ slash detection logic found")
    
    # Extract the function for analysis
    start = content.find('const handleActionSelect = (actionId: string) => {')
    if start != -1:
        # Find the end of the function
        brace_count = 0
        pos = start
        while pos < len(content):
            if content[pos] == '{':
                brace_count += 1
            elif content[pos] == '}':
                brace_count -= 1
                if brace_count == 0:
                    end = pos + 1
                    break
            pos += 1
        
        function_code = content[start:end]
        print(f"\n📝 Function length: {len(function_code)} characters")
        
        # Check for potential issues
        if 'cursorPosition = 0' in function_code:
            print("⚠️  WARNING: cursorPosition hardcoded to 0")
        if 'actionPopover.cursorPosition' in function_code:
            print("✅ Using actionPopover.cursorPosition")
        if 'console.log' in function_code:
            print("✅ Debug logging present")
            
else:
    print("❌ handleActionSelect function not found")

print("\n🔍 Checking RichChatInput integration...")

# Check RichChatInput
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    rich_content = f.read()

if 'actionRegex = /\\[([^\\]]+)\\]/g' in rich_content:
    print("✅ Action regex pattern found")
if 'ActionPill' in rich_content:
    print("✅ ActionPill component imported")
if 'renderContent' in rich_content:
    print("✅ renderContent function found")

print("\n🎯 Next Steps:")
print("1. Open browser dev tools")
print("2. Type '/' in chat input")
print("3. Click an action")
print("4. Look for these console messages:")
print("   - 🎯 ActionPopover: handleItemClick called")
print("   - 🎯 handleActionSelect called with:")
print("   - 📍 Current state:")
print("   - 🔄 Text replacement:")
print("   - 🎨 RichChatInput renderContent called")

print("\n💡 If no console logs appear:")
print("- ActionPopover might not be calling onSelect")
print("- handleActionSelect might not be connected")
print("- Check for JavaScript errors in console")
