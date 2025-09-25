import re

# Read the MessageContent file
with open('ui/desktop/src/components/MessageContent.tsx', 'r') as f:
    content = f.read()

print("Fixing MessageContent to display pills inline...")

# Replace the entire return statement to fix inline display
old_return = '''  return (
    <div className={`flex flex-wrap items-center gap-1 ${className || ''}`}>
      {parsedContent.map((part, index) => {
        if (part.type === 'action' && part.actionId) {
          const actionInfo = getActionInfo(part.actionId);
          return (
            <ActionPill
              key={`action-${index}`}
              actionId={part.actionId}
              label={part.content}
              icon={actionInfo.icon}
              variant="message"
              size="sm"
              // No onRemove for message display - pills are read-only
            />
          );
        } else if (part.content.trim()) {
          return (
            <MarkdownContent
              key={`text-${index}`}
              content={part.content}
              className={className}
            />
          );
        }
        return null;
      })}
    </div>
  );'''

new_return = '''  return (
    <span className={`inline-flex flex-wrap items-baseline gap-1 ${className || ''}`}>
      {parsedContent.map((part, index) => {
        if (part.type === 'action' && part.actionId) {
          const actionInfo = getActionInfo(part.actionId);
          return (
            <ActionPill
              key={`action-${index}`}
              actionId={part.actionId}
              label={part.content}
              icon={actionInfo.icon}
              variant="message"
              size="sm"
              // No onRemove for message display - pills are read-only
            />
          );
        } else if (part.content.trim()) {
          return (
            <span
              key={`text-${index}`}
              className={`inline ${className || ''}`}
              dangerouslySetInnerHTML={{
                __html: part.content
                  .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
                  .replace(/\*(.*?)\*/g, '<em>$1</em>')
                  .replace(/`(.*?)`/g, '<code>$1</code>')
                  .replace(/\n/g, '<br>')
              }}
            />
          );
        }
        return null;
      })}
    </span>
  );'''

content = content.replace(old_return, new_return)

# Write the modified content back
with open('ui/desktop/src/components/MessageContent.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed MessageContent to display pills inline with text")
print("Key changes:")
print("1. Changed container from <div> to <span> with inline-flex")
print("2. Replaced MarkdownContent with inline <span> and basic HTML formatting")
print("3. Used items-baseline for better text alignment")
print("4. Applied inline styling to prevent line breaks")
