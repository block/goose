import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing cursor positioning logic for line breaks...")

# The issue is in how we're processing character by character
# We need to handle newlines properly and render text as continuous flow
# Let's rewrite the text processing logic

old_text_processing = '''      // Add text before action with potential cursor
      if (beforeAction) {
        const textParts = [];
        for (let i = 0; i < beforeAction.length; i++) {
          const char = beforeAction[i];
          addCursorIfNeeded(currentPos);
          textParts.push(char);
          currentPos++;
        }
        
        parts.push(
          <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
            {textParts}
          </span>
        );
      }'''

new_text_processing = '''      // Add text before action with potential cursor
      if (beforeAction) {
        // Split text by cursor position to insert cursor at the right spot
        let textWithCursor = [];
        for (let i = 0; i < beforeAction.length; i++) {
          if (isFocused && cursorPosition === currentPos) {
            textWithCursor.push(
              <span key={`cursor-${keyCounter++}`} className="border-l-2 border-black inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.2em", marginLeft: "1px" }} />
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
      }'''

content = content.replace(old_text_processing, new_text_processing)

# Also fix the remaining text processing
old_remaining_processing = '''    // Add remaining text with potential cursor
    const remainingText = value.slice(lastIndex);
    if (remainingText) {
      const textParts = [];
      for (let i = 0; i < remainingText.length; i++) {
        const char = remainingText[i];
        addCursorIfNeeded(currentPos);
        textParts.push(char);
        currentPos++;
      }
      
      parts.push(
        <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
          {textParts}
        </span>
      );
    }'''

new_remaining_processing = '''    // Add remaining text with potential cursor
    const remainingText = value.slice(lastIndex);
    if (remainingText) {
      let textWithCursor = [];
      for (let i = 0; i < remainingText.length; i++) {
        if (isFocused && cursorPosition === currentPos) {
          textWithCursor.push(
            <span key={`cursor-${keyCounter++}`} className="border-l-2 border-black inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.2em", marginLeft: "1px" }} />
          );
        }
        textWithCursor.push(remainingText[i]);
        currentPos++;
      }
      
      parts.push(
        <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
          {textWithCursor}
        </span>
      );
    }'''

content = content.replace(old_remaining_processing, new_remaining_processing)

# Remove the separate addCursorIfNeeded calls since we're handling it inline now
content = content.replace('      // Add cursor before action if needed\n      addCursorIfNeeded(currentPos);', '')
content = content.replace('    // Add cursor at the end if needed\n    addCursorIfNeeded(currentPos);', '''    // Add cursor at the end if needed
    if (isFocused && cursorPosition === currentPos) {
      parts.push(
        <span key={`cursor-${keyCounter++}`} className="border-l-2 border-black inline-block" style={{ animation: "blink 1s step-end infinite", height: "1.2em", marginLeft: "1px" }} />
      );
    }''')

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed cursor positioning logic:")
print("   - Integrated cursor rendering directly into text processing")
print("   - Cursor now appears inline with text characters")
print("   - Should handle line breaks properly")
print("   - Cursor stays at correct position within the text flow")
