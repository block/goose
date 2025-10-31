# Monaco Editor Architecture for Goose

## 🏗️ Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        ChatInput                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                  RichChatInput                        │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │         Hidden Textarea Layer                   │  │  │
│  │  │  (Handles: Selection, IME, Native Events)       │  │  │
│  │  │  z-index: 2                                     │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │         Display Layer                           │  │  │
│  │  │  ┌──────────────────────────────────────────┐   │  │  │
│  │  │  │  Normal Text with Pills                  │   │  │  │
│  │  │  │  - Action Pills: [Command]               │   │  │  │
│  │  │  │  - Mention Pills: @file                  │   │  │  │
│  │  │  │  - Spell Check Highlights                │   │  │  │
│  │  │  └──────────────────────────────────────────┘   │  │  │
│  │  │  ┌──────────────────────────────────────────┐   │  │  │
│  │  │  │  Code Mode (when #language detected)     │   │  │  │
│  │  │  │  ┌────────────────────────────────────┐  │   │  │  │
│  │  │  │  │  Language Badge: [🐍 python]       │  │   │  │  │
│  │  │  │  └────────────────────────────────────┘  │   │  │  │
│  │  │  │  ┌────────────────────────────────────┐  │   │  │  │
│  │  │  │  │     MonacoCodeInput Component      │  │   │  │  │
│  │  │  │  │  ┌──────────────────────────────┐  │  │   │  │  │
│  │  │  │  │  │   Monaco Editor Instance     │  │  │   │  │  │
│  │  │  │  │  │   - Syntax Highlighting      │  │  │   │  │  │
│  │  │  │  │  │   - IntelliSense             │  │  │   │  │  │
│  │  │  │  │  │   - Autocomplete             │  │  │   │  │  │
│  │  │  │  │  │   - Multi-cursor             │  │  │   │  │  │
│  │  │  │  │  │   - Find/Replace             │  │  │   │  │  │
│  │  │  │  │  └──────────────────────────────┘  │  │   │  │  │
│  │  │  │  └────────────────────────────────────┘  │   │  │  │
│  │  │  └──────────────────────────────────────────┘   │  │  │
│  │  │  z-index: 3                                      │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔄 State Flow

### Normal Text Input
```
User types text
  ↓
Hidden Textarea captures input
  ↓
onChange(value, cursorPos)
  ↓
RichChatInput state updates
  ↓
Display Layer re-renders
  ↓
Shows: Text + Pills + Spell Check
```

### Code Mode Activation
```
User types: "#python "
  ↓
Regex detects: /#(python|javascript|...)/
  ↓
setCodeMode({ active: true, language: "python", startPos: X })
  ↓
Display Layer switches to Code Mode
  ↓
MonacoCodeInput lazy loads
  ↓
Monaco Editor renders with Python syntax
  ↓
User sees: [🐍 python] + IDE editor
```

### Code Editing
```
User types in Monaco
  ↓
Monaco onChange fires
  ↓
MonacoCodeInput.onChange(newCode)
  ↓
RichChatInput updates value:
  beforeCode + newCode + afterCode
  ↓
Parent ChatInput receives update
  ↓
Draft saved (debounced)
```

### Code Mode Exit
```
User presses Escape (or deletes #python)
  ↓
setCodeMode(null)
  ↓
Display Layer switches back to Normal
  ↓
Monaco Editor unmounts
  ↓
Memory cleaned up
```

---

## 📦 Component Hierarchy

```
ChatInput.tsx
  └── RichChatInput.tsx
      ├── Hidden Textarea (native input)
      ├── Display Layer
      │   ├── Normal Mode
      │   │   ├── Text rendering
      │   │   ├── ActionPill components
      │   │   ├── MentionPill components
      │   │   └── SpellCheckTooltip
      │   └── Code Mode
      │       ├── Language Badge
      │       └── MonacoCodeInput.tsx
      │           └── Editor (from @monaco-editor/react)
      │               └── Monaco Editor Instance
      │                   ├── Syntax Highlighter
      │                   ├── IntelliSense Engine
      │                   ├── Autocomplete Provider
      │                   └── Language Services
      └── Spell Check System
          ├── Electron Spell Check API
          └── SpellCheckTooltip.tsx
```

---

## 🔌 Data Flow

### Props Flow (Top-Down)
```
ChatInput
  ↓ value, onChange, onKeyDown
RichChatInput
  ↓ language, value, onChange, onKeyDown
MonacoCodeInput
  ↓ options, theme
Monaco Editor
```

### Events Flow (Bottom-Up)
```
Monaco Editor
  ↑ onChange, onKeyDown
MonacoCodeInput
  ↑ onChange (with full value)
RichChatInput
  ↑ onChange (with cursor position)
ChatInput
  ↑ handleSubmit, draft save
```

---

## 🎯 Key Integration Points

### 1. Code Mode Detection
```typescript
// In RichChatInput.tsx
useEffect(() => {
  const languageTriggerRegex = /#(javascript|typescript|python|...)/i;
  const match = value.match(languageTriggerRegex);
  
  if (match && !codeMode) {
    setCodeMode({
      active: true,
      language: match[1].toLowerCase(),
      startPos: match.index + match[0].length + 1
    });
  }
}, [value, codeMode]);
```

### 2. Monaco Integration
```typescript
// In renderContent()
if (codeMode && codeMode.active) {
  return (
    <div>
      {/* Language Badge */}
      <LanguageBadge language={codeMode.language} />
      
      {/* Monaco Editor */}
      <Suspense fallback={<LoadingSpinner />}>
        <MonacoCodeInput
          language={codeMode.language}
          value={codeContent}
          onChange={(newCode) => {
            const beforeCode = value.slice(0, codeMode.startPos);
            const afterCode = value.slice(codeMode.startPos + codeContent.length);
            onChange(beforeCode + newCode + afterCode);
          }}
          onKeyDown={handleMonacoKeyDown}
        />
      </Suspense>
    </div>
  );
}
```

### 3. Keyboard Handling
```typescript
// In MonacoCodeInput.tsx
const handleEditorDidMount: OnMount = (editor, monaco) => {
  // Cmd/Ctrl+Enter to send
  editor.addCommand(
    monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter,
    () => {
      onSendMessage(editor.getValue());
    }
  );
  
  // Escape to exit code mode
  editor.addCommand(
    monaco.KeyCode.Escape,
    () => {
      onExitCodeMode();
    }
  );
};
```

### 4. Height Synchronization
```typescript
// Dynamic height based on content
const calculateHeight = (code: string) => {
  const lines = code.split('\n').length;
  const lineHeight = 21;
  const padding = 16;
  const minHeight = 100;
  const maxHeight = 400;
  
  const contentHeight = (lines * lineHeight) + padding;
  return Math.min(Math.max(contentHeight, minHeight), maxHeight);
};

<MonacoCodeInput height={calculateHeight(codeContent)} />
```

---

## 🔐 State Management

### RichChatInput State
```typescript
interface RichChatInputState {
  // Text state
  value: string;
  displayValue: string;
  cursorPosition: number;
  
  // Code mode state
  codeMode: {
    active: boolean;
    language: string;
    startPos: number;
  } | null;
  
  // UI state
  isFocused: boolean;
  containerHeight: number;
  
  // Spell check state
  misspelledWords: MisspelledWord[];
  tooltip: TooltipState;
}
```

### MonacoCodeInput State
```typescript
interface MonacoCodeInputState {
  // Editor instance
  editorRef: React.RefObject<monaco.editor.IStandaloneCodeEditor>;
  
  // Loading state
  isLoading: boolean;
  
  // Monaco instance
  monacoInstance: typeof monaco | null;
}
```

---

## 🚀 Lazy Loading Strategy

### Monaco Lazy Load
```typescript
// MonacoCodeInput.tsx
import { lazy, Suspense } from 'react';

const Editor = lazy(() => import('@monaco-editor/react'));

export const MonacoCodeInput = (props) => {
  return (
    <Suspense fallback={<LoadingSpinner />}>
      <Editor {...props} />
    </Suspense>
  );
};
```

### Component Lazy Load
```typescript
// RichChatInput.tsx
import { lazy, Suspense } from 'react';

const MonacoCodeInput = lazy(() => import('./MonacoCodeInput'));

// In render:
{codeMode && (
  <Suspense fallback={<LoadingSpinner />}>
    <MonacoCodeInput ... />
  </Suspense>
)}
```

### Preload on Hover (Optional)
```typescript
// Preload Monaco when user hovers over #
const handleInputChange = (value: string) => {
  if (value.endsWith('#')) {
    // Preload Monaco in background
    import('./MonacoCodeInput');
  }
};
```

---

## 🎨 Styling Architecture

### Theme System
```typescript
// Custom Goose theme for Monaco
monaco.editor.defineTheme('goose-dark', {
  base: 'vs-dark',
  inherit: true,
  rules: [
    { token: 'comment', foreground: '6A9955' },
    { token: 'keyword', foreground: 'C586C0' },
    { token: 'string', foreground: 'CE9178' },
    { token: 'number', foreground: 'B5CEA8' },
    { token: 'function', foreground: 'DCDCAA' },
  ],
  colors: {
    'editor.background': '#1E1E1E',
    'editor.foreground': '#D4D4D4',
    'editor.lineHighlightBackground': '#2A2A2A',
    'editorCursor.foreground': '#FFFFFF',
    'editor.selectionBackground': '#264F78',
    'editorLineNumber.foreground': '#858585',
  },
});
```

### CSS Integration
```css
/* MonacoCodeInput wrapper */
.monaco-code-input-wrapper {
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  overflow: hidden;
  background: #1E1E1E;
}

/* Language badge */
.language-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  font-size: 12px;
  font-weight: 500;
  margin-bottom: 8px;
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

## 🔄 Lifecycle

### Component Mount
```
1. RichChatInput mounts
2. Hidden textarea renders
3. Display layer renders (normal mode)
4. User types "#python"
5. Code mode activates
6. MonacoCodeInput lazy loads
7. Monaco Editor initializes
8. Language services load
9. Editor ready for input
```

### Component Update
```
1. User types in Monaco
2. Monaco onChange fires
3. MonacoCodeInput updates parent
4. RichChatInput state updates
5. Value propagates to ChatInput
6. Draft saved (debounced)
```

### Component Unmount
```
1. User exits code mode (Escape)
2. setCodeMode(null)
3. MonacoCodeInput unmounts
4. Monaco Editor disposed
5. Language services cleaned up
6. Memory freed
7. Back to normal mode
```

---

## 🧪 Testing Architecture

### Unit Tests
```typescript
// MonacoCodeInput.test.tsx
describe('MonacoCodeInput', () => {
  it('renders Monaco Editor', () => {});
  it('handles onChange events', () => {});
  it('handles keyboard shortcuts', () => {});
  it('calculates height correctly', () => {});
  it('cleans up on unmount', () => {});
});
```

### Integration Tests
```typescript
// RichChatInput.test.tsx
describe('RichChatInput with Monaco', () => {
  it('activates code mode on #language', () => {});
  it('loads Monaco Editor', () => {});
  it('syncs value between Monaco and parent', () => {});
  it('exits code mode on Escape', () => {});
  it('sends message on Cmd+Enter', () => {});
});
```

### E2E Tests
```typescript
// chat-input.e2e.ts
describe('Chat Input with Monaco', () => {
  it('should enter code mode', async () => {
    await page.type('#chat-input', '#python ');
    await page.waitForSelector('.monaco-editor');
    expect(await page.$('.monaco-editor')).toBeTruthy();
  });
  
  it('should provide autocomplete', async () => {
    await page.type('.monaco-editor', 'def hel');
    await page.waitForSelector('.monaco-suggest-widget');
    expect(await page.$('.monaco-suggest-widget')).toBeTruthy();
  });
});
```

---

## 📊 Performance Monitoring

### Key Metrics
```typescript
// Performance tracking
const performanceMetrics = {
  monacoLoadTime: 0,      // Time to load Monaco
  editorMountTime: 0,     // Time to mount editor
  firstEditTime: 0,       // Time to first edit
  autocompleteTime: 0,    // Time to show autocomplete
  memoryUsage: 0,         // Memory used by Monaco
};

// Track Monaco load
const startTime = performance.now();
import('@monaco-editor/react').then(() => {
  performanceMetrics.monacoLoadTime = performance.now() - startTime;
});
```

### Memory Management
```typescript
// Cleanup on unmount
useEffect(() => {
  return () => {
    // Dispose editor
    editorRef.current?.dispose();
    
    // Clear Monaco models
    monaco.editor.getModels().forEach(model => model.dispose());
    
    // Clear workers
    monaco.editor.getEditors().forEach(editor => editor.dispose());
  };
}, []);
```

---

## 🎯 Success Criteria

### Performance
- [ ] Monaco loads in <1 second
- [ ] Editor mounts in <200ms
- [ ] Typing feels instant (60fps)
- [ ] Autocomplete appears in <100ms
- [ ] Memory usage <50MB

### Functionality
- [ ] All 30+ languages work
- [ ] Autocomplete works
- [ ] IntelliSense works
- [ ] Keyboard shortcuts work
- [ ] Height adjusts correctly

### User Experience
- [ ] Smooth transition to code mode
- [ ] Professional appearance
- [ ] No lag or jank
- [ ] Clear visual feedback
- [ ] Intuitive keyboard shortcuts

---

**Status**: 📋 Architecture Defined  
**Next**: Implement MonacoCodeInput component  
**Branch**: `spence/ideinput`
