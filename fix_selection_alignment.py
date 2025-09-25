import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing selection alignment - making hidden textarea match visual display exactly...")

# The issue is that the hidden textarea and visual display have different positioning/padding
# We need to make them align perfectly

# Update the hidden textarea to match the visual display exactly
old_hidden_textarea_style = '''        style={{
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
        }}'''

new_hidden_textarea_style = '''        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          width: '100%',
          height: '100%',
          opacity: 0.15, // Slightly more visible for selection
          zIndex: 2, // Higher z-index to capture mouse events
          background: 'transparent',
          border: 'none',
          outline: 'none',
          resize: 'none',
          color: 'rgba(59, 130, 246, 0.2)', // Slightly more visible blue text
          caretColor: 'transparent', // Hide caret (we show our own)
          pointerEvents: 'auto', // Ensure it can receive mouse events
          fontFamily: 'Cash Sans, sans-serif', // Match exact font
          fontSize: '0.875rem', // Match text-sm (14px)
          lineHeight: '1.5', // Match leading-relaxed
          padding: '0', // No padding to match visual display
          margin: '0',
          boxSizing: 'border-box',
          whiteSpace: 'pre-wrap', // Match visual display
          wordWrap: 'break-word',
        }}'''

content = content.replace(old_hidden_textarea_style, new_hidden_textarea_style)

# Also need to make sure the visual display has consistent styling
old_visual_display = '''      {/* Visual display with action pills and cursor */}
      <div
        ref={displayRef}
        className={`${className} cursor-text relative`}
        style={{
          ...style,
          minHeight: `${rows * 1.5}em`,
          zIndex: 1, // Lower z-index, behind textarea
          pointerEvents: 'none', // Don't interfere with text selection
          userSelect: 'none', // Prevent selection on visual layer
          WebkitUserSelect: 'none',
        }}
        role="textbox"
        aria-multiline="true"
        aria-placeholder={placeholder}
      >
        {renderContent()}
      </div>'''

new_visual_display = '''      {/* Visual display with action pills and cursor */}
      <div
        ref={displayRef}
        className={`${className} cursor-text relative`}
        style={{
          ...style,
          minHeight: `${rows * 1.5}em`,
          zIndex: 1, // Lower z-index, behind textarea
          pointerEvents: 'none', // Don't interfere with text selection
          userSelect: 'none', // Prevent selection on visual layer
          WebkitUserSelect: 'none',
          fontFamily: 'Cash Sans, sans-serif', // Match textarea font
          fontSize: '0.875rem', // Match textarea size
          lineHeight: '1.5', // Match textarea line height
          padding: '0', // Match textarea padding
          margin: '0',
          whiteSpace: 'pre-wrap', // Match textarea
          wordWrap: 'break-word',
        }}
        role="textbox"
        aria-multiline="true"
        aria-placeholder={placeholder}
      >
        {renderContent()}
      </div>'''

content = content.replace(old_visual_display, new_visual_display)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed selection alignment:")
print("   - Matched font family, size, and line height exactly")
print("   - Removed padding/margin differences")
print("   - Added consistent white-space and word-wrap")
print("   - Selection should now align perfectly with visual text")
print("   - No more 'holographic' offset effect")
