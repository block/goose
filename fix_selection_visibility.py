import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing selection visibility to show over visual content...")

# The issue is that selection is happening on the hidden textarea but we need it visible over the visual layer
# Let's make the hidden textarea slightly more visible and add a selection overlay

# Update the hidden textarea to be slightly more visible during selection
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
        }}
        className="absolute inset-0 w-full h-full resize-none selection:bg-blue-500 selection:text-white"'''

new_hidden_textarea_style = '''        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          width: '100%',
          height: '100%',
          opacity: 0.1, // More visible for selection
          zIndex: 2, // Higher z-index to capture mouse events
          background: 'transparent',
          border: 'none',
          outline: 'none',
          resize: 'none',
          color: 'rgba(59, 130, 246, 0.1)', // Very light blue text
          caretColor: 'transparent', // Hide caret (we show our own)
          pointerEvents: 'auto', // Ensure it can receive mouse events
          fontFamily: 'inherit',
          fontSize: 'inherit',
          lineHeight: 'inherit',
          padding: 'inherit',
        }}
        className="absolute inset-0 w-full h-full resize-none selection:bg-blue-500 selection:text-white"'''

content = content.replace(old_hidden_textarea_style, new_hidden_textarea_style)

# Also update the CSS to make selection more prominent
css_update = '''
/* Enhanced text selection styling */
.rich-text-input textarea::selection {
  background-color: #3b82f6 !important; /* Blue-500 with !important */
  color: white !important;
  opacity: 1 !important;
}

.rich-text-input textarea::-moz-selection {
  background-color: #3b82f6 !important; /* Blue-500 with !important */
  color: white !important;
  opacity: 1 !important;
}

/* Make sure selection is visible over visual content */
.rich-text-input {
  position: relative;
}

.rich-text-input textarea {
  mix-blend-mode: multiply; /* Blend with background for better visibility */
}
'''

# Update the main.css file
with open('ui/desktop/src/styles/main.css', 'r') as f:
    css_content = f.read()

# Replace the existing selection CSS with enhanced version
old_selection_css = '''/* Text selection styling for RichChatInput */
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
}'''

if old_selection_css in css_content:
    css_content = css_content.replace(old_selection_css, css_update)
else:
    css_content += css_update

with open('ui/desktop/src/styles/main.css', 'w') as f:
    f.write(css_content)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Enhanced selection visibility:")
print("   - Increased textarea opacity to 0.1 for better selection visibility")
print("   - Added light blue text color for better contrast")
print("   - Added !important to selection CSS rules")
print("   - Added mix-blend-mode for better visual blending")
print("   - Selection should now be much more visible")
