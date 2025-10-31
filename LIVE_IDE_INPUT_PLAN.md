# Live IDE Input Implementation Plan

## Goal
Type `#python` (or `#javascript`, `#typescript`, etc.) in the chat input to activate a live IDE-style code editor with syntax highlighting as you type.

## User Experience

1. User types `#python` in chat input
2. The `#python` text transforms into a language badge
3. Everything typed after becomes syntax-highlighted code
4. User can continue typing with live syntax highlighting
5. Press Enter to insert newlines (not send message)
6. Press Cmd+Enter (or Ctrl+Enter) to finish and send

## Implementation Strategy

### 1. Detect Language Triggers
- Pattern: `#(javascript|typescript|python|java|cpp|go|rust|html|css|json|yaml|sql|bash|...)`
- When detected, enter "code mode"
- Store: `codeMode: { active: boolean, language: string, startPos: number }`

### 2. Render in Code Mode
- Show language badge at the start
- Apply syntax highlighting to all text after the trigger
- Use inline `SyntaxHighlighter` or custom highlighting
- Maintain cursor position

### 3. Keyboard Handling
- Enter: Insert newline (don't send)
- Cmd/Ctrl+Enter: Exit code mode and send message
- Backspace at start: Exit code mode

### 4. Visual Design
- Subtle background color change to indicate code mode
- Language badge with icon
- Monospace font
- Syntax colors

## Technical Approach

### Option A: Transform on Detection (Simpler)
- Detect `#language` pattern
- Replace with language badge component
- Apply syntax highlighting to remaining text
- Simpler but less flexible

### Option B: Modal Code Editor (More Complex)
- Detect `#language` trigger
- Open inline code editor component
- Full IDE experience
- More complex but better UX

## Recommendation: Option A (Transform)
Start with Option A for quick implementation, can enhance later.

## Implementation Steps

1. Add language trigger detection to `renderContent()`
2. Create `CodeModeBadge` component
3. Apply syntax highlighting to text after badge
4. Modify keyboard handlers for Enter behavior
5. Add visual indicators for code mode
6. Test with various languages

