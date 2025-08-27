# Add Auto-Compact Threshold Configuration to Desktop UI

## Summary

This PR introduces a configurable auto-compact threshold feature to the Goose desktop application, allowing users to set and visualize when conversation compaction will be triggered. The implementation includes both backend API endpoints for configuration management and a comprehensive UI update with visual indicators and inline editing capabilities.

## Changes Overview

### Backend Changes
- **Config Management API**: Extended the configuration management routes to support getting and setting the `GOOSE_AUTO_COMPACT_THRESHOLD` configuration value
- **OpenAPI Updates**: Updated OpenAPI specifications to include the new configuration endpoints

### Frontend Changes
- **Threshold Configuration**: Added ability to fetch, display, and update the auto-compact threshold directly from the chat interface
- **Visual Progress Indicator**: Enhanced the alert system to show conversation length progress with color-coded indicators (green → yellow → orange → red)
- **Threshold Marker**: Visual marker on the progress bar showing where auto-compaction will trigger
- **Inline Editing**: Users can click on the threshold percentage to edit it directly without leaving the chat interface

## Technical Implementation

### Modified Files

#### Backend
- `crates/goose-server/src/routes/config_management.rs` - Added endpoints for getting/setting configuration values
- `crates/goose-server/src/openapi.rs` - Updated OpenAPI generation
- `crates/goose-mcp/src/developer/mod.rs` - Minor adjustments

#### Frontend  
- `ui/desktop/src/components/ChatInput.tsx` - Integrated threshold loading and auto-compact trigger logic
- `ui/desktop/src/components/alerts/AlertBox.tsx` - Major enhancements for threshold visualization and editing
- `ui/desktop/src/components/alerts/types.ts` - Added `autoCompactThreshold` property to Alert type
- `ui/desktop/src/components/bottom_menu/BottomMenuAlertPopover.tsx` - Improved click handling for inline editing
- `ui/desktop/src/api/sdk.gen.ts` & `types.gen.ts` - Generated API client updates

### Key Features

1. **Dynamic Threshold Loading**: The threshold is loaded from the backend configuration on component mount
2. **Real-time Updates**: Changes to the threshold are immediately reflected in the UI and persisted to the backend
3. **Visual Feedback**: 
   - Progress bar with 30 dots showing conversation length
   - Color progression: green (0-50%) → yellow (51-75%) → orange (76-90%) → red (91-100%)
   - Larger dot indicator at the threshold position
4. **User Experience**:
   - Click-to-edit functionality on the threshold percentage
   - Input validation (0-100 range)
   - Automatic save on blur or Enter key
   - Visual disabled state when threshold is set to 0

## API Changes

### New Endpoints
- `GET /config?key={key}` - Retrieve a configuration value
- `PUT /config` - Update a configuration value

### Response Format
```json
{
  "key": "GOOSE_AUTO_COMPACT_THRESHOLD",
  "value": 75
}
```

## Testing Considerations

- Verify threshold persistence across application restarts
- Test boundary values (0, 100, invalid inputs)
- Ensure auto-compaction triggers at the configured threshold
- Validate UI updates when threshold is changed
- Test click-outside behavior doesn't interfere with editing

## Migration Notes

No database migrations required. The configuration is stored in the existing configuration system.

## Breaking Changes

None - this feature is additive and maintains backward compatibility.

## Related Issues

This PR addresses the need for user-configurable conversation compaction thresholds to better manage token usage and conversation context in long-running sessions.

## Screenshots/Demo

The UI now displays:
- A progress indicator showing current conversation length
- A visual marker for the auto-compact threshold
- Inline editing capability for quick threshold adjustments
- Color-coded feedback based on conversation length percentage
