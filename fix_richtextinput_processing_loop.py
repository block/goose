import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing RichChatInput processing loop to handle both actions and mentions...")

# Find the entire processing loop section and replace it
# Look for the section that starts with "Process all matches in order"
old_processing_section = '''    // Process all matches in order
    for (const matchData of allMatches) {
      const { type, match, index, length, content } = matchData;
      console.log('✅ Found action match:', match);
      const beforeAction = value.slice(lastIndex, match.index);
      const actionLabel = match[1];
      console.log('📝 Processing action:', { beforeAction, actionLabel, matchIndex: match.index });
      
      // Add text before action with potential cursor
      if (beforeAction) {
        // Split text by cursor position to insert cursor at the right spot
        let textWithCursor = [];
        for (let i = 0; i < beforeAction.length; i++) {
          if (isFocused && cursorPosition === currentPos) {
            textWithCursor.push(
              <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
            );
          }
          textWithCursor.push(beforeAction[i]);
          currentPos++;
        }
        
        parts.push(
          <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
            {textWithCursor}
          </span>
        );
      }
      

      
      // Find matching action
      const actionEntry = Object.entries(ACTION_MAP).find(
        ([_, config]) => config.label === actionLabel
      );
      
      if (actionEntry) {
        const [actionId, config] = actionEntry;
        parts.push(
          <ActionPill
            key={`action-${keyCounter++}`}
            actionId={actionId}
            label={config.label}
            icon={config.icon}
            variant="default"
            size="sm"
            onRemove={() => handleRemoveAction(actionLabel)}
          />
        );
      } else {
        // If no matching action, render as text
        parts.push(
          <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
            {match[0]}
          </span>
        );
      }
      
      currentPos += match[0].length;
      lastIndex = match.index + match[0].length;
    }'''

new_processing_section = '''    // Process all matches in order
    for (const matchData of allMatches) {
      const { type, match, index, length, content } = matchData;
      console.log('✅ Found match:', { type, content, index });
      
      // Add text before this match with potential cursor
      const beforeMatch = value.slice(lastIndex, index);
      if (beforeMatch) {
        let textWithCursor = [];
        for (let i = 0; i < beforeMatch.length; i++) {
          if (isFocused && cursorPosition === currentPos) {
            textWithCursor.push(
              <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
            );
          }
          textWithCursor.push(beforeMatch[i]);
          currentPos++;
        }
        
        parts.push(
          <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
            {textWithCursor}
          </span>
        );
      }
      
      // Add cursor before match if needed
      if (isFocused && cursorPosition === currentPos) {
        parts.push(
          <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
        );
      }
      
      if (type === 'action') {
        // Handle action pills
        const actionLabel = content;
        const actionEntry = Object.entries(ACTION_MAP).find(
          ([_, config]) => config.label === actionLabel
        );
        
        console.log('🏷️ Creating action pill:', { actionLabel, actionEntry });
        
        if (actionEntry) {
          const [actionId, config] = actionEntry;
          parts.push(
            <ActionPill
              key={`action-${keyCounter++}`}
              actionId={actionId}
              label={config.label}
              icon={config.icon}
              variant="default"
              size="sm"
              onRemove={() => handleRemoveAction(actionLabel)}
            />
          );
        } else {
          // If no matching action, render as text
          parts.push(
            <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
              {match[0]}
            </span>
          );
        }
      } else if (type === 'mention') {
        // Handle mention pills
        const fileName = content; // filename without @
        const filePath = `@${fileName}`; // full mention text
        
        console.log('📁 Creating mention pill:', { fileName, filePath });
        
        parts.push(
          <MentionPill
            key={`mention-${keyCounter++}`}
            fileName={fileName}
            filePath={filePath}
            variant="default"
            size="sm"
            onRemove={() => handleRemoveMention(fileName)}
          />
        );
      }
      
      currentPos += length;
      lastIndex = index + length;
    }'''

content = content.replace(old_processing_section, new_processing_section)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed RichChatInput processing loop:")
print("   - Updated to use new combined action/mention processing")
print("   - Added proper mention pill creation")
print("   - Added debugging logs for both types")
print("   - Should now render @filename as green pills")
