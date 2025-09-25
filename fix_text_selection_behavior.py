import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing text selection behavior to work like a normal text input...")

# The main issue is that we need to make the hidden textarea more accessible
# and reduce interference from the visual display layer

# First, let's update the hidden textarea to be more interactive
old_hidden_textarea_style = '''        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          width: '100%',
          height: '100%',
          opacity: 0.01, // Nearly invisible but still selectable
          zIndex: 1,
          background: 'transparent',
          border: 'none',
          outline: 'none',
          resize: 'none',
          color: 'transparent', // Hide text but keep selection
          caretColor: 'transparent', // Hide caret (we show our own)
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
        }}'''

content = content.replace(old_hidden_textarea_style, new_hidden_textarea_style)

# Update the visual display to have lower z-index and not interfere with selection
old_visual_display = '''      {/* Visual display with action pills and cursor */}
      <div
        ref={displayRef}
        onClick={handleDisplayClick}
        onDoubleClick={handleDisplayClick}
        className={`${className} cursor-text relative`}
        style={{
          ...style,
          minHeight: `${rows * 1.5}em`,
          zIndex: 0,
          userSelect: 'text', // Allow text selection
          WebkitUserSelect: 'text', // Safari support
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
        }}
        role="textbox"
        aria-multiline="true"
        aria-placeholder={placeholder}
      >
        {renderContent()}
      </div>'''

content = content.replace(old_visual_display, new_visual_display)

# Remove the handleDisplayClick since we're not using it anymore
old_handle_display_click = '''  const handleDisplayClick = useCallback((e: React.MouseEvent) => {
    // Don't focus if clicking on an action pill
    if ((e.target as Element).closest('.action-pill-wrapper')) {
      return;
    }
    
    // Focus the hidden textarea for text selection
    if (hiddenTextareaRef.current) {
      hiddenTextareaRef.current.focus();
      
      // For double/triple click, select text appropriately
      if (e.detail === 2) {
        // Double click - select word (let browser handle this)
        setTimeout(() => {
          if (hiddenTextareaRef.current) {
            hiddenTextareaRef.current.select();
          }
        }, 0);
      } else if (e.detail === 3) {
        // Triple click - select all
        setTimeout(() => {
          if (hiddenTextareaRef.current) {
            hiddenTextareaRef.current.select();
          }
        }, 0);
      }
    }
  }, []);'''

new_handle_display_click = '''  // Removed handleDisplayClick - let the hidden textarea handle all mouse events naturally'''

content = content.replace(old_handle_display_click, new_handle_display_click)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed text selection behavior:")
print("   - Made hidden textarea capture all mouse events (higher z-index)")
print("   - Made visual display non-interactive (pointerEvents: 'none')")
print("   - Removed interfering click handlers")
print("   - Should now behave like a normal text input for selection")
print("   - Clicking, dragging, double-click, triple-click should all work normally")
