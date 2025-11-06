/**
 * Document Tools Extension for Goose
 * 
 * This extension provides tools for Goose to interact with collaborative documents.
 * It uses the Frontend extension type, which means tools are executed by the Electron
 * frontend rather than by an external MCP server.
 */

/**
 * Tool: document_view
 * 
 * Allows Goose to read the current content of a document.
 */
const documentViewTool = {
  name: 'document_view',
  description: 'Read the current content of a collaborative document. Use this to see what the user has written in a document before making suggestions or edits.',
  inputSchema: {
    type: 'object',
    properties: {
      doc_id: {
        type: 'string',
        description: 'The unique identifier of the document to view (e.g., "doc-12345")',
      },
    },
    required: ['doc_id'],
  },
};

/**
 * Tool: document_edit
 * 
 * Allows Goose to make edits to a document.
 */
const documentEditTool = {
  name: 'document_edit',
  description: 'Edit a collaborative document by inserting, replacing, or appending text. Use this to make changes to the document based on user requests. Always view the document first before editing to understand the context.',
  inputSchema: {
    type: 'object',
    properties: {
      doc_id: {
        type: 'string',
        description: 'The unique identifier of the document to edit',
      },
      action: {
        type: 'string',
        enum: ['insertText', 'replaceText', 'appendText', 'clear'],
        description: 'The type of edit to perform',
      },
      params: {
        type: 'object',
        description: 'Action-specific parameters',
        properties: {
          text: {
            type: 'string',
            description: 'The text to insert, replace, or append',
          },
          position: {
            type: 'number',
            description: 'Position to insert at (for insertText)',
          },
          from: {
            type: 'number',
            description: 'Start position (for replaceText)',
          },
          to: {
            type: 'number',
            description: 'End position (for replaceText)',
          },
        },
      },
    },
    required: ['doc_id', 'action'],
  },
};

/**
 * Tool: document_format
 * 
 * Allows Goose to apply formatting to text in a document.
 */
const documentFormatTool = {
  name: 'document_format',
  description: 'Apply formatting to text in a document. Use this to make text bold, italic, convert to headings, create lists, etc. Specify the range of text to format using from/to positions.',
  inputSchema: {
    type: 'object',
    properties: {
      doc_id: {
        type: 'string',
        description: 'The unique identifier of the document',
      },
      from: {
        type: 'number',
        description: 'Start position of the text to format',
      },
      to: {
        type: 'number',
        description: 'End position of the text to format',
      },
      format: {
        type: 'string',
        enum: [
          'bold',
          'italic',
          'underline',
          'strikethrough',
          'heading1',
          'heading2',
          'heading3',
          'bulletList',
          'orderedList',
          'code',
          'codeBlock',
          'blockquote',
        ],
        description: 'The formatting to apply',
      },
    },
    required: ['doc_id', 'from', 'to', 'format'],
  },
};

/**
 * Tool: list_documents
 * 
 * Allows Goose to see all currently open documents.
 */
const listDocumentsTool = {
  name: 'list_documents',
  description: 'List all currently open collaborative documents. Use this to see what documents are available before viewing or editing them.',
  inputSchema: {
    type: 'object',
    properties: {},
    required: [],
  },
};

/**
 * All document tools
 */
export const documentTools = [
  documentViewTool,
  documentEditTool,
  documentFormatTool,
  listDocumentsTool,
];

/**
 * Extension configuration for document tools
 */
export const documentToolsExtension = {
  type: 'frontend' as const,
  name: 'document-tools',
  tools: documentTools,
  instructions: `
# Document Tools

These tools allow you to interact with collaborative documents that the user is working on.

## Workflow

1. **List documents** using \`list_documents\` to see what documents are available
2. **View a document** using \`document_view\` to see its current content
3. **Edit the document** using \`document_edit\` to make changes
4. **Format text** using \`document_format\` to apply styling

## Best Practices

- Always view a document before editing it to understand the context
- When the user asks for help with a document, use document_view to see what they have
- Make edits that are relevant to the user's request
- Use formatting to make the document more readable (headings, lists, etc.)
- If you make a mistake, you can use document_edit with action="clear" to start over

## Example Usage

User: "I need help writing a blog post about React hooks"

1. Use \`document_view\` to see what they have so far
2. Use \`document_edit\` with action="appendText" to add an outline
3. Use \`document_format\` to make the outline items into a bullet list
4. Use \`document_format\` to make the title a heading1

User: "Make the first paragraph bold"

1. Use \`document_view\` to see the document and find the first paragraph
2. Note the from/to positions of the first paragraph
3. Use \`document_format\` with format="bold" to make it bold
`.trim(),
};

/**
 * Execute a document tool
 * 
 * This function is called when Goose uses one of the document tools.
 * It handles the tool execution by calling the appropriate IPC methods.
 */
export async function executeDocumentTool(
  toolName: string,
  args: Record<string, any>
): Promise<{ content: Array<{ type: string; text: string }> }> {
  console.log('[DocumentTools] Executing tool:', toolName, 'with args:', args);

  try {
    switch (toolName) {
      case 'document_view': {
        const { doc_id } = args;

        if (!doc_id || typeof doc_id !== 'string') {
          throw new Error('doc_id is required and must be a string');
        }

        // Get document state from Electron main process
        const state = await window.electron.ipcRenderer.invoke('get-document-state', doc_id);

        if (!state) {
          return {
            content: [
              {
                type: 'text',
                text: `Error: Document with ID "${doc_id}" not found. Use list_documents to see available documents.`,
              },
            ],
          };
        }

        // Return document content in a format Goose can understand
        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(
                {
                  docId: state.docId,
                  content: state.content,
                  plainText: state.plainText,
                  selection: state.selection,
                  timestamp: state.timestamp,
                  lastModified: state.lastModified,
                },
                null,
                2
              ),
            },
          ],
        };
      }

      case 'document_edit': {
        const { doc_id, action, params } = args;

        if (!doc_id || typeof doc_id !== 'string') {
          throw new Error('doc_id is required and must be a string');
        }

        if (!action || typeof action !== 'string') {
          throw new Error('action is required and must be a string');
        }

        // Send edit command to Electron main process
        window.electron.ipcRenderer.send('execute-document-edit', {
          docId: doc_id,
          action,
          params: params || {},
        });

        return {
          content: [
            {
              type: 'text',
              text: `Successfully executed ${action} on document ${doc_id}`,
            },
          ],
        };
      }

      case 'document_format': {
        const { doc_id, from, to, format } = args;

        if (!doc_id || typeof doc_id !== 'string') {
          throw new Error('doc_id is required and must be a string');
        }

        if (typeof from !== 'number' || typeof to !== 'number') {
          throw new Error('from and to must be numbers');
        }

        if (!format || typeof format !== 'string') {
          throw new Error('format is required and must be a string');
        }

        // Send format command to Electron main process
        window.electron.ipcRenderer.send('execute-document-edit', {
          docId: doc_id,
          action: 'formatText',
          params: { from, to, format },
        });

        return {
          content: [
            {
              type: 'text',
              text: `Successfully applied ${format} formatting to document ${doc_id} from position ${from} to ${to}`,
            },
          ],
        };
      }

      case 'list_documents': {
        // Get list of documents from Electron main process
        const documents = await window.electron.ipcRenderer.invoke('list-documents');

        if (!documents || documents.length === 0) {
          return {
            content: [
              {
                type: 'text',
                text: 'No documents are currently open.',
              },
            ],
          };
        }

        return {
          content: [
            {
              type: 'text',
              text: JSON.stringify(documents, null, 2),
            },
          ],
        };
      }

      default:
        throw new Error(`Unknown tool: ${toolName}`);
    }
  } catch (error) {
    console.error('[DocumentTools] Error executing tool:', error);
    return {
      content: [
        {
          type: 'text',
          text: `Error: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
  }
}
