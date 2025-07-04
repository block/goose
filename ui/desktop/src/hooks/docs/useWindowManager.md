# useWindowManager Hook

The `useWindowManager` hook provides functionality for dynamically resizing the application window and managing component mounting states based on window expansion/collapse operations.

## Overview

This hook is designed to handle window resizing operations in Electron applications, particularly for implementing expandable UI panels or sidebars. It manages the window state, handles transitions, and coordinates component mounting/unmounting during resize operations.

## Usage

```typescript
import { useWindowManager } from '../hooks/useWindowManager';

function MyComponent() {
  const { windowState, toggleWindow, isComponentMounted, canExpand } = useWindowManager({
    expandPercentage: 50,
    maxWidthForExpansion: 900,
    transitionDuration: 300
  });

  return (
    <div>
      <button onClick={toggleWindow} disabled={!canExpand}>
        {windowState.isExpanded ? 'Collapse' : 'Expand'} Window
      </button>
      
      {isComponentMounted && (
        <div>Content that appears when window is expanded</div>
      )}
    </div>
  );
}
```

## API Reference

### Parameters

The hook accepts an optional `WindowManagerOptions` object:

```typescript
interface WindowManagerOptions {
  expandPercentage?: number;      // Default: 50
  transitionDuration?: number;    // Default: 300
  maxWidthForExpansion?: number;  // Default: 900
}
```

#### Options Details

- **`expandPercentage`** (optional, default: `50`)
  - The percentage by which to expand the window width
  - Type: `number`
  - Example: `50` means expand by 50% of current width

- **`transitionDuration`** (optional, default: `300`)
  - Duration of the transition animation in milliseconds
  - Type: `number`
  - Currently used for timing component mounting/unmounting

- **`maxWidthForExpansion`** (optional, default: `900`)
  - Maximum window width at which expansion is allowed
  - Type: `number`
  - Windows wider than this value will not be resized but components can still be mounted

### Return Value

The hook returns a `WindowManagerHook` object:

```typescript
interface WindowManagerHook {
  windowState: WindowState;
  toggleWindow: () => Promise<void>;
  isComponentMounted: boolean;
  canExpand: boolean;
}
```

#### Return Value Details

- **`windowState`**: Current window state information
  ```typescript
  interface WindowState {
    isExpanded: boolean;      // Whether window is currently expanded
    originalWidth: number;    // Original window width before expansion
    currentWidth: number;     // Current window width
    isTransitioning: boolean; // Whether a resize operation is in progress
  }
  ```

- **`toggleWindow`**: Async function to toggle window expansion/collapse
  - Returns: `Promise<void>`
  - Handles the complete resize operation including component mounting/unmounting

- **`isComponentMounted`**: Boolean indicating if components should be rendered
  - `true` when window is expanded and transition is complete
  - `false` when window is collapsed or during transitions

- **`canExpand`**: Boolean indicating if window can be expanded
  - Based on current window width vs `maxWidthForExpansion` option
  - `false` if current width exceeds the maximum allowed width

## Behavior Details

### Expansion Process

1. **Pre-expansion**: Component is unmounted, transition state is set
2. **Resize**: Window width is increased by the specified percentage
3. **Post-expansion**: Component is mounted after a small delay (50ms)
4. **State update**: Window state reflects the new expanded state

### Collapse Process

1. **Pre-collapse**: Component is immediately unmounted
2. **Delay**: Small delay (100ms) to allow clean unmounting
3. **Resize**: Window is resized back to original width
4. **State update**: Window state reflects the collapsed state

### Window Resize Handling

The hook automatically handles external window resize events:
- Updates current width when window is manually resized
- Preserves original width when not in expanded state
- Prevents conflicts with programmatic resize operations

## Integration with Electron

The hook relies on Electron's main process API for window resizing:

```typescript
// Expected Electron API
window.electron.resizeWindow(expandPercentage: number): Promise<boolean>
```

- **Parameter**: `expandPercentage` - percentage to expand (0 to reset to original size)
- **Returns**: Promise resolving to `boolean` indicating success/failure

## Error Handling

The hook includes comprehensive error handling:

- **Concurrent Operations**: Prevents multiple simultaneous resize operations
- **Resize Failures**: Handles failed resize operations gracefully
- **State Recovery**: Resets to consistent state on errors
- **Component State**: Ensures component mounting state remains consistent

## Example: Diff Panel Integration

```typescript
// Example from App.tsx showing improved integration with a diff panel
function App() {
  const { toggleWindow, windowState, isComponentMounted, canExpand } = useWindowManager({
    expandPercentage: 50,
    maxWidthForExpansion: 900,
  });

  const [isDiffPanelOpen, setIsDiffPanelOpen] = useState(false);

  useEffect(() => {
    const handleToggleDiffViewer = async () => {
      // Prevent action if window is already transitioning
      if (windowState.isTransitioning) {
        console.log('Window is already transitioning, ignoring diff viewer toggle');
        return;
      }

      const currentDiffContent = window.pendingDiffContent;
      const diffContentMatches = currentDiffContent === diffSidePanelContent;

      setDiffSidePanelContent(currentDiffContent || '');

      try {
        if (!isDiffPanelOpen) {
          setIsDiffPanelOpen(true);
          await toggleWindow(); // Expand window for diff panel
        } else if (diffContentMatches) {
          setIsDiffPanelOpen(false);
          await toggleWindow(); // Collapse window
        }
      } catch (error) {
        console.error('Failed to toggle window for diff viewer:', error);
        // Revert UI state on error
        setIsDiffPanelOpen(!isDiffPanelOpen);
      }

      // Clear the pending diff content
      window.pendingDiffContent = undefined;
    };

    window.addEventListener('toggle-diff-viewer', handleToggleDiffViewer);
    return () => window.removeEventListener('toggle-diff-viewer', handleToggleDiffViewer);
  }, [isDiffPanelOpen, diffSidePanelContent, toggleWindow, windowState.isTransitioning]);

  return (
    <div>
      {/* Main content */}
      {isDiffPanelOpen && (
        <DiffSidePanel
          onClose={async () => {
            // Prevent action if window is already transitioning
            if (windowState.isTransitioning) {
              console.log('Window is transitioning, cannot close diff panel now');
              return;
            }

            try {
              setIsDiffPanelOpen(false);
              await toggleWindow();
            } catch (error) {
              console.error('Failed to toggle window when closing diff panel:', error);
              // Revert state on error
              setIsDiffPanelOpen(true);
            }
          }}
        />
      )}
    </div>
  );
}
```

## Best Practices

### 1. Component Mounting

Always use `isComponentMounted` to conditionally render expensive components:

```typescript
// ✅ Good
{isComponentMounted && <ExpensiveComponent />}

// ❌ Avoid - component always rendered
<ExpensiveComponent style={{ display: isComponentMounted ? 'block' : 'none' }} />
```

### 2. Disable Actions During Transitions

Prevent user actions during transitions:

```typescript
<button 
  onClick={toggleWindow} 
  disabled={windowState.isTransitioning || !canExpand}
>
  Toggle Window
</button>
```

### 3. Handle Async Operations

Always handle the promise returned by `toggleWindow`:

```typescript
const handleToggle = async () => {
  // Prevent action if window is already transitioning
  if (windowState.isTransitioning) {
    console.log('Window is already transitioning, ignoring toggle request');
    return;
  }

  try {
    await toggleWindow();
    // Additional logic after successful toggle
  } catch (error) {
    console.error('Failed to toggle window:', error);
    // Handle error appropriately - revert UI state if needed
    setComponentState(previousState);
  }
};
```

### 4. Check Transition State

Always check if the window is transitioning before calling `toggleWindow`:

```typescript
// ✅ Good - Check transition state
const handleAction = async () => {
  if (windowState.isTransitioning) {
    console.log('Window is transitioning, skipping action');
    return;
  }
  await toggleWindow();
};

// ❌ Avoid - No transition check
const handleAction = async () => {
  await toggleWindow(); // May conflict with ongoing transitions
};
```

### 5. Responsive Design

Consider the `maxWidthForExpansion` limit for responsive behavior:

```typescript
const { canExpand } = useWindowManager({ maxWidthForExpansion: 1200 });

// Provide alternative UI for non-expandable windows
{!canExpand && <AlternativeLayout />}
```

## Troubleshooting

### Common Issues

1. **Window not resizing**: Ensure Electron's `resizeWindow` API is properly implemented
2. **Component flickering**: Check that `isComponentMounted` is used correctly
3. **State inconsistencies**: Verify error handling in resize operations
4. **Performance issues**: Use `isComponentMounted` to prevent unnecessary renders

### Debug Information

The hook logs important operations to the console:
- Resize operation status
- Error conditions
- State transitions

Enable console logging to troubleshoot issues:

```typescript
// Check browser console for debug information
console.log('Window state:', windowState);
console.log('Can expand:', canExpand);
console.log('Component mounted:', isComponentMounted);
```

## Dependencies

- React hooks: `useState`, `useEffect`, `useCallback`, `useRef`
- Electron renderer process APIs
- Window resize event listeners

## Version History

- **Initial Implementation**: Basic window resize functionality
- **Component Mounting**: Added intelligent component mounting/unmounting
- **Error Handling**: Enhanced error handling and state recovery
- **Responsive Behavior**: Added `maxWidthForExpansion` and `canExpand` logic
