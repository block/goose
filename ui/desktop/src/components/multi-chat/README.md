# Multi-Chat Browser-Style Tabs

A browser-style tabbed interface for managing multiple chat sessions simultaneously.

## Overview

This implementation provides a familiar browser-like tab experience for managing multiple Goose chat sessions in a single window.

## Components

### `MultiChatView.tsx`
Main container component that orchestrates the multi-chat experience.

**Features:**
- Tab bar with horizontal scrolling
- Keyboard shortcuts (Cmd/Ctrl + 1-9, Cmd/Ctrl + T, Cmd/Ctrl + W, Cmd/Ctrl + Tab)
- Drag-and-drop tab reordering
- Scroll left/right buttons when tabs overflow
- New tab button
- Empty state when no tabs are open

**Props:**
```typescript
interface MultiChatViewProps {
  setChat: (chat: ChatType) => void;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
}
```

### `ChatTab.tsx`
Individual tab component representing a single chat session.

**Features:**
- Active/inactive states with visual indicators
- Truncated session names (max 20 chars)
- Close button (visible on hover or when active)
- Drag handle (visible on hover)
- Unread indicator dot
- Loading state with pulse animation
- Active tab indicator line

**Props:**
```typescript
interface ChatTabProps {
  session: Session | null;
  isActive: boolean;
  isLoading?: boolean;
  onSelect: () => void;
  onClose: () => void;
  onDragStart?: (e: React.DragEvent) => void;
  onDragOver?: (e: React.DragEvent) => void;
  onDrop?: (e: React.DragEvent) => void;
  hasUnread?: boolean;
}
```

### `useMultiChat.ts`
Custom hook for managing multi-chat state.

**Features:**
- Manages open sessions array
- Persists to localStorage
- Loads session details asynchronously
- Handles session opening/closing
- Creates new sessions
- Reorders sessions (for drag-and-drop)
- Limits to 10 concurrent sessions
- Auto-switches to adjacent tab when closing active tab

**API:**
```typescript
interface UseMultiChatReturn {
  openSessions: OpenSession[];
  activeSessionId: string | null;
  setActiveSessionId: (sessionId: string) => void;
  openSession: (sessionId: string) => void;
  closeSession: (sessionId: string) => void;
  createNewSession: () => void;
  reorderSessions: (fromIndex: number, toIndex: number) => void;
}
```

## Usage

### Basic Integration

```tsx
import { MultiChatView } from './components/multi-chat';

function App() {
  return (
    <MultiChatView
      setChat={setChat}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
    />
  );
}
```

### Add Route (App.tsx)

```tsx
import { MultiChatView } from './components/multi-chat';

// In your routes:
<Route
  path="multi-chat"
  element={
    <MultiChatView
      setChat={setChat}
      setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
    />
  }
/>
```

### Open from Navigation

```tsx
// From TopNavigation or any other component:
navigate('/multi-chat');

// Or with initial sessions:
navigate('/multi-chat', {
  state: { sessionIds: ['session-1', 'session-2'] }
});
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl + 1-9` | Switch to tab 1-9 |
| `Cmd/Ctrl + T` | New tab |
| `Cmd/Ctrl + W` | Close current tab |
| `Cmd/Ctrl + Tab` | Next tab |
| `Cmd/Ctrl + Shift + Tab` | Previous tab |

## Features

### ✅ Implemented

- [x] Browser-style tab bar
- [x] Open/close tabs
- [x] Active tab highlighting
- [x] Tab reordering via drag-and-drop
- [x] Keyboard shortcuts
- [x] Horizontal scrolling with scroll buttons
- [x] Session name truncation
- [x] Loading states
- [x] Empty state
- [x] localStorage persistence
- [x] Max 10 concurrent sessions
- [x] Auto-switch on tab close

### 🚧 TODO

- [ ] Unread indicators (requires backend API)
- [ ] Tab context menu (right-click)
- [ ] Pin/unpin tabs
- [ ] Duplicate tab
- [ ] Close all tabs
- [ ] Close other tabs
- [ ] Tab preview on hover
- [ ] Session search/filter
- [ ] Tab groups/colors
- [ ] Restore closed tabs (history)
- [ ] Sync across windows

## State Management

### localStorage Schema

```json
{
  "sessionIds": ["session-1", "session-2", "session-3"],
  "activeId": "session-2"
}
```

**Key:** `goose_multi_chat_sessions`

### Session Lifecycle

1. **Open Session**
   - Add to `openSessions` array with loading state
   - Fetch session details asynchronously
   - Update with loaded data

2. **Close Session**
   - Remove from `openSessions` array
   - If closing active session, switch to adjacent tab
   - Update localStorage

3. **New Session**
   - Generate temporary ID: `new-{timestamp}`
   - Add to `openSessions` array
   - Set as active
   - Backend will assign real ID on first message

## Integration with BaseChat2

Each tab renders a separate instance of `BaseChat2`:

```tsx
<BaseChat2
  key={sessionId}  // Force remount on session change
  sessionId={sessionId}
  setChat={setChat}
  setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
  suppressEmptyState={false}
/>
```

**Key Points:**
- Each `BaseChat2` instance manages its own state via `useChatStream`
- The `key` prop forces React to remount when switching sessions
- Session state is preserved by the backend, not in React

## Styling

### Tab Bar
- Height: `auto` (based on content)
- Background: `bg-background-muted`
- Border: `border-b border-border-default`

### Active Tab
- Background: `bg-background-default`
- Text: `text-text-default`
- Indicator: Bottom border with `bg-background-accent`

### Inactive Tab
- Background: `bg-background-muted`
- Text: `text-text-muted`
- Hover: `bg-background-medium`

### Tab Dimensions
- Min width: `140px`
- Max width: `200px`
- Padding: `px-4 py-2.5`

## Performance Considerations

1. **Lazy Loading**: Session details are loaded asynchronously
2. **Key-based Remounting**: Each tab gets a unique key to prevent state leakage
3. **Scroll Optimization**: Uses `scroll-smooth` and debounced scroll checks
4. **Drag Performance**: Uses native drag-and-drop API
5. **localStorage**: Persists only session IDs, not full session data

## Browser Compatibility

- Chrome/Edge: ✅ Full support
- Firefox: ✅ Full support
- Safari: ✅ Full support
- Electron: ✅ Full support (primary target)

## Accessibility

- **Keyboard Navigation**: Full keyboard support
- **ARIA Labels**: Close buttons have `aria-label`
- **Focus Management**: Proper focus handling on tab switch
- **Screen Readers**: Semantic HTML structure

## Future Enhancements

### Phase 2: Advanced Features
- Tab context menu (right-click)
- Pin important tabs
- Tab preview thumbnails
- Session search within tabs
- Tab groups with colors

### Phase 3: Collaboration
- Shared tabs (multiple users)
- Real-time sync across windows
- Presence indicators

### Phase 4: Productivity
- Tab templates
- Quick actions menu
- Batch operations
- Export/import tab sets

## Testing

### Manual Testing Checklist

- [ ] Open multiple tabs
- [ ] Switch between tabs
- [ ] Close tabs (middle, first, last)
- [ ] Drag-and-drop reorder
- [ ] Keyboard shortcuts
- [ ] Scroll tabs (overflow)
- [ ] Create new tab
- [ ] Refresh page (persistence)
- [ ] Close active tab (auto-switch)
- [ ] Max 10 tabs limit

### Unit Tests (TODO)

```typescript
describe('useMultiChat', () => {
  it('should open a session');
  it('should close a session');
  it('should reorder sessions');
  it('should limit to 10 sessions');
  it('should persist to localStorage');
  it('should switch to adjacent tab on close');
});
```

## Known Issues

1. **New Session ID**: Temporary IDs need to be replaced with real IDs after first message
2. **Unread Tracking**: Requires backend API changes
3. **Session Sync**: No real-time sync between tabs yet
4. **Memory**: Multiple BaseChat2 instances may use more memory

## Contributing

When adding features:
1. Update this README
2. Add TypeScript types
3. Follow existing patterns
4. Test keyboard shortcuts
5. Check accessibility
6. Update TODO list
