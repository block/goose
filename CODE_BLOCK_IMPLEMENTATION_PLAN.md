# IDE-Style Code Block Rendering Implementation Plan

## Goal
Add IDE-style syntax highlighting for code blocks pasted into the chat input.

## Approach

### 1. Detection Pattern
- Match triple backtick code fences: ` ```language\ncode\n``` `
- Regex: `/```(\w+)?\n([\s\S]*?)```/g`
- Extract language (optional) and code content

### 2. Integration Points
- Add to `allMatches` array in `renderContent()` alongside actions, mentions, misspelled words
- Type: `'codeblock'`
- Store: `{ language, code, index, length }`

### 3. Rendering
- Use `react-syntax-highlighter` (already installed)
- Theme: `vscDarkPlus` (VS Code dark theme)
- Wrapper: Styled `<div>` with IDE-like appearance
  - Dark background
  - Rounded corners
  - Padding
  - Optional language label
  - Monospace font

### 4. Styling Considerations
- Disable spell check within code blocks
- Maintain cursor positioning
- Handle height synchronization
- Ensure code blocks don't break layout

### 5. Features
- **Syntax highlighting** based on language
- **Language badge** in top-right corner
- **Copy button** (future enhancement)
- **Line numbers** (optional, future enhancement)

## Implementation Steps

1. ✅ Import `SyntaxHighlighter` and theme
2. ⏳ Add code block regex to `renderContent()`
3. ⏳ Extract code blocks and add to `allMatches`
4. ⏳ Create code block rendering component
5. ⏳ Handle cursor positioning around code blocks
6. ⏳ Test with various languages (js, py, tsx, etc.)
7. ⏳ Ensure height sync works with code blocks

## Example Input
\`\`\`typescript
function hello(name: string) {
  console.log(\`Hello, \${name}!\`);
}
\`\`\`

## Expected Output
A beautifully syntax-highlighted code block with:
- VS Code dark theme colors
- Proper indentation
- Language-specific highlighting
- Clean, IDE-like appearance

