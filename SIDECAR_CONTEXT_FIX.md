# Sidecar Context Injection Fix

## Problem

The sidecar context was **not being injected** into AI messages because the **EnhancedBentoBox** component (which is actually being used by the dock system) was **not registering sidecars** with the `UnifiedSidecarContext`.

## Root Cause Analysis

### **CRITICAL DISCOVERY: Two Message Streaming Systems**

The app uses **TWO DIFFERENT** message streaming implementations:

1. **`useMessageStream`** (in `src/hooks/useMessageStream.ts`)
   - ✅ **HAS** context injection via `injectSidecarContext()`
   - ❌ **NOT USED** by BaseChat2 (the main chat component)

2. **`useChatStream`** (in `src/hooks/useChatStream.ts`) 
   - ❌ **MISSING** context injection
   - ✅ **USED BY** BaseChat2 (the main chat component)

**This is why context wasn't being injected!** BaseChat2 uses `useChatStream` which makes direct `fetch` calls and bypasses the `useMessageStream` hook entirely.

### Architecture Overview

The system has two sidecar implementations:

1. **SidecarLayout** (legacy) - Used for old-style sidecars
   - ✅ **WAS** registering with UnifiedSidecarContext
   - ❌ **NOT** being used by the dock system

2. **EnhancedBentoBox** (current) - Used by the dock above chat input
   - ❌ **WAS NOT** registering with UnifiedSidecarContext
   - ✅ **IS** being used by the dock system

### Data Flow (Before Fix)

```
User clicks dock icon (SidecarInvoker)
    ↓
Dispatches 'add-container' event
    ↓
MainPanelLayout creates container
    ↓
EnhancedBentoBox renders container
    ↓
❌ NO REGISTRATION with UnifiedSidecarContext
    ↓
useMessageStream.injectSidecarContext() finds 0 active sidecars
    ↓
Context NOT injected into user message
```

### Code Evidence

**EnhancedBentoBox.tsx** (before fix):
```bash
$ grep -n "registerSidecar\|unifiedSidecarContext" src/components/Layout/EnhancedBentoBox.tsx
# NO RESULTS - No integration with UnifiedSidecarContext!
```

**SidecarLayout.tsx** (working but unused):
```typescript
// Lines 550-650: Has full registration logic
useEffect(() => {
  if (!unifiedSidecarContext) return;
  
  activeViews.forEach(viewId => {
    const view = views.find(v => v.id === viewId);
    if (view) {
      unifiedSidecarContext.registerSidecar(sidecarInfo);
    }
  });
}, [unifiedSidecarContext, activeViews, views]);
```

## Solution

Added `UnifiedSidecarContext` integration to **EnhancedBentoBox** for components that don't self-register:

**Important Discovery**: The **WebViewer component** already handles its own registration with real-time navigation updates (URL changes, page titles, loading state, etc.). We only need to register components that don't self-register.

### Changes Made

#### **1. Fixed EnhancedBentoBox Registration** (`src/components/Layout/EnhancedBentoBox.tsx`)

Added context registration for non-self-registering components:

1. **Import the context hook**:
```typescript
import { useUnifiedSidecarContextOptional } from '../../contexts/UnifiedSidecarContext';
```

2. **Get the context in the component**:
```typescript
const unifiedSidecarContext = useUnifiedSidecarContextOptional();
```

3. **Register containers with context** (useEffect hook):
```typescript
useEffect(() => {
  if (!unifiedSidecarContext) return;

  // Register all containers
  containers.forEach(container => {
    let sidecarInfo;
    
    switch (container.contentType) {
      case 'web-viewer':
        sidecarInfo = {
          id: container.id,
          type: 'web-viewer' as const,
          title: container.title || 'Web Browser',
          url: container.contentProps?.initialUrl || 'https://google.com',
          domain: urlObj.hostname,
          isSecure: urlObj.protocol === 'https:',
          // ... other properties
        };
        break;
      
      case 'localhost':
        sidecarInfo = {
          id: container.id,
          type: 'localhost-viewer' as const,
          // ... properties
        };
        break;
      
      // ... other cases for file, document-editor, app-installer
    }

    if (sidecarInfo) {
      unifiedSidecarContext.registerSidecar(sidecarInfo);
    }
  });

  // Cleanup: unregister on unmount
  return () => {
    containers.forEach(container => {
      unifiedSidecarContext.unregisterSidecar(container.id);
    });
  };
}, [unifiedSidecarContext, containers]);
```

#### **2. Fixed useChatStream Context Injection** (`src/hooks/useChatStream.ts`)

**CRITICAL**: BaseChat2 uses `useChatStream` (NOT `useMessageStream`), so we needed to add the same context injection logic:

```typescript
const handleSubmit = useCallback(
  async (userMessage: string) => {
    // ... existing code ...
    
    // Inject sidecar context into the user message before creating the message object
    let messageWithContext = userMessage;
    
    // Get unified sidecar context from global window object
    const unifiedSidecarContext = (window as any).__unifiedSidecarContext;
    
    if (unifiedSidecarContext && unifiedSidecarContext.getSidecarContext) {
      try {
        const activeSidecars = unifiedSidecarContext.getActiveSidecars ? 
          unifiedSidecarContext.getActiveSidecars() : [];
        
        const contextInfo = unifiedSidecarContext.getSidecarContext();
        
        if (contextInfo.trim()) {
          // Prepend context to user message
          messageWithContext = `${contextInfo}\n\n---\n\n${userMessage}`;
          console.log('🔧 useChatStream: Successfully injected sidecar context');
        }
      } catch (error) {
        console.error('🔧 useChatStream: Error injecting sidecar context:', error);
      }
    }
    
    const currentMessages = [...messagesRef.current, createUserMessage(messageWithContext)];
    // ... rest of submit logic ...
  },
  [sessionId, session, gooseEnabled, setMessagesAndLog, onFinish, onSessionIdChange]
);
```

This ensures that **both** message streaming systems inject context properly.

## Data Flow (After Fix)

```
User clicks dock icon (SidecarInvoker)
    ↓
Dispatches 'add-container' event
    ↓
MainPanelLayout creates container
    ↓
EnhancedBentoBox renders container
    ↓
✅ REGISTERS with UnifiedSidecarContext
    ↓
useMessageStream.injectSidecarContext() finds active sidecars
    ↓
✅ Context INJECTED into user message
    ↓
AI receives rich context about open sidecars
```

## Context Injection Details

### What Gets Injected

When a user sends a message, `useMessageStream` calls `injectSidecarContext()` which:

1. Gets active sidecars from `window.__unifiedSidecarContext`
2. Calls `getSidecarContext()` to generate markdown context
3. Prepends context to the last user message

### Example Context Injection

**User types**: "What do you think?"

**AI receives**:
```markdown
## Active Tools & Context
The user currently has the following tools and content open in sidecars:

### 1. Web Browser - Google
Currently viewing **https://google.com** (google.com). Secure HTTPS connection. Page loaded.
**Helpful actions:**
- Help analyze or summarize the current webpage content
- Explain concepts or information from the current page
- Navigate to related resources or documentation

### 2. File Viewer - example.ts
Viewing file **/Users/user/project/example.ts** (ts, 5KB). File is readable. Last modified: 1/24/2025.
**Helpful actions:**
- Analyze or explain the file content
- Suggest improvements or modifications
- Help with file format conversion
- Explain file structure or syntax
- Create related files or documentation

---

What do you think?
```

## Testing

### How to Verify the Fix

1. **Open the app** and start a new chat
2. **Click a dock icon** (e.g., Safari/Web Browser)
3. **Open browser console** and check for logs:
   ```
   🔧 EnhancedBentoBox: Registering 1 containers with unified context
   🔧 EnhancedBentoBox: Registered sidecar: bento-123456789 web-viewer
   ```
4. **Send a message** to the AI
5. **Check console** for injection logs:
   ```
   🔧 useMessageStream: Found unified sidecar context
   🔧 useMessageStream: Active sidecars before context generation: 1
   🔧 useMessageStream: Generated context info length: 456 chars
   🔧 useMessageStream: Injecting context into user message
   🔧 useMessageStream: Successfully injected sidecar context
   ```

### Expected Behavior

- AI should be aware of open sidecars
- AI should reference specific files, URLs, or content
- AI should provide contextual suggestions based on what's open

## Performance Considerations

### Registration Overhead

- **When**: Every time `containers` array changes
- **Cost**: O(n) where n = number of containers
- **Typical**: 1-3 containers, negligible impact

### Memory Impact

- Each registered sidecar: ~500 bytes
- Max realistic containers: ~10
- Total overhead: ~5KB (negligible)

### Optimization Opportunities

1. **Memoize sidecar info generation**:
```typescript
const sidecarInfo = useMemo(() => {
  return generateSidecarInfo(container);
}, [container.id, container.contentType, container.contentProps]);
```

2. **Debounce rapid container changes**:
```typescript
const debouncedRegister = useMemo(
  () => debounce(registerSidecar, 100),
  [registerSidecar]
);
```

## Future Enhancements

### 1. Real-time Content Updates

Currently, sidecar info is static. Could add:
- WebViewer: Track navigation state, page title
- FileViewer: Track scroll position, selected text
- DocumentEditor: Track cursor position, word count

### 2. Selective Context Injection

Add user preference to control which sidecars contribute to context:
```typescript
interface SidecarPreferences {
  includeInContext: boolean;
  priority: number; // For context ordering
}
```

### 3. Context Size Limits

Implement smart truncation for large contexts:
```typescript
const MAX_CONTEXT_LENGTH = 2000; // chars
if (contextInfo.length > MAX_CONTEXT_LENGTH) {
  contextInfo = truncateSmartly(contextInfo, MAX_CONTEXT_LENGTH);
}
```

### 4. Context Caching

Cache generated context to avoid regeneration:
```typescript
const contextCache = new Map<string, { context: string; timestamp: number }>();
const CACHE_TTL = 5000; // 5 seconds
```

## Related Files

- ✅ `src/components/Layout/EnhancedBentoBox.tsx` - Added context registration (FIXED)
- ✅ `src/hooks/useChatStream.ts` - Added context injection for BaseChat2 (FIXED)
- ✅ `src/hooks/useMessageStream.ts` - Already had context injection (no changes needed)
- `src/contexts/UnifiedSidecarContext.tsx` - Context provider (no changes needed)
- `src/components/WebViewer.tsx` - Self-registers with context (no changes needed)
- `src/components/BaseChat2.tsx` - Uses useChatStream (no changes needed)
- `src/components/SidecarLayout.tsx` - Legacy implementation (reference only)

## Commit Message

```
fix(sidecar): Add UnifiedSidecarContext registration to EnhancedBentoBox

The EnhancedBentoBox component (used by the dock system) was not registering
sidecars with UnifiedSidecarContext, preventing AI context injection.

Added useEffect hook to register/unregister containers with the context,
enabling the AI to be aware of open sidecars and provide contextual responses.

Fixes: Context not being injected into user messages
Affects: All dock-based sidecars (Web Browser, File Viewer, etc.)
```

## Verification Checklist

- [x] Import UnifiedSidecarContext hook
- [x] Add context registration useEffect
- [x] Handle all container types (web-viewer, localhost, file, document-editor, app-installer)
- [x] Add cleanup/unregistration on unmount
- [x] Add console logging for debugging
- [x] Test with multiple containers
- [x] Verify context injection in useMessageStream
- [x] Document the fix

## Notes

- The fix is **backward compatible** - SidecarLayout still works independently
- No changes needed to `useMessageStream` - it already had the injection logic
- The global `window.__unifiedSidecarContext` pattern is preserved
- Console logs can be removed in production if desired

### Component Self-Registration

**WebViewer** (`src/components/WebViewer.tsx`) handles its own registration:
- Registers on mount when `childWindowCreated` becomes true (lines 389-445)
- Updates context with real-time navigation data (URL, title, loading state)
- Uses its own `childWindowId` (not the container ID) for registration
- Unregisters on unmount

This is the **correct approach** for components with dynamic state. EnhancedBentoBox should NOT duplicate this registration.

**Other components** (FileViewer, DocumentEditor, SidecarTabs, AppInstaller) do NOT self-register, so EnhancedBentoBox must register them.

## Discovery 4: Tab-Specific Sidecars Not Registering

### Problem
After fixing the dock sidecar registration and message streaming, it was discovered that **tab-specific sidecars** (the sidecars that open on the right side of individual tabs when you click the dock icons in the chat input area) were also not registering with `UnifiedSidecarContext`.

### Architecture
- **TabContext** (`src/contexts/TabContext.tsx`) - Manages tab-specific sidecar state
- **TabSidecarInvoker** (`src/components/TabSidecarInvoker.tsx`) - The dock buttons above chat input
- **TabSidecar** (`src/components/TabSidecar.tsx`) - Renders the tab-specific sidecar content
- **TabbedChatContainer** (`src/components/TabbedChatContainer.tsx`) - Orchestrates tabs and sidecars

### Root Cause
`TabSidecar` component was rendering sidecar content but had **no integration** with `UnifiedSidecarContext`. It was completely unaware of the AI context system.

### Solution
Added `UnifiedSidecarContext` registration to `TabSidecar` component:

#### Changes Made to `src/components/TabSidecar.tsx`:

1. **Import the context hook**:
```typescript
import { useUnifiedSidecarContextOptional } from '../contexts/UnifiedSidecarContext';
```

2. **Get the context in the component**:
```typescript
const unifiedSidecarContext = useUnifiedSidecarContextOptional();
```

3. **Register the current view with context** (useEffect hook):
```typescript
useEffect(() => {
  if (!unifiedSidecarContext || !currentView) {
    return;
  }

  const sidecarId = `tab-${tabId}-${currentView.id}`;
  let sidecarInfo;

  switch (currentView.contentType) {
    case 'diff':
      sidecarInfo = {
        id: sidecarId,
        type: 'diff-viewer' as const,
        title: currentView.title || 'Diff Viewer',
        fileName: currentView.fileName || 'File',
        addedLines,
        removedLines,
        totalChanges: addedLines + removedLines,
        viewMode: viewMode,
        timestamp: Date.now(),
      };
      break;

    case 'localhost':
      sidecarInfo = {
        id: sidecarId,
        type: 'localhost-viewer' as const,
        title: currentView.title || 'Localhost Viewer',
        url: localhostUrl,
        port,
        protocol: new URL(localhostUrl).protocol.replace(':', '') as 'http' | 'https',
        isLocal: true,
        serviceType: 'development',
        timestamp: Date.now(),
      };
      break;

    case 'file':
      sidecarInfo = {
        id: sidecarId,
        type: 'file-viewer' as const,
        title: currentView.title || 'File Viewer',
        filePath,
        fileName,
        fileSize: 0,
        fileType: fileExtension,
        isReadable: true,
        lastModified: Date.now(),
        timestamp: Date.now(),
      };
      break;

    case 'editor':
      sidecarInfo = {
        id: sidecarId,
        type: 'document-editor' as const,
        title: currentView.title || 'Document Editor',
        filePath: editorPath,
        fileName: editorFileName,
        contentLength: (currentView.contentProps.content || '').length,
        hasUnsavedChanges: false,
        isNewDocument: !editorPath,
        language: editorPath ? editorPath.split('.').pop() : undefined,
        timestamp: Date.now(),
      };
      break;

    case 'web':
      // Skip - WebBrowser component handles its own registration
      console.log('🔧 TabSidecar: Skipping web viewer registration (handled by WebBrowser component)');
      break;
  }

  if (sidecarInfo) {
    unifiedSidecarContext.registerSidecar(sidecarInfo);
    console.log('🔧 TabSidecar: Registered sidecar:', sidecarInfo.id, sidecarInfo.type);
  }

  // Cleanup: unregister when view changes or component unmounts
  return () => {
    if (sidecarInfo) {
      unifiedSidecarContext.unregisterSidecar(sidecarId);
      console.log('🔧 TabSidecar: Unregistered sidecar:', sidecarId);
    }
  };
}, [unifiedSidecarContext, currentView, tabId, viewMode]);
```

### Key Points
- Tab-specific sidecars use a unique ID format: `tab-${tabId}-${viewId}`
- WebBrowser component still handles its own registration (no duplication)
- Registration updates when view changes or viewMode changes (for diff viewer)
- Proper cleanup on unmount or view change

### Testing
1. Open a chat tab
2. Click a dock icon (e.g., Document Editor) above the chat input
3. Sidecar should open on the right side of the tab
4. Check console for registration logs:
   ```
   🔧 TabSidecar: Registering view with unified context: editor document-editor
   🔧 TabSidecar: Registered sidecar: tab-xxx-editor document-editor
   ```
5. Send a message to the AI
6. AI should now be aware of the document editor content

## Summary of All Fixes

1. ✅ **EnhancedBentoBox** - Fixed registration for dock containers (non-self-registering components)
2. ✅ **useChatStream** - Added context injection for BaseChat2's message streaming
3. ✅ **TabSidecar** - Added registration for tab-specific sidecars
4. ✅ **EnhancedBentoBox** - Fixed duplicate registration issue (skip web-viewer, let WebViewer self-register)

All sidecar types now properly register with `UnifiedSidecarContext` and context is injected into AI messages via both message streaming systems.
