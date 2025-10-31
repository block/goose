# Monaco Editor Integration - Quick Start Guide

## 🚀 Getting Started

### Prerequisites
```bash
# Ensure you're on the right branch
git checkout spence/ideinput

# Navigate to the UI directory
cd ui/desktop
```

### Installation
```bash
# Install Monaco Editor React wrapper
npm install @monaco-editor/react

# Or with yarn
yarn add @monaco-editor/react
```

**Package**: `@monaco-editor/react` v4.6.0  
**Peer Dependency**: `monaco-editor` (auto-loaded from CDN)

---

## 📁 File Structure

```
ui/desktop/src/components/
├── RichChatInput.tsx          # Main rich text input (modify)
├── MonacoCodeInput.tsx        # NEW: Monaco wrapper component
├── ChatInput.tsx              # Parent component (no changes)
├── SpellCheckTooltip.tsx      # Existing (no changes)
├── ActionPill.tsx             # Existing (no changes)
└── MentionPill.tsx            # Existing (no changes)
```

---

## 🔧 Step-by-Step Implementation

### Step 1: Create MonacoCodeInput Component

**File**: `ui/desktop/src/components/MonacoCodeInput.tsx`

```typescript
import React, { useRef, useEffect } from 'react';
import Editor, { OnMount, OnChange } from '@monaco-editor/react';
import type * as monaco from 'monaco-editor';

interface MonacoCodeInputProps {
  language: string;
  value: string;
  onChange: (value: string) => void;
  onSend?: () => void;
  onExit?: () => void;
  height?: string | number;
  theme?: 'vs-dark' | 'light';
}

export const MonacoCodeInput: React.FC<MonacoCodeInputProps> = ({
  language,
  value,
  onChange,
  onSend,
  onExit,
  height = 'auto',
  theme = 'vs-dark',
}) => {
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);

  const handleEditorDidMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;
    editor.focus();

    // Cmd/Ctrl+Enter to send
    editor.addCommand(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
      () => onSend?.()
    );

    // Escape to exit
    editor.addCommand(
      monaco.KeyCode.Escape,
      () => onExit?.()
    );

    // Position cursor at end
    const model = editor.getModel();
    if (model) {
      const lineCount = model.getLineCount();
      const lastLineLength = model.getLineLength(lineCount);
      editor.setPosition({ lineNumber: lineCount, column: lastLineLength + 1 });
    }
  };

  const handleEditorChange: OnChange = (value) => {
    onChange(value || '');
  };

  // Calculate height based on line count
  const calculatedHeight = React.useMemo(() => {
    if (typeof height === 'number' || height !== 'auto') return height;
    
    const lines = value.split('\n').length;
    const lineHeight = 21;
    const padding = 16;
    const minHeight = 100;
    const maxHeight = 400;
    
    return Math.min(Math.max(lines * lineHeight + padding, minHeight), maxHeight);
  }, [value, height]);

  return (
    <div className="monaco-code-input-wrapper">
      <Editor
        height={calculatedHeight}
        language={language}
        value={value}
        theme={theme}
        options={{
          minimap: { enabled: false },
          scrollBeyondLastLine: false,
          fontSize: 14,
          fontFamily: 'Monaco, Menlo, "Ubuntu Mono", Consolas, monospace',
          lineNumbers: 'on',
          renderLineHighlight: 'line',
          automaticLayout: true,
          wordWrap: 'on',
          padding: { top: 8, bottom: 8 },
          suggest: {
            showKeywords: true,
            showSnippets: true,
          },
          quickSuggestions: {
            other: true,
            comments: false,
            strings: false,
          },
        }}
        onMount={handleEditorDidMount}
        onChange={handleEditorChange}
        loading={
          <div className="flex items-center justify-center h-32">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500" />
          </div>
        }
      />
    </div>
  );
};
```

### Step 2: Integrate into RichChatInput

**File**: `ui/desktop/src/components/RichChatInput.tsx`

#### 2.1 Add Import
```typescript
import { MonacoCodeInput } from './MonacoCodeInput';
import { lazy, Suspense } from 'react';

// Lazy load Monaco for better performance
const MonacoCodeInputLazy = lazy(() => import('./MonacoCodeInput').then(m => ({ default: m.MonacoCodeInput })));
```

#### 2.2 Update renderContent() - Replace SyntaxHighlighter
Find this section (around line 600):
```typescript
// OLD CODE - REMOVE THIS:
<SyntaxHighlighter
  language={codeMode.language}
  style={vscDarkPlus}
  customStyle={{...}}
>
  {codeContent}
</SyntaxHighlighter>
```

Replace with:
```typescript
// NEW CODE - ADD THIS:
<Suspense fallback={
  <div className="flex items-center justify-center h-32">
    <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500" />
  </div>
}>
  <MonacoCodeInputLazy
    language={codeMode.language}
    value={codeContent}
    onChange={(newCode) => {
      const beforeCode = value.slice(0, codeMode.startPos);
      const afterCode = value.slice(codeMode.startPos + codeContent.length);
      const newValue = beforeCode + newCode + afterCode;
      onChange(newValue, codeMode.startPos + newCode.length);
    }}
    onSend={() => {
      // Trigger send via parent's onKeyDown
      const syntheticEvent = new CustomEvent('submit', {
        detail: { value },
      }) as unknown as React.FormEvent;
      onKeyDown?.(syntheticEvent as any);
    }}
    onExit={() => {
      // Exit code mode
      setCodeMode(null);
    }}
    height="auto"
    theme="vs-dark"
  />
</Suspense>
```

### Step 3: Add Styling

**File**: `ui/desktop/src/styles/main.css` (or component CSS)

```css
/* Monaco Code Input Wrapper */
.monaco-code-input-wrapper {
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  overflow: hidden;
  background: #1E1E1E;
  margin-top: 8px;
}

/* Smooth transition when entering code mode */
.monaco-code-input-wrapper {
  animation: slideDown 0.2s ease-out;
}

@keyframes slideDown {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* Loading spinner */
.monaco-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100px;
}
```

---

## 🧪 Testing

### Manual Testing
1. Start the app: `npm run dev`
2. Open chat input
3. Type: `#python `
4. Verify Monaco loads
5. Type code and test autocomplete
6. Press `Cmd+Enter` to send
7. Press `Escape` to exit

### Test Cases
```typescript
// Test 1: Code mode activation
// Type: #python
// Expected: Monaco editor appears

// Test 2: Autocomplete
// Type: def hel
// Expected: Autocomplete shows "hello"

// Test 3: Multi-cursor
// Hold Cmd/Ctrl + click multiple locations
// Expected: Multiple cursors appear

// Test 4: Send message
// Type code + press Cmd+Enter
// Expected: Message sent with code

// Test 5: Exit code mode
// Press Escape
// Expected: Back to normal input
```

---

## 🎯 Quick Reference

### Keyboard Shortcuts
| Shortcut | Action |
|----------|--------|
| `Enter` | New line (in code mode) |
| `Cmd/Ctrl+Enter` | Send message |
| `Escape` | Exit code mode |
| `Cmd/Ctrl+F` | Find (Monaco built-in) |
| `Cmd/Ctrl+D` | Multi-cursor (Monaco built-in) |

### Supported Languages
JavaScript, TypeScript, Python, Java, C++, C, Go, Rust, Ruby, PHP, Swift, Kotlin, Scala, HTML, CSS, JSON, YAML, SQL, Bash, Shell, PowerShell, R, MATLAB, Lua, Perl, Haskell, Elixir, Clojure, Dart, JSX, TSX

### Props - MonacoCodeInput
```typescript
interface MonacoCodeInputProps {
  language: string;        // Language for syntax highlighting
  value: string;          // Code content
  onChange: (value: string) => void;  // Called on edit
  onSend?: () => void;    // Called on Cmd+Enter
  onExit?: () => void;    // Called on Escape
  height?: string | number;  // Editor height
  theme?: 'vs-dark' | 'light';  // Color theme
}
```

---

## 🐛 Troubleshooting

### Monaco doesn't load
**Problem**: Editor shows loading spinner indefinitely  
**Solution**: Check network tab for CDN errors, ensure internet connection

### Autocomplete not working
**Problem**: No suggestions appear  
**Solution**: Verify language is supported, check Monaco language services loaded

### Height not adjusting
**Problem**: Editor too small or too large  
**Solution**: Check `calculatedHeight` logic, verify min/max constraints

### Keyboard shortcuts not working
**Problem**: Cmd+Enter doesn't send  
**Solution**: Check `onSend` prop is passed, verify `addCommand` in `onMount`

### Memory leak
**Problem**: Memory grows over time  
**Solution**: Ensure editor is disposed in cleanup: `editorRef.current?.dispose()`

---

## 📊 Performance Tips

### 1. Lazy Loading
```typescript
// Always use lazy loading for Monaco
const MonacoCodeInput = lazy(() => import('./MonacoCodeInput'));
```

### 2. Preloading (Optional)
```typescript
// Preload Monaco when user types #
if (value.endsWith('#')) {
  import('./MonacoCodeInput');
}
```

### 3. Debounce onChange
```typescript
// Debounce onChange for better performance
const debouncedOnChange = useMemo(
  () => debounce(onChange, 100),
  [onChange]
);
```

### 4. Cleanup
```typescript
// Always cleanup on unmount
useEffect(() => {
  return () => {
    editorRef.current?.dispose();
  };
}, []);
```

---

## 🎨 Customization

### Custom Theme
```typescript
// Define custom theme
monaco.editor.defineTheme('goose-dark', {
  base: 'vs-dark',
  inherit: true,
  rules: [
    { token: 'comment', foreground: '6A9955' },
    { token: 'keyword', foreground: 'C586C0' },
  ],
  colors: {
    'editor.background': '#1E1E1E',
  },
});

// Use custom theme
<MonacoCodeInput theme="goose-dark" />
```

### Custom Options
```typescript
<Editor
  options={{
    fontSize: 16,
    lineNumbers: 'off',
    minimap: { enabled: true },
    // ... more options
  }}
/>
```

---

## 📚 Resources

- **Monaco Editor API**: https://microsoft.github.io/monaco-editor/api/index.html
- **@monaco-editor/react**: https://github.com/suren-atoyan/monaco-react
- **Monaco Playground**: https://microsoft.github.io/monaco-editor/playground.html
- **Language Support**: https://github.com/microsoft/monaco-editor/tree/main/src/basic-languages

---

## ✅ Checklist

### Before Starting
- [ ] Read MONACO_INTEGRATION_PLAN.md
- [ ] Review MONACO_ARCHITECTURE.md
- [ ] Understand current RichChatInput.tsx

### Implementation
- [ ] Install @monaco-editor/react
- [ ] Create MonacoCodeInput.tsx
- [ ] Update RichChatInput.tsx
- [ ] Add CSS styling
- [ ] Test basic functionality

### Testing
- [ ] Manual testing (all languages)
- [ ] Keyboard shortcuts work
- [ ] Performance is acceptable
- [ ] No memory leaks
- [ ] Accessibility verified

### Deployment
- [ ] Create feature flag
- [ ] Beta test with team
- [ ] Monitor performance
- [ ] Gather feedback
- [ ] Full rollout

---

**Status**: Ready to implement!  
**Estimated Time**: 2-3 hours for basic integration  
**Branch**: `spence/ideinput`

Good luck! 🚀
