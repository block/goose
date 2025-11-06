# Sidecar Plus Button & Bento Box Implementation Review

## Overview
This document reviews the sidecar implementation focusing on the **Plus Button** hover mechanism and the **Bento Box** multi-container system in the Goose Desktop application.

---

## Architecture

### Component Hierarchy
```
AppLayout (SidecarProvider)
  └── AppLayoutContent
      ├── SidecarInvoker (Plus Button)
      └── SidebarInset
          └── Outlet (Routes)
              └── MainPanelLayout
                  ├── Chat Panel (resizable)
                  ├── ResizeHandle
                  └── BentoBox (multi-container)
```

---

## 1. SidecarInvoker (Plus Button)

**Location**: `ui/desktop/src/components/Layout/SidecarInvoker.tsx`

### Purpose
Provides a hover-activated UI on the right edge of the screen that allows users to add new sidecar containers.

### Key Features

#### Hover Detection Zone
- **Fixed positioning**: `fixed top-0 right-0` with dynamic width
- **Width behavior**:
  - Closed: 16px (thin hover zone)
  - Open: 200px (covers menu area)
- **Pointer events**: Uses `pointer-events-none` on container, `pointer-events-auto` on interactive elements

#### Plus Button
- **Positioning**: Absolute, centered vertically on right edge
- **Animation**: Smooth fade-in/slide-in on hover
  ```tsx
  className={`transition-all duration-300 ease-out ${
    isHovering || showMenu ? 'opacity-100 translate-x-0' : 'opacity-0 translate-x-2'
  }`}
  ```
- **Visual**: 32px circular button with Plus icon

#### Floating Menu
- **Positioning**: Appears to the left of plus button
- **Animation**: `animate-in fade-in slide-in-from-right-2 duration-200`
- **Options**:
  1. **Sidecar View** - Generic sidecar container
  2. **Localhost Viewer** - Web viewer for localhost URLs
  3. **Open File** - File viewer with native file picker

### State Management
```tsx
const [isHovering, setIsHovering] = useState(false);
const [showMenu, setShowMenu] = useState(false);
```

### Event Flow
1. User hovers over right edge → `setIsHovering(true)`
2. Plus button fades in
3. User clicks plus → `setShowMenu(true)`
4. Menu appears with 3 options
5. User selects option → `onAddContainer(type, filePath?)`
6. Menu closes, event dispatched

### Integration
```tsx
// In AppLayout
<SidecarInvoker 
  onShowLocalhost={handleShowLocalhost}
  onShowFileViewer={handleShowFileViewer}
  onAddContainer={handleAddContainer}
  isVisible={shouldShowSidecarInvoker}
/>
```

### Visibility Logic
Only shown on chat-related pages:
```tsx
const shouldShowSidecarInvoker = 
  (location.pathname === '/' || 
   location.pathname === '/chat' || 
   location.pathname === '/pair');
```

---

## 2. BentoBox (Multi-Container System)

**Location**: `ui/desktop/src/components/Layout/MainPanelLayout.tsx`

### Purpose
A flexible container system that holds multiple sidecar views side-by-side with dynamic resizing.

### Architecture

#### Container Structure
```tsx
interface SidecarContainer {
  id: string;                              // Unique identifier
  content: React.ReactNode;                // The actual content
  contentType: 'sidecar' | 'localhost' | 'file' | null;
  title?: string;                          // Display title
}
```

#### State Management
```tsx
const [hasBentoBox, setHasBentoBox] = useState(false);
const [bentoBoxContainers, setBentoBoxContainers] = useState<SidecarContainer[]>([]);
const [chatWidth, setChatWidth] = useState(600);
```

### Key Features

#### 1. Dynamic Container Widths
- **Equal distribution**: Containers split available space equally
- **Percentage-based**: Uses percentages for responsive behavior
- **Auto-recalculation**: Updates when containers are added/removed

```tsx
useEffect(() => {
  if (containers.length > 0) {
    const equalWidth = Math.floor(100 / containers.length);
    const widths = {};
    containers.forEach(container => {
      widths[container.id] = equalWidth;
    });
    setContainerWidths(widths);
  }
}, [containers.length]);
```

#### 2. Resize Handles
- **Between containers**: Drag to adjust relative widths
- **Visual feedback**: Hover and active states
- **Constraints**:
  - Minimum width: 5%
  - Maximum width: 80%
  - Preserves total width

```tsx
const ResizeHandleBento: React.FC<{ containerIndex: number }> = ({ containerIndex }) => {
  // Handles mouse events for resizing
  // Updates containerWidths state
  // Applies min/max constraints
};
```

#### 3. Container Management

**Adding Containers**:
```tsx
const addToBentoBox = useCallback((contentType, filePath?) => {
  const newContainer: SidecarContainer = {
    id: `bento-${Date.now()}`,
    content: null,
    contentType: null
  };
  
  // Create content based on type
  if (contentType === 'sidecar') { /* ... */ }
  else if (contentType === 'localhost') { /* ... */ }
  else if (contentType === 'file') { /* ... */ }
  
  // Add to bento box
  if (!hasBentoBox) {
    setHasBentoBox(true);
    setBentoBoxContainers([newContainer]);
  } else {
    setBentoBoxContainers(prev => [...prev, newContainer]);
  }
}, [hasBentoBox]);
```

**Removing Containers**:
```tsx
const removeFromBentoBox = useCallback((containerId: string) => {
  setBentoBoxContainers(prev => {
    const updated = prev.filter(c => c.id !== containerId);
    
    // Hide bento box if no containers left
    if (updated.length === 0) {
      setHasBentoBox(false);
    }
    return updated;
  });
}, []);
```

#### 4. Close Buttons

**Individual Container Close**:
- Red X button in top-right of each container
- Removes only that container
- Highly visible with hover effects

**Entire Bento Box Close**:
- Small X button in top-left of bento box
- Removes all containers at once
- Closes the entire bento box

### Layout Behavior

#### Without Bento Box
```tsx
<div className="flex flex-col flex-1">
  {children} {/* Chat takes full width */}
</div>
```

#### With Bento Box
```tsx
<div className="flex flex-col flex-shrink-0" style={{ width: `${chatWidth}px` }}>
  {children} {/* Chat has fixed width */}
</div>
<ResizeHandle /> {/* Drag to adjust chat width */}
<BentoBox /> {/* Takes remaining space */}
```

### Content Types

#### 1. Sidecar View (Generic)
```tsx
<div className="h-full w-full flex items-center justify-center">
  <p>Sidecar content will go here</p>
</div>
```

#### 2. Localhost Viewer
```tsx
<SidecarTabs initialUrl="http://localhost:3000" />
```
- Web viewer component
- Supports multiple localhost ports
- Tab-based interface

#### 3. File Viewer
```tsx
<FileViewer filePath={filePath} />
```
- Native file picker integration
- Displays file content
- Shows filename in title

---

## 3. Event Communication

### Custom Events
The system uses custom DOM events for cross-component communication:

```tsx
// Dispatch (from SidecarInvoker)
window.dispatchEvent(new CustomEvent('add-container', { 
  detail: { type, filePath } 
}));

// Listen (in MainPanelLayout)
useEffect(() => {
  const handleAddContainer = (e: CustomEvent) => {
    addToBentoBox(e.detail.type, e.detail.filePath);
  };
  window.addEventListener('add-container', handleAddContainer);
  return () => window.removeEventListener('add-container', handleAddContainer);
}, [addToBentoBox]);
```

### Why Custom Events?
- **Decoupling**: Components don't need direct references
- **Flexibility**: Easy to add new listeners
- **React-friendly**: Works with component lifecycle

---

## 4. Styling & UX

### Design Tokens
The implementation uses a consistent design system:

```tsx
// Background colors
bg-background-default
bg-background-medium
bg-background-muted

// Border colors
border-border-subtle
border-border-strong

// Text colors
text-text-default
text-text-muted
text-textStandard
text-textSubtle
```

### Animations
- **Fade-in**: `animate-in fade-in`
- **Slide-in**: `slide-in-from-right-2`
- **Transitions**: `transition-all duration-200`
- **Hover effects**: `hover:scale-105`, `hover:shadow-xl`

### Responsive Behavior
- Minimum container width: 5%
- Maximum container width: 80%
- Chat panel min width: 300px
- Chat panel max width: 1000px

---

## 5. Integration Points

### With SidecarProvider
The old sidecar system is still available via context:

```tsx
const sidecar = useSidecar();

// Old API (single sidecar)
sidecar?.showLocalhostViewer(url);
sidecar?.showFileViewer(filePath);
sidecar?.showDiffViewer(content);
```

### With Electron
Native file picker integration:

```tsx
const filePath = await window.electron.selectFileOrDirectory();
```

### With Router
Visibility based on current route:

```tsx
const shouldShowSidecarInvoker = 
  (location.pathname === '/' || 
   location.pathname === '/chat' || 
   location.pathname === '/pair');
```

---

## 6. Potential Issues & Improvements

### Current Issues

#### 1. **Z-Index Management**
The X button uses `z-index: 999999` which is excessive:
```tsx
style={{ 
  zIndex: 999999,  // ❌ Too high
  position: 'absolute',
  top: '8px',
  right: '8px'
}}
```
**Recommendation**: Use a more reasonable z-index (e.g., 50-100) and manage stacking contexts properly.

#### 2. **Debug Logging**
Extensive console.log statements throughout:
```tsx
console.log('🔍 SidecarInvoker: Sidecar button clicked');
console.log('🔍 MainPanelLayout: removeFromBentoBox called');
```
**Recommendation**: Remove or wrap in a debug flag for production.

#### 3. **Alert in Production Code**
```tsx
onClick={() => {
  console.log('🔍 X BUTTON CLICKED for container:', container.id);
  alert(`Removing container: ${container.id}`);  // ❌ Alert in production
  onRemoveContainer(container.id);
}}
```
**Recommendation**: Remove the alert statement.

#### 4. **Unused useSidecar Hook**
In SidecarInvoker:
```tsx
const sidecar = useSidecar();  // ❌ Not used
```
**Recommendation**: Remove if not needed.

#### 5. **Parent Width Caching**
The resize logic caches parent width but updates it on window resize:
```tsx
const parentWidthRef = useRef<number>(800);  // Default 800px
```
**Recommendation**: Initialize with actual width or use a more robust measurement strategy.

### Potential Improvements

#### 1. **Keyboard Navigation**
Add keyboard shortcuts:
- `Cmd/Ctrl + K` to open plus menu
- `Escape` to close menu
- Arrow keys to navigate options

#### 2. **Drag & Drop**
Allow reordering containers by dragging:
```tsx
// Potential implementation
<DraggableContainer 
  id={container.id}
  onReorder={handleReorder}
>
  {container.content}
</DraggableContainer>
```

#### 3. **Persistence**
Save bento box state to localStorage:
```tsx
useEffect(() => {
  localStorage.setItem('bentoBoxState', JSON.stringify({
    containers: bentoBoxContainers,
    chatWidth
  }));
}, [bentoBoxContainers, chatWidth]);
```

#### 4. **Container Templates**
Predefined layouts:
```tsx
const templates = {
  'split': [50, 50],
  'thirds': [33, 33, 34],
  'focus': [70, 30]
};
```

#### 5. **Accessibility**
- Add ARIA labels
- Keyboard focus management
- Screen reader announcements

```tsx
<Button
  aria-label="Add new sidecar container"
  aria-expanded={showMenu}
  aria-haspopup="menu"
>
  <Plus />
</Button>
```

#### 6. **Error Boundaries**
Wrap containers in error boundaries:
```tsx
<ErrorBoundary fallback={<ContainerError />}>
  {container.content}
</ErrorBoundary>
```

#### 7. **Loading States**
Show loading indicators when content is being fetched:
```tsx
{isLoading ? (
  <Spinner />
) : (
  container.content
)}
```

#### 8. **Container Maximization**
Allow temporary full-screen of a container:
```tsx
const [maximizedContainer, setMaximizedContainer] = useState<string | null>(null);
```

---

## 7. Testing Considerations

### Unit Tests Needed
1. **SidecarInvoker**
   - Hover detection
   - Menu open/close
   - Option selection
   - Click outside to close

2. **BentoBox**
   - Container addition/removal
   - Width calculations
   - Resize constraints
   - Equal distribution

3. **Event Communication**
   - Custom event dispatch
   - Event listener cleanup

### Integration Tests
1. Full flow: hover → click → add container
2. Multiple containers with resizing
3. Container removal and bento box cleanup
4. Chat panel resize interaction

### E2E Tests
1. User adds localhost viewer
2. User adds file viewer
3. User resizes containers
4. User removes individual containers
5. User closes entire bento box

---

## 8. Performance Considerations

### Current Optimizations
- `useCallback` for event handlers
- `useMemo` could be added for expensive calculations
- Ref caching for parent width

### Potential Optimizations
1. **Debounce resize events**:
```tsx
const debouncedResize = useMemo(
  () => debounce(handleResize, 16), // 60fps
  [handleResize]
);
```

2. **Virtual scrolling** for many containers
3. **Lazy loading** container content
4. **Memoize container components**:
```tsx
const MemoizedContainer = React.memo(Container);
```

---

## 9. Code Quality

### Strengths ✅
- Clear component separation
- Consistent naming conventions
- Good use of TypeScript interfaces
- Proper cleanup in useEffect
- Responsive design with percentages

### Areas for Improvement ⚠️
- Remove debug code
- Add error handling
- Improve accessibility
- Add unit tests
- Document complex logic
- Reduce z-index values
- Add loading states

---

## 10. Summary

### What Works Well
1. **Intuitive UX**: Hover-to-reveal plus button is discoverable
2. **Flexible Layout**: Bento box supports multiple containers with resizing
3. **Clean Architecture**: Good separation of concerns
4. **Smooth Animations**: Professional feel with transitions
5. **Type Safety**: Strong TypeScript usage

### What Needs Work
1. **Production Readiness**: Remove debug code and alerts
2. **Accessibility**: Add ARIA labels and keyboard support
3. **Error Handling**: Add boundaries and fallbacks
4. **Testing**: Comprehensive test coverage needed
5. **Performance**: Optimize resize calculations
6. **Documentation**: Add JSDoc comments

### Recommended Next Steps
1. ✅ Remove debug logging and alerts
2. ✅ Fix z-index issues
3. ✅ Add accessibility features
4. ✅ Write unit tests
5. ✅ Add error boundaries
6. ✅ Implement keyboard shortcuts
7. ✅ Add persistence
8. ✅ Performance profiling

---

## 11. Code Examples

### Adding a New Container Type

```tsx
// 1. Update the interface
type ContentType = 'sidecar' | 'localhost' | 'file' | 'terminal';

// 2. Add to SidecarInvoker menu
<Button onClick={handleTerminalClick}>
  <Terminal className="w-4 h-4 mr-2" />
  Terminal
</Button>

// 3. Handle in addToBentoBox
else if (contentType === 'terminal') {
  newContainer.content = <TerminalViewer />;
  newContainer.contentType = 'terminal';
  newContainer.title = 'Terminal';
}
```

### Custom Resize Constraints

```tsx
const handleResize = useCallback((containerIndex: number, delta: number) => {
  setContainerWidths(prev => {
    const newWidths = { ...prev };
    
    // Custom constraints
    const MIN_WIDTH = 10; // 10%
    const MAX_WIDTH = 70; // 70%
    
    // Apply delta with constraints
    const leftWidth = Math.max(MIN_WIDTH, 
      Math.min(MAX_WIDTH, prev[containerIndex] + deltaPercent)
    );
    
    return newWidths;
  });
}, []);
```

---

## Conclusion

The sidecar plus button and bento box implementation provides a solid foundation for a multi-view interface. The architecture is clean and extensible, but needs polish for production readiness. Focus on removing debug code, adding accessibility features, and implementing comprehensive tests.

The hover-based UX is intuitive, and the flexible container system allows for powerful multi-tasking workflows. With the recommended improvements, this will be a professional-grade feature.
