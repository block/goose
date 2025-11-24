# Responsive Comment System

## Overview

The comment system now adapts to available screen space, automatically switching between **full mode** (floating panel) and **condensed mode** (compact badge + modal) based on viewport width and sidecar state.

## Architecture

### Display Modes

#### Full Mode (Default)
- **When**: Wide screens (>1200px) with no sidecar open
- **UI**: Comments float to the right of messages in a fixed-width panel (320px)
- **Interaction**: Inline editing, replying, and resolving

#### Condensed Mode
- **When**: Narrow screens (<1200px) OR sidecar is open
- **UI**: Compact badge showing comment count
- **Interaction**: Click badge to open modal with full comment functionality

### Key Components

#### 1. `useCommentDisplayMode` Hook
**Location**: `ui/desktop/src/hooks/useCommentDisplayMode.ts`

Detects display mode based on:
- Container width (via ResizeObserver)
- Sidecar state (via UnifiedSidecarContext)
- Configurable breakpoint (default: 1200px)

```typescript
const { displayMode, availableWidth, hasSidecar } = useCommentDisplayMode({
  containerRef: messageContainerRef,
  breakpoint: 1200,
  condenseWithSidecar: true,
});
```

**Returns**:
- `displayMode`: 'full' | 'condensed'
- `availableWidth`: Current width in pixels
- `hasSidecar`: Boolean indicating if any sidecar is active
- `isCondensed`: Boolean helper

#### 2. `CommentBadge` Component
**Location**: `ui/desktop/src/components/CommentBadge.tsx`

Compact indicator shown in condensed mode:
- Shows total comment count
- Displays resolved count if applicable
- Hover preview of first comment
- Color-coded (blue = active, green = all resolved)
- Click to open modal

#### 3. `CommentDrawer` Component
**Location**: `ui/desktop/src/components/CommentDrawer.tsx`

Expandable drawer for condensed mode (like an accordion):
- Expands inline from the badge
- Smooth height transition (200ms)
- Max height: 600px with scrollable content
- Collapsible header with chevron icon
- Comment creation/editing/replying
- Uses app design tokens
- Stays within message flow

#### 4. Updated `MessageComments` Component
**Location**: `ui/desktop/src/components/MessageComments.tsx`

Now supports both display modes:
```typescript
if (displayMode === 'condensed') {
  return (
    <>
      <CommentBadge ... />
      <CommentModal ... />
    </>
  );
}

// Full mode: inline comments
return <div>...</div>;
```

#### 5. Updated `GooseMessage` Component
**Location**: `ui/desktop/src/components/GooseMessage.tsx`

Integrates display mode detection:
- Uses `useCommentDisplayMode` hook
- Passes `displayMode` to `MessageComments`
- Adjusts container positioning based on mode

## User Experience

### Full Mode
1. User selects text in a message
2. "💬 Comment" button appears to the right
3. Click to show comment input in right panel
4. Comments remain visible while scrolling
5. Highlights show on hover

### Condensed Mode
1. User selects text in a message
2. "💬 Comment" button appears inline
3. Comment badge appears below message with count
4. Click badge to expand drawer inline (accordion style)
5. Drawer expands smoothly with all comments
6. Click header or chevron to collapse
7. Stays within message flow - no layout shift
8. Smooth 200ms transition

## Breakpoint Logic

```typescript
const isCondensed = 
  availableWidth < breakpoint ||  // Narrow screen
  (condenseWithSidecar && hasSidecar);  // Sidecar open
```

**Default Breakpoint**: 1200px
- Below this width, always condense
- Above this width, condense only if sidecar is open

## Sidecar Detection

The system polls `UnifiedSidecarContext` every 500ms to detect:
- Web viewers
- File viewers
- Document editors
- Localhost viewers
- App installers
- Diff viewers

When any sidecar is active, comments automatically condense to preserve reading space.

## Styling

### Badge Styling
- Uses app design tokens (`background-default`, `text-prominent`, `border-subtle`)
- Consistent with app's design language
- Hover state with scale effect
- Shows resolved count with checkmark
- Hover preview tooltip

### Drawer Styling
- Expands inline with smooth height transition
- Max height: 600px with scrollable content
- Uses app design tokens for consistency
- Collapsible header with chevron indicator
- Rounded corners and subtle border
- Dark mode support via design tokens
- No layout shift - stays in message flow

## Future Enhancements

### Planned
- [ ] Smooth transitions between modes
- [ ] Manual toggle override
- [ ] Persist user preference
- [ ] Animation when switching modes
- [ ] Improved badge positioning algorithm

### Potential
- [ ] Tablet-optimized breakpoint (768px)
- [ ] Mobile-first design (<640px)
- [ ] Swipe gestures on mobile
- [ ] Floating action button for comments
- [ ] Inline preview on badge hover

## Testing Checklist

- [ ] Test with no sidecars (should be full mode on wide screens)
- [ ] Test with web viewer sidecar (should condense)
- [ ] Test with file viewer sidecar (should condense)
- [ ] Test window resize from wide to narrow
- [ ] Test opening/closing sidecar
- [ ] Test multiple comments on one message
- [ ] Test creating comment in each mode
- [ ] Test replying in modal
- [ ] Test resolving comments
- [ ] Test keyboard navigation in modal
- [ ] Test Escape key to close modal
- [ ] Test click outside to close modal
- [ ] Test dark mode appearance

## Performance Considerations

### Optimizations
- ResizeObserver for efficient width monitoring
- Polling interval (500ms) for sidecar state
- Memoized display mode calculation
- Lazy modal rendering (only when open)

### Potential Issues
- Polling overhead (consider event-based approach)
- Multiple ResizeObservers (one per message)
- Modal re-rendering on state changes

### Recommendations
- Consider debouncing resize events
- Add event emitter to UnifiedSidecarContext
- Memoize expensive calculations
- Use React.memo for badge/modal components

## Accessibility

### Current
- Keyboard navigation (Esc to close)
- Focus management in modal
- Semantic HTML structure
- Color contrast compliance

### TODO
- ARIA labels for badge
- ARIA live regions for updates
- Screen reader announcements
- Keyboard shortcuts documentation
- Focus trap in modal

## Browser Compatibility

- **ResizeObserver**: Modern browsers (IE11 requires polyfill)
- **CSS Grid/Flexbox**: All modern browsers
- **Backdrop filter**: Safari 9+, Chrome 76+, Firefox 103+

## Migration Notes

### Breaking Changes
None - fully backward compatible

### New Props
- `MessageComments`: Added optional `displayMode` prop
- `GooseMessage`: No new props (internal changes only)

### Deprecations
None

## Code Examples

### Using the Hook
```typescript
import { useCommentDisplayMode } from '../hooks/useCommentDisplayMode';

function MyComponent() {
  const containerRef = useRef<HTMLDivElement>(null);
  const { displayMode, isCondensed } = useCommentDisplayMode({
    containerRef,
    breakpoint: 1200,
  });
  
  return (
    <div ref={containerRef}>
      {isCondensed ? <CompactView /> : <FullView />}
    </div>
  );
}
```

### Customizing Breakpoint
```typescript
// Use a narrower breakpoint for earlier condensing
const { displayMode } = useCommentDisplayMode({
  breakpoint: 1024,
  condenseWithSidecar: true,
});
```

### Disabling Sidecar Detection
```typescript
// Only condense based on width
const { displayMode } = useCommentDisplayMode({
  breakpoint: 1200,
  condenseWithSidecar: false,
});
```

## Related Files

- `ui/desktop/src/hooks/useCommentDisplayMode.ts` - Display mode detection
- `ui/desktop/src/components/CommentBadge.tsx` - Condensed badge UI
- `ui/desktop/src/components/CommentDrawer.tsx` - Expandable inline drawer
- `ui/desktop/src/components/MessageComments.tsx` - Mode switching logic
- `ui/desktop/src/components/GooseMessage.tsx` - Integration point
- `ui/desktop/src/contexts/UnifiedSidecarContext.tsx` - Sidecar state
- `ui/desktop/src/types/comment.ts` - Comment type definitions

## Questions?

For questions or issues, check:
1. This documentation
2. Component source code comments
3. Type definitions in `comment.ts`
4. Existing comment system documentation
