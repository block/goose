# Channels Breadcrumb Dropdown Feature

## Overview
Enhanced the `SpaceBreadcrumb` component to include interactive dropdown menus for both Spaces and Rooms, allowing users to quickly switch between channels without leaving the chat view.

## Features Implemented

### 1. **Space Dropdown** 
- Click on the Space name to see all available Spaces
- Shows Space icon, name, and member count
- Current Space is highlighted with an accent indicator
- Smooth animations with Framer Motion

### 2. **Room Dropdown**
- Click on the Room name to see all rooms in the current Space
- Shows room icon, name, topic (if available), and member count
- Current room is highlighted with an accent indicator
- Smooth animations with Framer Motion

### 3. **Visual Improvements**
- Changed Space icon from `Hash` to `Layers` for better hierarchy
- Added `ChevronDown` icons that rotate when dropdowns are open
- Hover states with background color changes
- Click-outside-to-close functionality

### 4. **Accessibility**
- Proper ARIA labels and roles
- Keyboard navigation support
- Screen reader friendly
- Semantic HTML with `<nav>` element

## Component Structure

```
SpaceBreadcrumb
├── Space Dropdown Button
│   ├── Layers Icon
│   ├── Space Name
│   └── ChevronDown Icon
├── Separator (ChevronRight)
└── Room Dropdown Button
    ├── Hash Icon
    ├── Room Name
    └── ChevronDown Icon
```

## Usage

### Basic Usage (Read-only)
```tsx
<SpaceBreadcrumb roomId={matrixRoomId} />
```

### With Navigation Handlers
```tsx
<SpaceBreadcrumb 
  roomId={matrixRoomId}
  onRoomClick={(roomId) => {
    // Switch to the selected room
    openMatrixChat(roomId, userId, roomName);
  }}
  onSpaceClick={(spaceId) => {
    // Navigate to the Space view
    navigate(`/channels?space=${spaceId}`);
  }}
/>
```

## Integration Points

### TabbedChatContainer.tsx
The breadcrumb is integrated into the chat container with full navigation support:

```tsx
<SpaceBreadcrumb 
  roomId={activeTabState.tab.matrixRoomId}
  onRoomClick={handleRoomSwitch}
  onSpaceClick={handleSpaceNavigation}
/>
```

**Room Switching:**
- When a user selects a different room from the dropdown
- The current tab switches to that room's chat
- Uses `openMatrixChat()` from TabContext
- Preserves chat history and state

**Space Navigation:**
- Currently logs the space selection
- Can be extended to navigate to ChannelsView or SpaceRoomsView
- Placeholder for future implementation

## Dropdown Behavior

### Space Dropdown
- Lists all available Spaces from `matrixService.getSpaces()`
- Shows member count for each Space
- Highlights current Space with accent color and indicator dot
- Closes on selection or click outside

### Room Dropdown
- Lists all rooms in the current Space from `matrixService.getSpaceChildren()`
- Shows room topic if available, otherwise shows member count
- Highlights current room with accent color and indicator dot
- Closes on selection or click outside

## Styling

### Theme-Aware
- Uses CSS custom properties for colors
- Supports light and dark modes
- Consistent with app design system

### Responsive
- Dropdown widths: Space (256px), Room (288px)
- Max height with scroll: 320px
- Smooth animations (150ms)
- Proper z-index layering (z-50)

## State Management

### Component State
- `showSpaceDropdown`: Controls Space dropdown visibility
- `showRoomDropdown`: Controls Room dropdown visibility
- `allSpaces`: List of all available Spaces
- `roomsInCurrentSpace`: List of rooms in the current Space
- `breadcrumb`: Current Space and Room information

### Event Listeners
- Matrix events: `spaceChildAdded`, `spaceChildRemoved`, `ready`
- Mouse events: Click outside to close dropdowns
- Automatic updates when Space structure changes

## Performance Considerations

### Current Implementation
- Loads all Spaces on mount
- Loads rooms for current Space only
- Re-fetches on Matrix events
- O(n) space lookup (iterates through all Spaces)

### Future Optimizations
Consider adding to MatrixService:
```typescript
// Cache room-to-space mapping
private roomToSpaceMap: Map<string, string> = new Map();

getRoomParentSpace(roomId: string): Space | null {
  const spaceId = this.roomToSpaceMap.get(roomId);
  return spaceId ? this.getSpace(spaceId) : null;
}
```

## Testing Checklist

- [ ] Dropdown opens on click
- [ ] Dropdown closes on outside click
- [ ] Dropdown closes on item selection
- [ ] Current Space/Room is highlighted
- [ ] Room switching works correctly
- [ ] Space navigation works (when implemented)
- [ ] Animations are smooth
- [ ] Works in light and dark mode
- [ ] Keyboard navigation works
- [ ] Screen reader announces correctly
- [ ] Handles empty states (no Spaces/Rooms)
- [ ] Updates when Matrix structure changes

## Future Enhancements

1. **Search/Filter**
   - Add search input in dropdown
   - Filter Spaces/Rooms by name

2. **Recent Rooms**
   - Show recently accessed rooms at the top
   - Persist in localStorage

3. **Favorites**
   - Star/favorite specific rooms
   - Show favorites section in dropdown

4. **Unread Indicators**
   - Show unread message counts
   - Highlight rooms with mentions

5. **Room Actions**
   - Right-click context menu
   - Quick actions (mute, leave, settings)

6. **Performance**
   - Implement room-to-space mapping cache
   - Lazy load room details
   - Virtual scrolling for large lists

7. **Space Navigation**
   - Complete the `handleSpaceNavigation` implementation
   - Navigate to SpaceRoomsView or ChannelsView
   - Show Space overview/settings

## Files Modified

1. **SpaceBreadcrumb.tsx**
   - Added dropdown state management
   - Added Space and Room dropdown UI
   - Added click-outside handlers
   - Enhanced accessibility

2. **TabbedChatContainer.tsx**
   - Added `handleRoomSwitch` function
   - Added `handleSpaceNavigation` placeholder
   - Passed handlers to SpaceBreadcrumb
   - Integrated with TabContext

## Dependencies

- `react`: State and effects
- `framer-motion`: Animations
- `lucide-react`: Icons
- `matrixService`: Space and Room data
- `TabContext`: Navigation functions

## API Reference

### SpaceBreadcrumb Props

```typescript
interface SpaceBreadcrumbProps {
  roomId: string;                          // Required: Current room ID
  className?: string;                      // Optional: Additional CSS classes
  onSpaceClick?: (spaceId: string) => void; // Optional: Space selection handler
  onRoomClick?: (roomId: string) => void;   // Optional: Room selection handler
}
```

### Space Interface

```typescript
interface Space {
  roomId: string;      // Matrix room ID for the Space
  name: string;        // Display name
  memberCount: number; // Number of members
}
```

### Room Interface

```typescript
interface Room {
  roomId: string;      // Matrix room ID
  name: string;        // Display name
  topic?: string;      // Optional room topic/description
  memberCount: number; // Number of members
}
```

## Example Screenshots

### Closed State
```
[Layers Icon] My Workspace > [Hash Icon] general
```

### Space Dropdown Open
```
[Layers Icon] My Workspace ▼ > [Hash Icon] general
┌─────────────────────────────┐
│ [Layers] My Workspace    ● │ ← Current
│          12 members        │
│ [Layers] Design Team       │
│          8 members         │
│ [Layers] Engineering       │
│          24 members        │
└─────────────────────────────┘
```

### Room Dropdown Open
```
[Layers Icon] My Workspace > [Hash Icon] general ▼
                            ┌──────────────────────────────┐
                            │ [Hash] general            ● │ ← Current
                            │        Team discussions     │
                            │ [Hash] announcements        │
                            │        Important updates    │
                            │ [Hash] random               │
                            │        12 members           │
                            └──────────────────────────────┘
```

## Conclusion

The enhanced breadcrumb provides a seamless way for users to navigate between Spaces and Rooms without leaving their current conversation. The dropdown UI is intuitive, accessible, and follows the app's design patterns.
