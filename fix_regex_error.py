import re

# Read the MessageContent file
with open('ui/desktop/src/components/MessageContent.tsx', 'r') as f:
    content = f.read()

print("Fixing regex syntax error in MessageContent...")

# Fix the broken regex - the newline replacement got split
old_broken_html = '''              dangerouslySetInnerHTML={{
                __html: part.content
                  .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
                  .replace(/\*(.*?)\*/g, '<em>$1</em>')
                  .replace(/`(.*?)`/g, '<code>$1</code>')
                  .replace(/
/g, '<br>')
              }}'''

new_fixed_html = '''              dangerouslySetInnerHTML={{
                __html: part.content
                  .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
                  .replace(/\*(.*?)\*/g, '<em>$1</em>')
                  .replace(/`(.*?)`/g, '<code>$1</code>')
                  .replace(/\\n/g, '<br>')
              }}'''

content = content.replace(old_broken_html, new_fixed_html)

# Write the modified content back
with open('ui/desktop/src/components/MessageContent.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed regex syntax error")
print("Changed .replace(/\\n/g, '<br>') - newline regex was broken across lines")
