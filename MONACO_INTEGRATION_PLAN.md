# Monaco Editor Integration Plan

## 🎯 Goal
Replace the current syntax highlighting approach in `RichChatInput.tsx` with **Monaco Editor** (via `@monaco-editor/react`) to provide a true IDE experience for code input in the Goose chat interface.

## 📦 Library: @monaco-editor/react

**Repository**: https://github.com/suren-atoyan/monaco-react  
**NPM Package**: `@monaco-editor/react`  
**Current React Version**: 19.2.0 (compatible)

### Key Features
- ✅ Full Monaco Editor (VS Code's editor)
- ✅ No webpack config needed
- ✅ Lazy loading support
- ✅ TypeScript support
- ✅ IntelliSense & autocomplete
- ✅ Multi-cursor editing
- ✅ Find/replace
- ✅ Minimap
- ✅ Syntax highlighting for 100+ languages

---

## 🏗️ Architecture Design

### Current State (react-syntax-highlighter)
```
User types #python
  ↓
Regex detects trigger
  ↓
State: codeMode = { active: true, language: "python", startPos: X }
  ↓
Render: <SyntaxHighlighter> with static highlighting
  ↓
Enter = newline, Cmd+Enter = send
```

**Limitations:**
- No autocomplete
- No IntelliSense
- No multi-cursor
- Limited editing features
- Static highlighting only

### Proposed State (Monaco Editor)
```
User types #python
  ↓
Regex detects trigger
  ↓
State: codeMode = { active: true, language: "python", startPos: X }
  ↓
Render: <Editor> Monaco component (lazy loaded)
  ↓
Full IDE features + Enter = newline, Cmd+Enter = send
```

**Benefits:**
- ✅ Full IDE experience
- ✅ IntelliSense
- ✅ Autocomplete
- ✅ Multi-cursor
- ✅ Find/replace
- ✅ Better performance for large code

---

## 🔧 Implementation Strategy

### Phase 1: Installation & Setup
```bash
cd ui/desktop
npm install @monaco-editor/react
# or yarn add @monaco-editor/react
```

**Dependencies:**
- `@monaco-editor/react`: ^4.6.0 (latest)
- `monaco-editor`: Peer dependency (auto-loaded via CDN)

### Phase 2: Create Monaco Code Input Component

**New File**: `ui/desktop/src/components/MonacoCodeInput.tsx`

```typescript
import React, { useRef, useEffect } from 'react';
import Editor, { OnMount, OnChange } from '@monaco-editor/react';
import * as monaco from 'monaco-editor';

interface MonacoCodeInputProps {
  language: string;
  value: string;
  onChange: (value: string) => void;
  onKeyDown?: (e: monaco.IKeyboardEvent) => void;
  height?: string | number;
  theme?: 'vs-dark' | 'light';
  options?: monaco.editor.IStandaloneEditorConstructionOptions;
}

export const MonacoCodeInput: React.FC<MonacoCodeInputProps> = ({
  language,
  value,
  onChange,
  onKeyDown,
  height = '200px',
  theme = 'vs-dark',
  options = {},
}) => {
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);

  const handleEditorDidMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;
    
    // Focus the editor
    editor.focus();
    
    // Add custom key bindings
    if (onKeyDown) {
      editor.onKeyDown((e) => {
        onKeyDown(e);
      });
    }
    
    // Position cursor at end
    const model = editor.getModel();
    if (model) {
      const lineCount = model.getLineCount();
      const lastLineLength = model.getLineLength(lineCount);
      editor.setPosition({ lineNumber: lineCount, column: lastLineLength + 1 });
    }
  };

  const handleEditorChange: OnChange = (value, ev) => {
    onChange(value || '');
  };

  const defaultOptions: monaco.editor.IStandaloneEditorConstructionOptions = {
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    fontSize: 14,
    fontFamily: 'Monaco, Menlo, "Ubuntu Mono", Consolas, source-code-pro, monospace',
    lineNumbers: 'on',
    renderLineHighlight: 'line',
    automaticLayout: true,
    wordWrap: 'on',
    wrappingStrategy: 'advanced',
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
    ...options,
  };

  return (
    <div className="monaco-code-input-wrapper">
      <Editor
        height={height}
        language={language}
        value={value}
        theme={theme}
        options={defaultOptions}
        onMount={handleEditorDidMount}
        onChange={handleEditorChange}
        loading={
          <div className="flex items-center justify-center h-full">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500"></div>
          </div>
        }
      />
    </div>
  );
};
```

### Phase 3: Integrate into RichChatInput

**Modify**: `ui/desktop/src/components/RichChatInput.tsx`

#### 3.1 Import Monaco Component
```typescript
import { MonacoCodeInput } from './MonacoCodeInput';
```

#### 3.2 Update Code Mode Rendering
Replace the `SyntaxHighlighter` section in `renderContent()`:

```typescript
// OLD CODE (remove):
<SyntaxHighlighter
  language={codeMode.language}
  style={vscDarkPlus}
  customStyle={{...}}
>
  {codeContent}
</SyntaxHighlighter>

// NEW CODE (add):
<MonacoCodeInput
  language={codeMode.language}
  value={codeContent}
  onChange={(newCode) => {
    // Update the value with the new code
    const beforeCode = value.slice(0, codeMode.startPos);
    const afterCode = value.slice(codeMode.startPos + codeContent.length);
    const newValue = beforeCode + newCode + afterCode;
    onChange(newValue, codeMode.startPos + newCode.length);
  }}
  onKeyDown={(e) => {
    // Handle Cmd/Ctrl+Enter to send
    if ((e.metaKey || e.ctrlKey) && e.code === 'Enter') {
      e.preventDefault();
      // Trigger send logic
      const syntheticEvent = new CustomEvent('submit', {
        detail: { value },
      }) as unknown as React.FormEvent;
      onKeyDown?.(syntheticEvent as any);
    }
  }}
  height="auto"
  theme="vs-dark"
  options={{
    lineNumbers: 'on',
    minimap: { enabled: false },
  }}
/>
```

#### 3.3 Handle Height Calculation
Monaco Editor needs explicit height. Options:

**Option A: Fixed Height**
```typescript
<MonacoCodeInput height="200px" />
```

**Option B: Dynamic Height (based on line count)**
```typescript
const calculateHeight = (code: string) => {
  const lines = code.split('\n').length;
  const lineHeight = 21; // Monaco default
  const minHeight = 100;
  const maxHeight = 400;
  return Math.min(Math.max(lines * lineHeight, minHeight), maxHeight);
};

<MonacoCodeInput height={calculateHeight(codeContent)} />
```

**Option C: Auto-grow with max height**
```typescript
<MonacoCodeInput 
  height="auto"
  options={{
    scrollBeyondLastLine: false,
    automaticLayout: true,
    maxHeight: 400,
  }}
/>
```

### Phase 4: Handle Keyboard Shortcuts

Monaco has its own keyboard handling. We need to:

1. **Preserve Enter = newline** (Monaco default)
2. **Add Cmd/Ctrl+Enter = send** (custom binding)
3. **Handle Escape = exit code mode** (custom binding)

```typescript
const handleEditorDidMount: OnMount = (editor, monaco) => {
  // Add Cmd/Ctrl+Enter to send
  editor.addCommand(
    monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
    () => {
      // Trigger send
      onSendMessage(editor.getValue());
    }
  );
  
  // Add Escape to exit code mode
  editor.addCommand(
    monaco.KeyCode.Escape,
    () => {
      // Exit code mode
      onExitCodeMode();
    }
  );
};
```

### Phase 5: Styling Integration

Monaco comes with its own themes. We need to:

1. **Match Goose's dark theme**
2. **Customize colors** if needed
3. **Ensure proper borders/padding**

```typescript
// Custom theme definition
monaco.editor.defineTheme('goose-dark', {
  base: 'vs-dark',
  inherit: true,
  rules: [
    { token: 'comment', foreground: '6A9955' },
    { token: 'keyword', foreground: 'C586C0' },
    { token: 'string', foreground: 'CE9178' },
  ],
  colors: {
    'editor.background': '#1E1E1E',
    'editor.foreground': '#D4D4D4',
    'editor.lineHighlightBackground': '#2A2A2A',
    'editorCursor.foreground': '#FFFFFF',
    'editor.selectionBackground': '#264F78',
  },
});
```

---

## 🎨 UI/UX Considerations

### 1. Language Badge
Keep the existing language badge above Monaco:
```tsx
<div className="inline-flex items-center gap-1 px-2 py-0.5 mb-1">
  <Code size={12} />
  <span>{codeMode.language}</span>
</div>
<MonacoCodeInput ... />
```

### 2. Loading State
Monaco takes ~500ms to load. Show spinner:
```tsx
loading={
  <div className="flex items-center justify-center h-32">
    <Spinner />
  </div>
}
```

### 3. Transition Animation
Smooth transition when entering code mode:
```css
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
```

### 4. Exit Code Mode
Two options:
- **Double Enter** (current) - Keep for consistency
- **Escape key** - Add as alternative
- **Click outside** - Optional

### 5. Mobile Considerations
Monaco works on mobile but may need:
- Touch-friendly controls
- Larger tap targets
- Virtual keyboard handling

---

## 📊 Performance Considerations

### 1. Lazy Loading
Monaco is ~3MB. Use lazy loading:
```typescript
import { lazy, Suspense } from 'react';

const MonacoCodeInput = lazy(() => import('./MonacoCodeInput'));

// In render:
<Suspense fallback={<Spinner />}>
  <MonacoCodeInput ... />
</Suspense>
```

### 2. CDN vs Bundle
Monaco can load from CDN (default) or bundle:
- **CDN**: Smaller bundle, but requires internet
- **Bundle**: Larger bundle, works offline

**Recommendation**: Use CDN (default) for now.

### 3. Memory Management
Monaco creates workers. Ensure cleanup:
```typescript
useEffect(() => {
  return () => {
    // Dispose editor on unmount
    editorRef.current?.dispose();
  };
}, []);
```

---

## 🧪 Testing Plan

### Unit Tests
- [ ] MonacoCodeInput renders correctly
- [ ] onChange handler fires on edit
- [ ] Cmd+Enter triggers send
- [ ] Escape exits code mode
- [ ] Height calculation works

### Integration Tests
- [ ] Code mode activates on #language
- [ ] Monaco loads and displays code
- [ ] Editing updates parent state
- [ ] Sending message includes code
- [ ] Exiting code mode works

### E2E Tests
- [ ] Type #python, see Monaco
- [ ] Write code with autocomplete
- [ ] Use Cmd+Enter to send
- [ ] Verify message sent correctly
- [ ] Test all 30+ languages

### Performance Tests
- [ ] Monaco loads in <1s
- [ ] Editing is smooth (60fps)
- [ ] No memory leaks
- [ ] Works with large code (1000+ lines)

---

## 🚀 Migration Path

### Step 1: Feature Flag
Add feature flag to toggle Monaco:
```typescript
const USE_MONACO = process.env.REACT_APP_USE_MONACO === 'true';

{USE_MONACO ? (
  <MonacoCodeInput ... />
) : (
  <SyntaxHighlighter ... />
)}
```

### Step 2: Beta Testing
- Enable for internal users
- Gather feedback
- Fix issues

### Step 3: Gradual Rollout
- 10% of users
- 50% of users
- 100% of users

### Step 4: Remove Old Code
Once stable, remove SyntaxHighlighter.

---

## 📝 Implementation Checklist

### Phase 1: Setup
- [ ] Install `@monaco-editor/react`
- [ ] Verify React 19 compatibility
- [ ] Test basic Monaco rendering

### Phase 2: Component
- [ ] Create `MonacoCodeInput.tsx`
- [ ] Add props interface
- [ ] Implement editor mounting
- [ ] Add keyboard shortcuts
- [ ] Add custom theme

### Phase 3: Integration
- [ ] Import Monaco into RichChatInput
- [ ] Replace SyntaxHighlighter
- [ ] Update onChange handler
- [ ] Update height calculation
- [ ] Test code mode activation

### Phase 4: Polish
- [ ] Add loading spinner
- [ ] Add transition animation
- [ ] Style language badge
- [ ] Add error boundaries
- [ ] Optimize performance

### Phase 5: Testing
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Manual testing
- [ ] Performance profiling
- [ ] Accessibility testing

### Phase 6: Documentation
- [ ] Update FINAL_SUMMARY.md
- [ ] Add Monaco usage guide
- [ ] Document keyboard shortcuts
- [ ] Add troubleshooting section

---

## 🎯 Expected Outcomes

### Before (SyntaxHighlighter)
- Static syntax highlighting
- No autocomplete
- No IntelliSense
- Basic editing

### After (Monaco Editor)
- ✅ Full IDE experience
- ✅ IntelliSense & autocomplete
- ✅ Multi-cursor editing
- ✅ Find/replace
- ✅ Better performance
- ✅ Professional appearance

---

## 🔮 Future Enhancements

### Phase 2 Features
1. **Language-specific snippets**
2. **Custom autocomplete** (Goose-specific)
3. **Error highlighting** (linting)
4. **Format on paste**
5. **Collaborative editing** (future)

### Advanced Features
1. **Multiple editors** (side-by-side)
2. **Diff view** (compare code)
3. **Git integration** (show changes)
4. **Terminal integration** (run code)

---

## 📚 Resources

- **Monaco Editor Docs**: https://microsoft.github.io/monaco-editor/
- **@monaco-editor/react**: https://github.com/suren-atoyan/monaco-react
- **Monaco Playground**: https://microsoft.github.io/monaco-editor/playground.html
- **VS Code Themes**: https://vscodethemes.com/

---

## 🎉 Success Metrics

- [ ] Monaco loads in <1 second
- [ ] Editing feels native (60fps)
- [ ] Autocomplete works for all languages
- [ ] No regressions in existing features
- [ ] Positive user feedback
- [ ] <5% increase in bundle size (with lazy loading)

---

**Status**: 📋 Planning Phase  
**Branch**: `spence/ideinput`  
**Next Step**: Install dependencies and create MonacoCodeInput component
