import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Updating RichChatInput to support mention pills...")

# Add MentionPill import
old_imports = "import { ActionPill } from './ActionPill';"
new_imports = """import { ActionPill } from './ActionPill';
import MentionPill from './MentionPill';"""

content = content.replace(old_imports, new_imports)

# Update the renderContent function to handle both actions and mentions
# Find the while loop that processes actions and extend it to handle mentions too
old_regex_section = '''    const parts: React.ReactNode[] = [];
    const actionRegex = /\[([^\]]+)\]/g;
    let lastIndex = 0;
    let match;
    let keyCounter = 0;
    let currentPos = 0; // Track position for cursor placement'''

new_regex_section = '''    const parts: React.ReactNode[] = [];
    const actionRegex = /\[([^\]]+)\]/g;
    const mentionRegex = /@([^\s]+)/g; // Match @filename patterns
    let lastIndex = 0;
    let match;
    let keyCounter = 0;
    let currentPos = 0; // Track position for cursor placement'''

content = content.replace(old_regex_section, new_regex_section)

# Now we need to process both actions and mentions in the correct order
# Find the existing while loop and replace it with a more comprehensive approach
old_while_loop_start = '''    console.log('🎨 RichChatInput renderContent called with value:', value);
    console.log('🔍 Looking for action patterns with regex:', actionRegex);
    
    while ((match = actionRegex.exec(value)) !== null) {'''

new_while_loop_start = '''    console.log('🎨 RichChatInput renderContent called with value:', value);
    console.log('🔍 Looking for action and mention patterns with regex:', { actionRegex, mentionRegex });
    
    // Find all actions and mentions, then sort by position
    const allMatches = [];
    
    // Find all action matches
    let actionMatch;
    actionRegex.lastIndex = 0; // Reset regex
    while ((actionMatch = actionRegex.exec(value)) !== null) {
      allMatches.push({
        type: 'action',
        match: actionMatch,
        index: actionMatch.index,
        length: actionMatch[0].length,
        content: actionMatch[1]
      });
    }
    
    // Find all mention matches
    let mentionMatch;
    mentionRegex.lastIndex = 0; // Reset regex
    while ((mentionMatch = mentionRegex.exec(value)) !== null) {
      allMatches.push({
        type: 'mention',
        match: mentionMatch,
        index: mentionMatch.index,
        length: mentionMatch[0].length,
        content: mentionMatch[1] // filename without @
      });
    }
    
    // Sort matches by position
    allMatches.sort((a, b) => a.index - b.index);
    
    console.log('🔍 Found matches:', allMatches);
    
    // Process all matches in order
    for (const matchData of allMatches) {
      const { type, match, index, length, content } = matchData;'''

content = content.replace(old_while_loop_start, new_while_loop_start)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Updated RichChatInput imports and regex setup:")
print("   - Added MentionPill import")
print("   - Added mentionRegex to detect @filename patterns")
print("   - Started updating processing logic to handle both actions and mentions")
print("   - Next step: Update the processing loop to render mention pills")
