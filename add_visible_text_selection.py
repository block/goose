import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Adding visible blue text selection highlight...")

# Add selection styling to the hidden textarea
old_hidden_textarea_style = '''        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          width: '100%',
          height: '100%',
          opacity: 0.01, // Nearly invisible but still selectable
          zIndex: 2, // Higher z-index to capture mouse events
          background: 'transparent',
          border: 'none',
          outline: 'none',
          resize: 'none',
          color: 'transparent', // Hide text but keep selection
          caretColor: 'transparent', // Hide caret (we show our own)
          pointerEvents: 'auto', // Ensure it can receive mouse events
        }}'''

new_hidden_textarea_style = '''        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          width: '100%',
          height: '100%',
          opacity: 0.01, // Nearly invisible but still selectable
          zIndex: 2, // Higher z-index to capture mouse events
          background: 'transparent',
          border: 'none',
          outline: 'none',
          resize: 'none',
          color: 'transparent', // Hide text but keep selection
          caretColor: 'transparent', // Hide caret (we show our own)
          pointerEvents: 'auto', // Ensure it can receive mouse events
        }}
        className="absolute inset-0 w-full h-full resize-none selection:bg-blue-500 selection:text-white"'''

content = content.replace(old_hidden_textarea_style, new_hidden_textarea_style)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

# Also add CSS to the main.css file for better selection styling
with open('ui/desktop/src/styles/main.css', 'r') as f:
    css_content = f.read()

# Add selection styling at the end of the file
selection_css = '''
/* Text selection styling for RichChatInput */
.rich-text-input textarea::selection {
  background-color: #3b82f6; /* Blue-500 */
  color: white;
}

.rich-text-input textarea::-moz-selection {
  background-color: #3b82f6; /* Blue-500 */
  color: white;
}

/* Global selection styling for better visibility */
::selection {
  background-color: #3b82f6; /* Blue-500 */
  color: white;
}

::-moz-selection {
  background-color: #3b82f6; /* Blue-500 */
  color: white;
}
'''

if '/* Text selection styling for RichChatInput */' not in css_content:
    css_content += selection_css
    
    with open('ui/desktop/src/styles/main.css', 'w') as f:
        f.write(css_content)
    
    print("✅ Added selection CSS to main.css")

# Also add a wrapper class to the RichChatInput component
old_wrapper = '''  return (
    <div className="relative">'''

new_wrapper = '''  return (
    <div className="relative rich-text-input">'''

content = content.replace(old_wrapper, new_wrapper)

# Write the final content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Added visible text selection highlighting:")
print("   - Added selection:bg-blue-500 selection:text-white to textarea")
print("   - Added CSS rules for ::selection styling")
print("   - Added rich-text-input wrapper class")
print("   - Selection should now be clearly visible with blue highlight")
