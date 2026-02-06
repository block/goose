# Multiple message listeners accumulate causing duplicate event handling

## Description

When using `AppRenderer`, multiple `message` event listeners accumulate on `window`, causing each postMessage to be received multiple times. This results in:
- Console spam with "Ignoring message from unknown source" errors
- Performance degradation from redundant event processing

## Steps to Reproduce

1. Render an `AppRenderer` component with callback props (`onCallTool`, `onMessage`, etc.) instead of a `client`
2. Trigger a message from the guest iframe (e.g., click a button that calls `tools/call`)
3. Observe multiple console messages for a single action

## Expected Behavior

One message listener should handle each postMessage event.

## Actual Behavior

Multiple listeners fire for the same event. Checking `getEventListeners(window).message` in DevTools shows 5+ listeners accumulating:

```javascript
getEventListeners(window).message
// Returns array of 5 listeners when there should be 1
```

Console output for a single button click:
```
Ignoring message from unknown source  MessageEvent {...}
Ignoring message from unknown source  MessageEvent {...}
Ignoring message from unknown source  MessageEvent {...}
Parsed message {jsonrpc: '2.0', id: 17, method: 'tools/call', params: {...}}
Ignoring message from unknown source  MessageEvent {...}
```

## Root Cause Analysis

The issue appears to be in the lifecycle management of `PostMessageTransport` instances:

1. `AppRenderer` creates a new `AppBridge` in a `useEffect` 
2. `AppFrame` calls `appBridge.connect(new PostMessageTransport(...))` which adds a `window.addEventListener("message", ...)`
3. When props change or re-renders occur, new instances are created but `transport.close()` is not called to remove old listeners

The `PostMessageTransport.close()` method exists in `@modelcontextprotocol/ext-apps` and correctly removes the listener, but it doesn't appear to be called during component cleanup.

## Environment

- `@mcp-ui/client`: 6.0.0
- `@modelcontextprotocol/ext-apps`: 0.3.1
- React: 19.2.4
- Platform: Electron desktop app

## Suggested Fix

Ensure `useEffect` cleanup functions call `transport.close()` or `appBridge.close()` before creating new instances:

```javascript
useEffect(() => {
  const bridge = new AppBridge(...);
  // ... setup ...
  
  return () => {
    bridge.close();  // Clean up old transport/listeners
  };
}, [dependencies]);
```
