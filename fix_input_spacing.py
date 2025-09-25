import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing input spacing to match original ChatInput padding...")

# The original ChatInput has: px-3 pt-3 pb-1.5 pr-20 text-sm
# That translates to: padding: 12px 80px 6px 12px; font-size: 14px;

# Update both the hidden textarea and visual display to have proper padding
old_hidden_textarea_style = '''        style={{
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
          padding: '12px 80px 6px 12px', // Match original: px-3 pt-3 pb-1.5 pr-20
          margin: '0',
          boxSizing: 'border-box',
          whiteSpace: 'pre-wrap', // Match visual display
          wordWrap: 'break-word',
        }}'''

content = content.replace(old_hidden_textarea_style, new_hidden_textarea_style)

# Update the visual display to have matching padding
old_visual_display_style = '''        style={{
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
        }}'''

new_visual_display_style = '''        style={{
          ...style,
          minHeight: `${rows * 1.5}em`,
          zIndex: 1, // Lower z-index, behind textarea
          pointerEvents: 'none', // Don't interfere with text selection
          userSelect: 'none', // Prevent selection on visual layer
          WebkitUserSelect: 'none',
          fontFamily: 'Cash Sans, sans-serif', // Match textarea font
          fontSize: '0.875rem', // Match textarea size
          lineHeight: '1.5', // Match textarea line height
          padding: '12px 80px 6px 12px', // Match original: px-3 pt-3 pb-1.5 pr-20
          margin: '0',
          whiteSpace: 'pre-wrap', // Match textarea
          wordWrap: 'break-word',
        }}'''

content = content.replace(old_visual_display_style, new_visual_display_style)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed input spacing:")
print("   - Added proper padding: 12px 80px 6px 12px (px-3 pt-3 pb-1.5 pr-20)")
print("   - Left padding: 12px (px-3)")
print("   - Top padding: 12px (pt-3)")
print("   - Bottom padding: 6px (pb-1.5)")
print("   - Right padding: 80px (pr-20) - space for action buttons")
print("   - Both layers now have matching spacing")
