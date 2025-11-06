# Next Steps: Creating the edit_document Tool

## Overview

We've successfully implemented document context parsing and AI prompt enhancement. Now we need to create the `edit_document` tool that allows the AI to programmatically edit documents.

## Architecture

```
AI Model
    ↓ (calls tool)
edit_document Tool Handler (Rust)
    ↓ (sends IPC message)
Electron IPC Bridge (TypeScript)
    ↓ (executes JavaScript)
window.gooseEditors[docId].method()
    ↓ (updates)
Document Editor (React)
```

## Step 1: Find Tool Registration System

First, we need to understand how tools are registered in Goose:

```bash
cd /Users/spencermartin/Desktop/goose/crates/goose
rg "Tool" src/agents/ -A 3 | head -50
```

Look for:
- Tool trait or struct definitions
- Tool registration functions
- Existing tool implementations (e.g., file operations, shell commands)

## Step 2: Create edit_document Tool

### Tool Schema

The tool should accept these parameters:

```json
{
  "name": "edit_document",
  "description": "Edit a collaborative document that the user is working on",
  "parameters": {
    "type": "object",
    "properties": {
      "doc_id": {
        "type": "string",
        "description": "The document ID"
      },
      "action": {
        "type": "string",
        "enum": ["insertText", "replaceText", "appendText", "formatText", "clear"],
        "description": "The editing action to perform"
      },
      "text": {
        "type": "string",
        "description": "Text to insert/replace/append (required for insertText, replaceText, appendText)"
      },
      "position": {
        "type": "integer",
        "description": "Position to insert text (optional for insertText)"
      },
      "from": {
        "type": "integer",
        "description": "Start position for replace/format"
      },
      "to": {
        "type": "integer",
        "description": "End position for replace/format"
      },
      "format": {
        "type": "object",
        "description": "Formatting to apply (for formatText action)",
        "properties": {
          "bold": {"type": "boolean"},
          "italic": {"type": "boolean"},
          "underline": {"type": "boolean"},
          "color": {"type": "string"}
        }
      }
    },
    "required": ["doc_id", "action"]
  }
}
```

### Tool Implementation (Conceptual)

```rust
// In crates/goose/src/agents/document_tools.rs (new file)

use mcp_core::{Content, Tool, ToolCall, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct EditDocumentParams {
    doc_id: String,
    action: String,
    text: Option<String>,
    position: Option<usize>,
    from: Option<usize>,
    to: Option<usize>,
    format: Option<Value>,
}

pub struct EditDocumentTool;

impl Tool for EditDocumentTool {
    fn name(&self) -> &str {
        "edit_document"
    }

    fn description(&self) -> &str {
        "Edit a collaborative document that the user is working on"
    }

    fn parameters(&self) -> Value {
        // Return the JSON schema defined above
    }

    async fn execute(&self, params: Value) -> ToolResult<Vec<Content>> {
        let params: EditDocumentParams = serde_json::from_value(params)?;
        
        // TODO: Send IPC message to Electron
        // This is where we need the IPC bridge
        
        // For now, return a placeholder
        Ok(vec![Content::text(format!(
            "Would execute {} on document {}",
            params.action, params.doc_id
        ))])
    }
}
```

## Step 3: Create IPC Bridge in Electron

### Location
`ui/desktop/src/main/index.ts` (or wherever the main Electron process is)

### Implementation

```typescript
// In main process
import { ipcMain, BrowserWindow } from 'electron';

ipcMain.handle('execute-document-edit', async (event, docId: string, method: string, args: any[]) => {
  console.log(`Executing document edit: ${docId}.${method}(${JSON.stringify(args)})`);
  
  const window = BrowserWindow.getFocusedWindow();
  if (!window) {
    return { error: 'No focused window' };
  }

  try {
    const result = await window.webContents.executeJavaScript(`
      (function() {
        const editor = window.gooseEditors['${docId}'];
        if (!editor) {
          return { error: 'Editor not found: ${docId}' };
        }
        if (typeof editor.${method} !== 'function') {
          return { error: 'Method not found: ${method}' };
        }
        try {
          const result = editor.${method}(...${JSON.stringify(args)});
          return { success: true, result };
        } catch (e) {
          return { error: e.message };
        }
      })()
    `);
    return result;
  } catch (error) {
    console.error('Error executing document edit:', error);
    return { error: error.message };
  }
});
```

### Expose to Preload

```typescript
// In preload script
import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('electron', {
  // ... existing methods ...
  
  executeDocumentEdit: (docId: string, method: string, args: any[]) => 
    ipcRenderer.invoke('execute-document-edit', docId, method, args),
});
```

## Step 4: Connect Rust Tool to IPC

We need a way for the Rust backend to communicate with the Electron main process. This could be:

### Option A: HTTP Endpoint in Electron
Add an HTTP server in Electron that listens for document edit requests:

```typescript
// In Electron main process
import express from 'express';

const app = express();
app.use(express.json());

app.post('/document-edit', async (req, res) => {
  const { docId, method, args } = req.body;
  const result = await executeDocumentEdit(docId, method, args);
  res.json(result);
});

app.listen(3001, () => {
  console.log('Document edit server listening on port 3001');
});
```

Then in Rust:
```rust
async fn execute_document_edit(doc_id: &str, method: &str, args: Value) -> Result<Value> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3001/document-edit")
        .json(&json!({
            "docId": doc_id,
            "method": method,
            "args": args
        }))
        .send()
        .await?;
    
    Ok(response.json().await?)
}
```

### Option B: WebSocket Connection
Establish a WebSocket connection between Rust and Electron for real-time communication.

### Option C: Shared State File
Write edit requests to a file that Electron watches (less ideal, but simple).

## Step 5: Register the Tool

Once the tool is implemented, register it with the agent:

```rust
// In the agent initialization code
agent.register_tool(Box::new(EditDocumentTool));
```

## Step 6: Testing

### Test 1: Tool Registration
```bash
# Verify the tool is registered
# Check agent initialization logs for "Registered tool: edit_document"
```

### Test 2: Tool Execution (Mock)
```bash
# Test the tool with mock parameters
# Verify it returns expected results
```

### Test 3: IPC Bridge
```javascript
// In browser console
await window.electron.executeDocumentEdit('doc-123', 'appendText', ['Hello!']);
// Should append "Hello!" to the document
```

### Test 4: End-to-End
```bash
# 1. Open document
# 2. Type some content
# 3. Click "Ask Goose"
# 4. Send: "Add a heading that says 'Introduction'"
# 5. Verify:
#    - Backend receives document context ✓
#    - AI calls edit_document tool
#    - IPC bridge executes
#    - Document updates
#    - "Goose is editing..." shows
```

## 🎯 Success Criteria

- [ ] Tool is defined with correct schema
- [ ] Tool is registered with agent
- [ ] IPC bridge is created in Electron
- [ ] Rust can call IPC bridge
- [ ] IPC bridge can execute window.gooseEditors methods
- [ ] Document updates in real-time
- [ ] Visual feedback works
- [ ] Errors are handled gracefully

## 📚 Resources

- **MCP Tool Documentation**: Check `crates/mcp-core/src/tool.rs`
- **Existing Tools**: Look at `crates/goose/src/agents/` for examples
- **Electron IPC**: https://www.electronjs.org/docs/latest/api/ipc-main
- **WebContents.executeJavaScript**: https://www.electronjs.org/docs/latest/api/web-contents#contentsexecutejavascriptcode-usergesture

## 💡 Tips

1. **Start Simple**: Get the IPC bridge working first with a simple test
2. **Mock First**: Test the tool with mock data before connecting to IPC
3. **Log Everything**: Add extensive logging to debug the data flow
4. **Handle Errors**: Make sure errors are caught and reported properly
5. **Visual Feedback**: Use the existing "Goose is editing..." indicator

## 🚀 Ready to Continue?

The foundation is solid. We have:
- ✅ Frontend API (`window.gooseEditors`)
- ✅ Document context parsing
- ✅ AI prompt enhancement
- ⏳ Tool creation (next step)
- ⏳ IPC bridge (next step)

Once these final pieces are in place, users will be able to have Goose edit their documents in real-time!
