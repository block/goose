# Multi-Chat Visual Design

## Layout Overview

```
┌─────────────────────────────────────────────────────────────────┐
│ [≡] [Chat 1 ×] [Chat 2 ×] [Chat 3 ×] [New Session ×] [+]  [≡] │ ← Tab Bar
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│                                                                 │
│                     Active Chat Content                         │
│                     (BaseChat2 Component)                       │
│                                                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Tab States

### Active Tab
```
┌─────────────────┐
│ ⋮⋮ Chat Name  × │  ← Drag handle, name, close button
└─────────────────┘
        ▔▔▔▔▔▔▔▔▔    ← Accent color indicator
```
- Background: `bg-background-default`
- Text: `text-text-default`
- Bottom border: `bg-background-accent` (2px)
- Close button: Always visible

### Inactive Tab
```
┌─────────────────┐
│    Chat Name  × │  ← No drag handle (until hover)
└─────────────────┘
```
- Background: `bg-background-muted`
- Text: `text-text-muted`
- Hover: `bg-background-medium`
- Close button: Visible on hover

### Tab with Unread
```
┌─────────────────┐
│ ● Chat Name   × │  ← Blue dot indicator
└─────────────────┘
```
- Blue dot: `bg-blue-500` (8px diameter)
- Position: Left side, vertically centered

### Loading Tab
```
┌─────────────────┐
│ ⋮⋮ Loading... × │  ← Pulsing animation
└─────────────────┘
```
- Text: `animate-pulse`
- Placeholder name: "Loading..."

## Tab Bar Features

### Overflow Scrolling
```
┌──────────────────────────────────────────────────────┐
│ [<] [Tab 1] [Tab 2] [Tab 3] [Tab 4] [Tab 5] [>] [+] │
│      └─────────────────────────────────┘             │
│           Scrollable area                            │
└──────────────────────────────────────────────────────┘
```
- Scroll buttons: `[<]` and `[>]` appear when tabs overflow
- Smooth scrolling: `scroll-smooth`
- Hidden scrollbar: `scrollbar-hide`

### New Tab Button
```
┌───┐
│ + │  ← Always visible, right-aligned
└───┘
```
- Icon: `Plus` from lucide-react
- Position: Fixed right side
- Border: Left border separator

## Interaction States

### Drag and Drop
```
┌─────────────────┐     ┌─────────────────┐
│ ⋮⋮ Chat 1     × │ ──→ │ ⋮⋮ Chat 2     × │
└─────────────────┘     └─────────────────┘
     Dragging                Drop target
     (opacity: 0.5)          (highlight)
```

### Hover State
```
┌─────────────────┐
│ ⋮⋮ Chat Name  × │  ← Drag handle appears
└─────────────────┘     Close button visible
     ↑ Hover
```

## Empty State

```
┌─────────────────────────────────────────────┐
│                                             │
│                    ┌───┐                    │
│                    │ + │                    │
│                    └───┘                    │
│                                             │
│              No chat open                   │
│                                             │
│      Create a new chat to get started       │
│                                             │
│            [  New Chat  ]                   │
│                                             │
└─────────────────────────────────────────────┘
```

## Color Palette

### Light Mode
- **Tab Bar Background**: `#f4f6f7` (neutral-50)
- **Active Tab**: `#ffffff` (white)
- **Inactive Tab**: `#f4f6f7` (neutral-50)
- **Inactive Tab Hover**: `#e3e6ea` (neutral-100)
- **Text Active**: `#3f434b` (neutral-800)
- **Text Inactive**: `#878787` (neutral-400)
- **Accent Indicator**: `#32353b` (neutral-900)
- **Unread Dot**: `#5c98f9` (blue-500)
- **Border**: `#e3e6ea` (neutral-100)

### Dark Mode
- **Tab Bar Background**: `#3f434b` (neutral-800)
- **Active Tab**: `#22252a` (neutral-950)
- **Inactive Tab**: `#3f434b` (neutral-800)
- **Inactive Tab Hover**: `#474e57` (neutral-700)
- **Text Active**: `#ffffff` (white)
- **Text Inactive**: `#878787` (neutral-400)
- **Accent Indicator**: `#ffffff` (white)
- **Unread Dot**: `#7cacff` (blue-100)
- **Border**: `#32353b` (neutral-900)

## Typography

- **Tab Label**: 14px (text-sm), truncated at 20 chars
- **Font**: Cash Sans (system default)
- **Weight**: 400 (normal)

## Spacing

- **Tab Padding**: 16px horizontal, 10px vertical
- **Tab Gap**: 0px (tabs touch)
- **Icon Size**: 14px (w-3.5 h-3.5)
- **Drag Handle**: 12px (w-3 h-3)
- **Tab Bar Height**: Auto (based on content, ~42px)

## Animations

### Tab Switch
- Duration: 150ms
- Easing: `ease-in-out`
- Properties: `background-color`, `color`

### Drag Handle Fade
- Duration: 150ms
- Easing: `ease-in-out`
- Property: `opacity` (0 → 1)

### Close Button Fade
- Duration: 150ms
- Easing: `ease-in-out`
- Property: `opacity` (0 → 1)

### Loading Pulse
- Duration: 2s
- Easing: `cubic-bezier(0.4, 0, 0.6, 1)`
- Property: `opacity` (1 → 0.5 → 1)

### Scroll
- Duration: 300ms
- Easing: `smooth`
- Property: `scrollLeft`

## Responsive Behavior

### Desktop (≥1024px)
- Tab min-width: 140px
- Tab max-width: 200px
- Visible tabs: ~6-8 (depending on screen width)

### Tablet (768px - 1023px)
- Tab min-width: 140px
- Tab max-width: 180px
- Visible tabs: ~4-5

### Mobile (<768px)
- Tab min-width: 120px
- Tab max-width: 160px
- Visible tabs: ~2-3
- Scroll buttons always visible

## Accessibility

### Keyboard Navigation
```
Tab         → Focus next tab
Shift+Tab   → Focus previous tab
Enter       → Activate focused tab
Escape      → Close focused tab (with confirmation)
Cmd/Ctrl+W  → Close active tab
Cmd/Ctrl+T  → New tab
Cmd/Ctrl+1-9 → Switch to tab 1-9
```

### Screen Reader Announcements
- "Chat 1, active tab, 1 of 3"
- "Chat 2, inactive tab, 2 of 3"
- "New chat button"
- "Close tab button"

### Focus Indicators
- Outline: 2px solid accent color
- Offset: 2px
- Border radius: 4px

## Comparison with Browser Tabs

### Similar to Chrome/Firefox
✅ Horizontal tab bar
✅ Close button on each tab
✅ New tab button
✅ Drag-and-drop reordering
✅ Keyboard shortcuts
✅ Active tab highlighting

### Different from Browsers
❌ No favicon (uses icon instead)
❌ No tab preview on hover (future feature)
❌ No tab audio indicator
❌ No tab groups (future feature)
❌ No pinned tabs (future feature)

## Visual Examples

### 3 Tabs Open
```
┌───────────────────────────────────────────────────────┐
│ [Chat 1 ×] [Chat 2 ×] [Chat 3 ×]                 [+] │
│     ▔▔▔▔▔▔                                            │
└───────────────────────────────────────────────────────┘
```

### 10 Tabs Open (Max)
```
┌───────────────────────────────────────────────────────┐
│ [<] [1×][2×][3×][4×][5×][6×][7×][8×][9×][10×] [>] [+]│
└───────────────────────────────────────────────────────┘
```

### Drag in Progress
```
┌───────────────────────────────────────────────────────┐
│ [Chat 1 ×] [Chat 3 ×] ┌─────────┐              [+]   │
│                       │ Chat 2  │ ← Dragging          │
│                       └─────────┘                     │
└───────────────────────────────────────────────────────┘
```

## Implementation Notes

### CSS Classes Used
- `bg-background-muted` - Tab bar background
- `bg-background-default` - Active tab
- `bg-background-medium` - Hover state
- `text-text-default` - Active text
- `text-text-muted` - Inactive text
- `border-border-default` - Borders
- `bg-background-accent` - Active indicator
- `transition-all` - Smooth transitions
- `duration-150` - Animation timing
- `scrollbar-hide` - Hide scrollbar

### Tailwind Utilities
- `flex` - Flexbox layout
- `items-center` - Vertical centering
- `gap-2` - Spacing between elements
- `px-4 py-2.5` - Padding
- `min-w-[140px]` - Minimum width
- `max-w-[200px]` - Maximum width
- `truncate` - Text truncation
- `rounded` - Border radius
- `hover:bg-*` - Hover states

## Future Design Enhancements

### Tab Groups
```
┌───────────────────────────────────────────────────────┐
│ Work: [Chat 1 ×] [Chat 2 ×] | Personal: [Chat 3 ×]   │
│       └──────────────────┘              └────────┘    │
│         Blue group                      Green group   │
└───────────────────────────────────────────────────────┘
```

### Tab Preview
```
┌─────────────┐
│ Chat Name × │
└─────────────┘
      ↓ Hover
┌─────────────────────┐
│  Preview Thumbnail  │
│  ┌───────────────┐  │
│  │ Last message  │  │
│  │ preview...    │  │
│  └───────────────┘  │
└─────────────────────┘
```

### Pinned Tabs
```
┌───────────────────────────────────────────────────────┐
│ [📌 Chat 1] [📌 Chat 2] │ [Chat 3 ×] [Chat 4 ×]  [+] │
│  └──────────────────┘      └────────────────────┘     │
│    Pinned (no close)         Regular tabs             │
└───────────────────────────────────────────────────────┘
```
