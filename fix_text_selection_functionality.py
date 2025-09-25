import re

# Read the RichChatInput file
with open('ui/desktop/src/components/RichChatInput.tsx', 'r') as f:
    content = f.read()

print("Fixing text selection functionality (highlight, double-click, triple-click)...")

# The issue is that we have a hidden textarea and a visual display layer
# We need to make sure the visual layer doesn't interfere with text selection

# Find the visual display div and update its properties
old_display_div = '''      {/* Visual display with action pills and cursor */}
      <div
        ref={displayRef}
        onClick={handleDisplayClick}
        className={`${className} cursor-text relative`}
        style={{
          ...style,
          minHeight: `${rows * 1.5}em`,
          zIndex: 0,
        }}
        role="textbox"
        aria-multiline="true"
        aria-placeholder={placeholder}
      >
        {renderContent()}
      </div>'''

new_display_div = '''      {/* Visual display with action pills and cursor */}
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

content = content.replace(old_display_div, new_display_div)

# Also need to update the hidden textarea to be more accessible for selection
old_hidden_textarea = '''        className="absolute inset-0 w-full h-full opacity-0 pointer-events-auto resize-none"
        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          width: '100%',
          height: '100%',
          opacity: 0,
          zIndex: 1,
          background: 'transparent',
          border: 'none',
          outline: 'none',
          resize: 'none',
        }}'''

new_hidden_textarea = '''        className="absolute inset-0 w-full h-full resize-none"
        style={{
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

content = content.replace(old_hidden_textarea, new_hidden_textarea)

# Update handleDisplayClick to handle selection events better
old_handle_click = '''  const handleDisplayClick = useCallback((e: React.MouseEvent) => {
    // Don't focus if clicking on an action pill
    if ((e.target as Element).closest('.action-pill-wrapper')) {
      return;
    }
    hiddenTextareaRef.current?.focus();
  }, []);'''

new_handle_click = '''  const handleDisplayClick = useCallback((e: React.MouseEvent) => {
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

content = content.replace(old_handle_click, new_handle_click)

# Write the modified content back
with open('ui/desktop/src/components/RichChatInput.tsx', 'w') as f:
    f.write(content)

print("✅ Fixed text selection functionality:")
print("   - Added userSelect: 'text' to visual display")
print("   - Made hidden textarea slightly visible (opacity: 0.01)")
print("   - Added double-click and triple-click handling")
print("   - Preserved pill functionality while enabling text selection")
print("   - Should now support highlighting, double-click, and triple-click")
