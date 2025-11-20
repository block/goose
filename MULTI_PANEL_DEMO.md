# Multi-Panel Sidecar System Demo

## Overview
The new multi-panel sidecar system allows you to have multiple sidecar panels in different layouts within each chat tab. When you have 2 or more sidecar views active, the system automatically switches to the multi-panel mode.

## Features Implemented

### 1. **Automatic Layout Switching**
- **Single Panel**: When only 1 sidecar view is active, uses the original `TabSidecar` component
- **Multi-Panel**: When 2+ sidecar views are active, switches to `MultiPanelTabSidecar` component

### 2. **Layout Modes**
- **Columns**: Two panels side by side (default for 2 panels)
- **Rows**: Two panels stacked vertically  
- **Grid**: 2x2 grid layout (default for 3+ panels)
- **Custom**: Flexible positioning (future enhancement)

### 3. **Panel Management**
- Individual panel headers with close buttons
- Layout switching controls in the main header
- Responsive sizing based on content
- Smooth animations between layout changes

### 4. **Content Types Supported**
- **Diff Viewer**: Code diffs with unified/split view toggle
- **Web Browser**: External websites and web apps
- **Localhost Viewer**: Local development servers
- **File Viewer**: File content display
- **Document Editor**: Rich text editing

## How to Test

### 1. **Open Multiple Sidecar Views**
In a chat, use goose to open multiple sidecar views:

```
Hey goose, can you:
1. Show me a diff of some file changes
2. Open a localhost viewer for port 3000
3. Open a web browser to google.com
```

### 2. **Layout Switching**
Once you have 2+ panels:
- Click the column/row/grid icons in the header to change layouts
- Use the dropdown menu for additional options
- Close individual panels using the X button on each panel

### 3. **Panel Features**
- Each panel maintains its own state
- Diff viewers have unified/split toggle
- Web browsers are fully functional
- File viewers show content appropriately

## Technical Implementation

### Key Components
- `MultiPanelTabSidecar.tsx`: Main multi-panel component
- `TabbedChatContainer.tsx`: Updated to conditionally use multi-panel
- `TabContext.tsx`: Manages sidecar state per tab

### Layout Logic
```typescript
// Auto-switch based on active view count
if (activeViews.length <= 1) {
  setLayoutMode('single');
} else if (activeViews.length === 2 && layoutMode === 'single') {
  setLayoutMode('columns');
} else if (activeViews.length > 2 && (layoutMode === 'single' || layoutMode === 'columns')) {
  setLayoutMode('grid');
}
```

### Rendering Strategy
```typescript
// Conditional rendering in TabbedChatContainer
sidecarState && sidecarState.activeViews.length > 1 ? (
  <MultiPanelTabSidecar {...props} />
) : (
  <TabSidecar {...props} />
)
```

## Future Enhancements

### Phase 2 (Planned)
- [ ] Drag and drop panel reordering
- [ ] Individual panel resizing
- [ ] Panel splitting/merging
- [ ] Context menus for panel actions

### Phase 3 (Planned)  
- [ ] Keyboard shortcuts for layout switching
- [ ] Better memory management and cleanup
- [ ] Performance optimizations

### Phase 4 (Planned)
- [ ] Layout persistence per tab
- [ ] Custom layout templates
- [ ] Visual drop zones for drag operations
- [ ] Animation improvements

## Benefits

1. **Better Productivity**: View multiple tools simultaneously
2. **Flexible Layouts**: Arrange panels to suit your workflow  
3. **Backward Compatible**: Single panels work exactly as before
4. **Responsive**: Adapts to different screen sizes and content
5. **Extensible**: Easy to add new panel types and layouts

## Usage Examples

### Development Workflow
- **Column 1**: Code diff viewer
- **Column 2**: Localhost server preview
- Perfect for reviewing changes while seeing live results

### Research Workflow  
- **Top Row**: Web browser with documentation
- **Bottom Row**: File viewer with notes
- Great for referencing docs while working with files

### Multi-Tool Workflow
- **2x2 Grid**: Diff viewer, web browser, file viewer, document editor
- Ultimate productivity setup for complex tasks

The system automatically handles layout switching, content rendering, and state management, making it seamless to work with multiple tools in your sidecar space.
