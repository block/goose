import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Completing mention pill rendering in RichChatInput...")

# Find the section after we set up the matches and complete the processing logic
# We need to replace the old action-only processing with the new combined processing

# First, let's find where the old processing logic ends and replace it
# Look for the section that processes beforeAction
old_processing_section = '''      console.log('✅ Found action match:', match);
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

new_processing_section = '''      console.log('✅ Found match:', matchData);
      
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

# Add the handleRemoveMention function
old_handle_remove = '''  const handleRemoveAction = useCallback((actionLabel: string) => {
    const actionText = `[${actionLabel}]`;
    const newValue = value.replace(actionText, '');
    onChange(newValue);
  }, [value, onChange]);'''

new_handle_remove = '''  const handleRemoveAction = useCallback((actionLabel: string) => {
    const actionText = `[${actionLabel}]`;
    const newValue = value.replace(actionText, '');
    onChange(newValue);
  }, [value, onChange]);

  const handleRemoveMention = useCallback((fileName: string) => {
    const mentionText = `@${fileName}`;
    const newValue = value.replace(mentionText, '');
    onChange(newValue);
  }, [value, onChange]);'''

content = content.replace(old_handle_remove, new_handle_remove)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Completed mention pill rendering:")
print("   - Updated processing logic to handle both actions and mentions")
print("   - Added MentionPill rendering for @filename patterns")
print("   - Added handleRemoveMention function for pill removal")
print("   - Mentions will now appear as green pills with file icon")
