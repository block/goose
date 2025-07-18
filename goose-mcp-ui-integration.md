# MCP UI Integration in Goose

Implementation of interactive web components in AI agents using the Model Context Protocol (MCP) UI specification.

## Overview

The Model Context Protocol (MCP) enables AI agents to communicate with external tools and services. The MCP UI specification extends this to support interactive web components instead of static text responses.

### Before
- Plain text responses
- Static JSON data
- Limited user interaction

### After
- Interactive web components
- Rich user interfaces
- Dynamic content rendering

## Implementation

Extended Codename Goose to support MCP UI rendering capabilities.

**Platform**: Goose (TypeScript frontend + Rust backend)  
**Integration**: `@mcp-ui/client` library  
**Rendering**: Iframe-based component rendering  
**Security**: Sandboxed execution environment

## Technical Details

### Core Components

1. **UIResourceRenderer Component**
   - Wraps `@mcp-ui/client` with Goose styling
   - Handles UI actions (tool calls, intents, prompts, links)
   - Provides secure iframe rendering

2. **Resource Detection Logic**
   - Identifies UI resources in MCP server responses
   - Automatically switches between text and UI rendering
   - Maintains backward compatibility

3. **Integration Points**
   - Modified `ToolCallWithResponse` component
   - Added UI resource detection to message handling
   - Implemented action handling framework

### Key Implementation

```typescript
// Resource detection
export function isUIResource(content: Content): boolean {
  // Detects ui:// URIs and mimeType
}

// UI action handling
const handleUIAction = async (action: any) => {
  if (action.type === 'tool') {
    // Execute MCP tool calls from UI interactions
  }
  // Handle intents, prompts, notifications, links
};
```

### Files Modified
- `ui/desktop/package.json` - Added `@mcp-ui/client` dependency
- `ui/desktop/src/components/UIResourceRenderer.tsx` - New UI rendering component
- `ui/desktop/src/components/ToolCallWithResponse.tsx` - Modified for UI support

## Capabilities

### Current Features
- UI resource detection from MCP server responses
- Interactive component rendering in desktop app
- UI action processing (clicks, form submissions, tool calls)
- Secure iframe sandboxing
- Backward compatibility with existing text/image responses

### Technical Benefits
- Standards-compliant implementation
- Zero breaking changes to existing functionality
- Extensible framework for advanced UI components
- Proper security isolation

## Usage

The implementation automatically detects UI resources in MCP server responses and renders them as interactive components. No configuration required - existing Goose functionality remains unchanged.

## Development

### For MCP Server Developers
1. Study the MCP UI specification
2. Create interactive web components using the MCP UI format
3. Test with this Goose client implementation
4. Validate UI resource generation and action handling

### For Client Developers
1. Fork this implementation as a starting point
2. Extend with custom UI patterns
3. Add application-specific interaction handlers
4. Build on the existing component framework

## Status

Production-ready implementation. Successfully integrates MCP UI rendering into Goose desktop application with full backward compatibility.
