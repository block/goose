# 🔧 Code Mode Height Calculation Fix

## Problem

When entering code mode with `#python` (or other languages), the IDE code block container wasn't properly passing its height to the chat input component. This caused two issues:

1. **Initial load**: The IDE space was "below the fold" - not visible without scrolling
2. **Height not updating**: The container didn't expand to show the full code block

## Root Cause

The height synchronization logic (`syncDisplayHeight`) was measuring the **textarea's** `scrollHeight`, which only accounts for the raw text content. However, in code mode:

- The **display layer** renders a styled code block with:
  - Padding (`p-2`)
  - Borders (`border border-gray-700/50`)
  - Margins (`mt-1`, `mb-1`)
  - Language badge
  - Syntax highlighting container

The textarea has no knowledge of these visual elements, so its `scrollHeight` was much smaller than the actual rendered height of the display layer.

## Solution

Added a dedicated `useEffect` that:

1. **Triggers when `codeMode` changes** - detects when entering/exiting code mode
2. **Measures the display layer's `scrollHeight`** - gets the actual rendered height including all styling
3. **Uses a 50ms delay** - ensures `SyntaxHighlighter` has fully rendered before measuring
4. **Updates both layers and container** - sets textarea, display, and containerHeight to match

### Implementation

```typescript
// Sync height when code mode changes or when in code mode (for initial render and updates)
useEffect(() => {
  if (codeMode) {
    // When code mode is active, we need to measure the actual display height
    // because the textarea doesn't know about the styled code block
    console.log('💻 CODE MODE: Triggering height sync for code mode');
    
    // Use a small delay to ensure the SyntaxHighlighter has rendered
    const timer = setTimeout(() => {
      if (displayRef.current && hiddenTextareaRef.current) {
        const display = displayRef.current;
        const textarea = hiddenTextareaRef.current;
        
        // Get the actual rendered height of the display content
        const displayScrollHeight = display.scrollHeight;
        
        console.log('💻 CODE MODE: Display scrollHeight:', displayScrollHeight);
        
        // Calculate line height
        const computedStyle = window.getComputedStyle(textarea);
        const fontSize = parseFloat(computedStyle.fontSize);
        const lineHeightValue = computedStyle.lineHeight;
        const lineHeight = Math.round(lineHeightValue === "normal" ? fontSize * 1.2 : parseFloat(lineHeightValue));
        const minHeight = rows * lineHeight;
        const maxHeight = style?.maxHeight ? parseInt(style.maxHeight.toString()) : 300;
        
        // Use the display's scroll height instead of textarea's
        const desiredHeight = Math.min(displayScrollHeight, maxHeight);
        const finalHeight = Math.max(desiredHeight, minHeight);
        
        console.log('💻 CODE MODE: Setting height to', finalHeight);
        
        // Update both layers
        textarea.style.height = `${finalHeight}px`;
        display.style.height = `${finalHeight}px`;
        setContainerHeight(finalHeight);
      }
    }, 50); // Small delay to let SyntaxHighlighter render
    
    return () => clearTimeout(timer);
  }
}, [codeMode, value, rows, style]);
```

## How It Works

### Before the Fix

1. User types `#python`
2. Code mode activates
3. Display layer renders styled code block (100px tall)
4. `syncDisplayHeight` measures textarea scrollHeight (30px - just the raw text)
5. Container height set to 30px
6. Code block extends beyond container (70px hidden below the fold)

### After the Fix

1. User types `#python`
2. Code mode activates
3. Display layer renders styled code block (100px tall)
4. New `useEffect` triggers after 50ms
5. Measures display layer's scrollHeight (100px - includes all styling)
6. Container height set to 100px
7. Code block fully visible ✅

## Dependencies

The `useEffect` depends on:
- `codeMode` - triggers when entering/exiting code mode
- `value` - re-measures when code content changes
- `rows` - for minHeight calculation
- `style` - for maxHeight constraint

## Testing

To verify the fix:

1. **Initial Load Test**:
   - Type `#python` in the chat input
   - ✅ The code block should immediately be fully visible
   - ✅ No scrolling required to see the IDE container

2. **Height Update Test**:
   - Type `#python` and add multiple lines of code
   - ✅ The container should expand as you type
   - ✅ All code should remain visible

3. **Exit Code Mode Test**:
   - Delete the `#python` trigger
   - ✅ The container should shrink back to normal text height

## Related Files

- `ui/desktop/src/components/RichChatInput.tsx` - Main implementation

## Commits

```
commit 90b384c1c9f
Fix code mode height calculation to use display layer scrollHeight

commit 83eba3f3a79
Fix code block width constraints and ensure proper height calculation
```

## Notes

- The 50ms delay is necessary because `SyntaxHighlighter` renders asynchronously
- The effect cleans up the timer to prevent memory leaks
- This works alongside the existing `syncDisplayHeight` for normal text
- The display layer's scrollHeight is the source of truth in code mode
