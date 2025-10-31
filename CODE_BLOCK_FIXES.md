# 🔧 Code Block Width and Height Fixes

## Issues Fixed

### 1. **Code Block Extending Beyond Chat Input**
**Problem**: The code block in live IDE mode was extending beyond the boundaries of the chat input, causing horizontal overflow.

**Root Cause**: 
- The code container was using `inline-flex` display
- No width constraints were applied
- The `SyntaxHighlighter` component was not constrained

**Solution**:
- Changed outer container from `inline` to `block` display with `w-full`
- Added `maxWidth: '100%'` and `boxSizing: 'border-box'` to code container
- Added `overflow-x-auto` for horizontal scrolling when code is too wide
- Applied same constraints to `SyntaxHighlighter` customStyle

### 2. **Text Hidden Behind Toolbar**
**Problem**: When typing in code mode, text could be hidden behind the bottom toolbar.

**Root Cause**: The code block wasn't properly influencing the chat input's height calculation, so the container didn't expand enough.

**Solution**:
- The existing `syncDisplayHeight` function already monitors the display layer
- By making the code block a proper block-level element with correct sizing, it now properly contributes to the `scrollHeight`
- The `containerHeight` state is updated based on the actual content height
- This ensures the parent container allocates enough space

## Changes Made

### RichChatInput.tsx - Code Mode Rendering

```typescript
return (
  <div className="whitespace-pre-wrap relative w-full">  // Added w-full
    {/* Text before trigger */}
    {textBefore && (
      <span className="inline whitespace-pre-wrap">{textBefore}</span>
    )}
    
    {/* Language badge */}
    <div className="inline-flex items-center gap-1 px-2 py-0.5 mb-1 text-xs font-mono text-gray-300 bg-gray-800 rounded border border-gray-700">
      <Code size={12} />
      <span>{codeMode.language}</span>
    </div>
    
    {/* Code content - NOW PROPERLY CONSTRAINED */}
    <div 
      className="block font-mono text-sm bg-[#1E1E1E]/30 rounded p-2 border border-gray-700/50 mt-1 w-full overflow-x-auto"  // Changed to block, added w-full and overflow-x-auto
      style={{ 
        fontFamily: 'Monaco, Menlo, "Ubuntu Mono", Consolas, source-code-pro, monospace',
        maxWidth: '100%',        // Added
        boxSizing: 'border-box', // Added
      }}
    >
      <SyntaxHighlighter
        language={codeMode.language}
        style={vscDarkPlus}
        customStyle={{
          margin: 0,
          padding: 0,
          background: 'transparent',
          fontSize: '0.875rem',
          lineHeight: '1.5',
          maxWidth: '100%',    // Added
          overflowX: 'auto',   // Added
        }}
        PreTag="div"
        CodeTag="code"
        wrapLines={true}
        showLineNumbers={false}
      >
        {codeContent || ' '}
      </SyntaxHighlighter>
      
      {/* Cursor */}
      {isFocused && (
        <span 
          className="border-l border-text-default inline-block" 
          style={{ 
            animation: "blink 1s step-end infinite", 
            height: "1.3em",
            width: "1px",
            marginLeft: "2px",
            position: "relative"
          }} 
        />
      )}
    </div>
  </div>
);
```

## How It Works Now

1. **Width Constraint**:
   - The code block is now a block-level element that respects the container width
   - `w-full` ensures it takes the full width of the parent
   - `maxWidth: 100%` prevents it from exceeding the container
   - `boxSizing: border-box` ensures padding is included in width calculations

2. **Horizontal Scrolling**:
   - When code lines are too long, `overflow-x-auto` adds a horizontal scrollbar
   - This prevents the code from breaking the layout

3. **Height Calculation**:
   - The code block now properly contributes to the display layer's `scrollHeight`
   - `syncDisplayHeight()` detects the increased height
   - `containerHeight` state is updated
   - The parent container expands accordingly
   - This prevents text from being hidden behind the toolbar

## Testing

To verify the fixes:

1. **Width Test**:
   - Type `#python` in the chat input
   - Paste or type a very long line of code
   - ✅ The code should stay within the chat input boundaries
   - ✅ A horizontal scrollbar should appear if needed

2. **Height Test**:
   - Type `#python` in the chat input
   - Type multiple lines of code (10+ lines)
   - ✅ The chat input should expand to show all lines
   - ✅ No text should be hidden behind the toolbar
   - ✅ The container should scroll if it reaches maxHeight

3. **Inline Test**:
   - Type some text, then `#python`, then code
   - ✅ Text before the trigger should display normally
   - ✅ Code block should be properly constrained

## Related Files

- `ui/desktop/src/components/RichChatInput.tsx` - Main implementation
- `ui/desktop/src/components/ChatInput.tsx` - Parent component that uses RichChatInput

## Commit

```
commit 83eba3f3a79
Fix code block width constraints and ensure proper height calculation
```
