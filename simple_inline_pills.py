import re

# Read the ChatInput file
with open('ui/desktop/src/components/ChatInput.tsx', 'r') as f:
    content = f.read()

# Simple approach: Just remove the "Selected Actions Pills" section
# Find the section that starts with {/* Selected Actions Pills */}
start_marker = '{/* Selected Actions Pills */}'
start_pos = content.find(start_marker)

if start_pos != -1:
    # Find the end of this section - look for the closing div and brace
    # We need to find the matching closing brace for the conditional
    lines = content[start_pos:].split('\n')
    brace_count = 0
    end_line = 0
    
    for i, line in enumerate(lines):
        if i == 0:  # First line with the comment
            continue
        
        # Count braces to find the end of the conditional
        open_braces = line.count('{')
        close_braces = line.count('}')
        brace_count += open_braces - close_braces
        
        # Look for the end pattern
        if ')}\n' in line or line.strip() == ')}':
            end_line = i + 1
            break
    
    if end_line > 0:
        # Calculate the end position
        end_pos = start_pos + len('\n'.join(lines[:end_line]))
        
        # Remove the section
        content = content[:start_pos] + content[end_pos:]
        
        print(f"Removed Selected Actions Pills section from position {start_pos} to {end_pos}")
    else:
        print("Could not find end of Selected Actions Pills section")
else:
    print("Selected Actions Pills section not found")

# Write the modified content back
with open('ui/desktop/src/components/ChatInput.tsx', 'w') as f:
    f.write(content)

print("ChatInput.tsx updated - Selected Actions Pills section removed")
