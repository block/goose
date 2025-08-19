# Add Auto-Compact Threshold Configuration UI

## Summary

This PR implements a user interface for configuring the auto-compact threshold in the Goose desktop application. The feature allows users to dynamically adjust the threshold percentage that triggers automatic conversation compaction, providing better control over conversation management.

### Key Changes:
- **New Configuration UI**: Added an editable threshold slider in the AlertBox component
- **Backend Integration**: Implemented configuration management endpoints for persisting threshold values
- **Real-time Updates**: Added event-driven updates to ensure threshold changes are immediately reflected across components
- **Debugging Enhancements**: Added console logging for better visibility into threshold save operations

## Technical Implementation

### Frontend (TypeScript/React)
- **AlertBox Component**: Enhanced with an inline editor for the auto-compact threshold
  - Added percentage slider (0-100%) with real-time value display
  - Implemented save/cancel functionality with loading states
  - Added console logging for debugging threshold save operations
  - Dispatches custom events to notify other components of threshold changes

- **ChatInput Component**: 
  - Listens for `autoCompactThresholdChanged` events
  - Dynamically reloads threshold values without page refresh
  - Fetches initial threshold from backend configuration

- **BottomMenuAlertPopover**: Updated to properly display and handle auto-compact alerts

### Backend (Rust)
- **Configuration Management Routes** (`crates/goose-server/src/routes/config_management.rs`):
  - Added `/config/upsert` endpoint for updating configuration values
  - Supports storing threshold as decimal value (0.0 - 1.0)
  - Non-secret configuration storage for user preferences

- **OpenAPI Integration**:
  - Updated OpenAPI schema with new configuration endpoints
  - Generated TypeScript SDK for type-safe API calls

## Modified Files

### Frontend Files:
- `ui/desktop/src/components/alerts/AlertBox.tsx` - Main UI component for threshold editing
- `ui/desktop/src/components/ChatInput.tsx` - Integration with threshold configuration
- `ui/desktop/src/components/bottom_menu/BottomMenuAlertPopover.tsx` - Alert display improvements
- `ui/desktop/src/components/alerts/types.ts` - Type definitions for alerts
- `ui/desktop/src/api/sdk.gen.ts` - Generated SDK with config endpoints
- `ui/desktop/src/api/types.gen.ts` - Generated types for configuration
- `ui/desktop/openapi.json` - OpenAPI specification updates

### Backend Files:
- `crates/goose-server/src/routes/config_management.rs` - Configuration management implementation
- `crates/goose-server/src/openapi.rs` - OpenAPI route registration

### Documentation:
- `test_threshold_update.md` - Testing documentation for threshold updates

## Impact Analysis

- **User Experience**: Users can now easily adjust when conversations are automatically compacted
- **Performance**: No performance impact - configuration is loaded once and cached
- **Compatibility**: Backward compatible - defaults to existing behavior if no threshold is set
- **Storage**: Minimal impact - stores single configuration value per user

## Testing Approach

### Manual Testing:
1. Open the desktop application
2. Navigate to an auto-compact alert
3. Click the edit button to modify threshold
4. Adjust the slider to desired percentage
5. Click save and verify the value persists
6. Verify ChatInput component receives the updated threshold
7. Test cancel functionality to ensure changes are discarded

### Edge Cases Tested:
- Setting threshold to 0% (minimum)
- Setting threshold to 100% (maximum)
- Rapid slider movements
- Save/cancel during API calls
- Network failure handling

## Migration Steps

No migration required. The feature is additive and doesn't affect existing functionality.

## Breaking Changes

None. All changes are backward compatible.

## Related Issues

This PR addresses the need for user-configurable auto-compact thresholds, allowing users to customize when their conversations are automatically condensed based on their preferences and usage patterns.

## Next Steps

Future enhancements could include:
- Preset threshold options (Conservative, Balanced, Aggressive)
- Per-conversation threshold overrides
- Analytics on optimal threshold values based on usage patterns
