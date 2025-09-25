import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Adding comprehensive debugging for mention detection...")

# Find the mention regex section and add more debugging
old_mention_detection = '''    // Find all mention matches
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
    }'''

new_mention_detection = '''    // Find all mention matches
    let mentionMatch;
    mentionRegex.lastIndex = 0; // Reset regex
    console.log('🔍 Searching for mentions in value:', value);
    console.log('🔍 Using mention regex:', mentionRegex);
    while ((mentionMatch = mentionRegex.exec(value)) !== null) {
      console.log('📁 Found mention match:', mentionMatch);
      allMatches.push({
        type: 'mention',
        match: mentionMatch,
        index: mentionMatch.index,
        length: mentionMatch[0].length,
        content: mentionMatch[1] // filename without @
      });
    }'''

content = content.replace(old_mention_detection, new_mention_detection)

# Also add debugging after sorting matches
old_sort_section = '''    // Sort matches by position
    allMatches.sort((a, b) => a.index - b.index);
    
    console.log('🔍 Found matches:', allMatches);'''

new_sort_section = '''    // Sort matches by position
    allMatches.sort((a, b) => a.index - b.index);
    
    console.log('🔍 All matches found:', allMatches);
    console.log('📊 Match breakdown:', {
      actions: allMatches.filter(m => m.type === 'action').length,
      mentions: allMatches.filter(m => m.type === 'mention').length,
      total: allMatches.length
    });'''

content = content.replace(old_sort_section, new_sort_section)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Added comprehensive debugging:")
print("   - Debug logs for mention regex search")
print("   - Debug logs for found mention matches")
print("   - Match breakdown statistics")
print("   - Should help identify if mentions are being detected")

# Also let's test the regex pattern directly
import re

test_strings = [
    "@filename.txt",
    "Hello @test.py world",
    "@folder/file.js",
    "Multiple @file1.txt and @file2.py mentions"
]

mention_regex = re.compile(r'@([^\s]+)')

print("\n🧪 Testing mention regex pattern:")
for test in test_strings:
    matches = mention_regex.findall(test)
    print(f"  '{test}' → {matches}")
