/// <reference lib="dom" />
import { UIResourceRenderer as McpUIResourceRenderer } from '@mcp-ui/client';
import { Content } from '../types/message';
import { useState } from 'react';
import React from 'react';

// Define Resource type based on MCP SDK structure
interface Resource {
  name: string;
  uri: string;
  description?: string;
  mimeType?: string;
  text?: string;
  data?: string;
  [key: string]: unknown;
}

// Define UI Action types
interface UIAction {
  type: 'tool' | 'intent' | 'prompt' | 'notification' | 'link';
  payload: {
    toolName?: string;
    params?: Record<string, unknown>;
    intent?: string;
    prompt?: string;
    message?: string;
    url?: string;
  };
}

interface UIResourceRendererProps {
  resource: Resource;
  onUIAction?: (action: UIAction) => Promise<Record<string, unknown>>;
  className?: string;
}

// Main UIResourceRenderer component
export function UIResourceRenderer({
  resource,
  onUIAction,
  className = ''
}: UIResourceRendererProps) {
  console.log('=== UIResourceRenderer called ===');
  console.log('Resource:', resource);

  // SMART DETECTION: Override mimeType if content looks like Remote DOM JavaScript
  let effectiveResource = { ...resource };
  if (resource.text) {
    const isRemoteDOM = (
      resource.text.includes('// Remote DOM') || 
      resource.text.includes('DOM Product Catalog') ||
      resource.text.includes('document.createElement') ||
      resource.text.includes('container.setAttribute') ||
      resource.text.includes('const container =') ||
      resource.text.includes('productData =') ||
      resource.text.startsWith('// Remote DOM') ||
      resource.text.includes('Product Catalog Component')
    );
    
    if (isRemoteDOM) {
      console.log('Detected Remote DOM content, overriding mimeType to application/vnd.mcp-ui.remote-dom');
      effectiveResource = {
        ...resource,
        mimeType: 'application/vnd.mcp-ui.remote-dom'
      };
    }
  }

  const [error, setError] = useState<string | null>(null);

  console.log('Effective mimeType after smart detection:', effectiveResource.mimeType);
  
  if (!effectiveResource.uri && !effectiveResource.mimeType) {
    console.log('ERROR: No URI or mimeType found');
    return <div className="text-red-500">Invalid UI resource: missing URI or mimeType</div>;
  }

  const handleUIAction = async (action: UIAction): Promise<Record<string, unknown>> => {
    console.log('UI Action received:', action);

    // Handle different action types
    if (action.type === 'tool') {
      console.log('Tool call from UI:', action.payload.toolName, action.payload.params);
      if (onUIAction) {
        return await onUIAction(action);
      }
      return { status: 'handled' };
    }

    if (action.type === 'intent') {
      console.log('Intent from UI:', action.payload.intent, action.payload.params);
      if (onUIAction) {
        return await onUIAction(action);
      }
      return { status: 'handled' };
    }

    if (action.type === 'prompt') {
      console.log('Prompt from UI:', action.payload.prompt);
      if (onUIAction) {
        return await onUIAction(action);
      }
      return { status: 'handled' };
    }

    if (action.type === 'notification') {
      console.log('Notification from UI:', action.payload.message);
      return { status: 'handled' };
    }

    if (action.type === 'link') {
      console.log('Link from UI:', action.payload.url);
      if (action.payload.url) {
        window.electron.openInChrome(action.payload.url);
      }
      return { status: 'handled' };
    }

    return { status: 'unhandled' };
  };

  // Use the MCP-UI client library for most rendering, but handle Remote DOM ourselves
  console.log('RENDERING: Determining render method for mimeType:', effectiveResource.mimeType);
  
  // Handle Remote DOM specifically since the library doesn't seem to support it properly
  if (effectiveResource.mimeType === 'application/vnd.mcp-ui.remote-dom' || 
      effectiveResource.mimeType === 'application/vnd.mcp-ui.remote-dom+javascript') {
    console.log('RENDERING: Using custom Remote DOM renderer');
    
    if (!effectiveResource.text) {
      return (
        <div className={`ui-resource-renderer ${className} p-4 border border-borderSubtle rounded-lg`}>
          <div className="text-red-500">No content available for Remote DOM resource</div>
        </div>
      );
    }

    // Create wrapper HTML that executes the remote DOM script
    const wrapperHTML = `
      <!DOCTYPE html>
      <html>
      <head>
        <meta charset="utf-8">
        <title>MCP Remote DOM</title>
        <style>
          body { 
            margin: 0; 
            padding: 16px; 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
          }
        </style>
      </head>
      <body>
        <div id="root"></div>
        <script>
          // Setup communication with parent
          function sendUIAction(type, payload) {
            window.parent.postMessage({ type, payload }, '*');
          }
          
          // Make sendUIAction globally available
          window.sendUIAction = sendUIAction;
          
          // Execute the remote DOM script
          try {
            ${effectiveResource.text}
          } catch (error) {
            console.error('Remote DOM execution error:', error);
            document.getElementById('root').innerHTML = 
              '<div style="color: red; padding: 16px;">Error executing remote DOM script: ' + error.message + '</div>';
          }
        </script>
      </body>
      </html>
    `;

    // Setup postMessage listener for this iframe
    const setupPostMessageListener = (iframe: HTMLIFrameElement) => {
      const messageHandler = (event: MessageEvent) => {
        // Security: Check if message came from our iframe
        if (event.source !== iframe.contentWindow) return;

        console.log('Received postMessage from Remote DOM iframe:', event.data);

        // Handle different message types
        if (event.data && typeof event.data === 'object') {
          handleUIAction(event.data as UIAction);
        }
      };

      window.addEventListener('message', messageHandler);
      
      // Note: In a real React component, this would be in useEffect cleanup
      return () => {
        window.removeEventListener('message', messageHandler);
      };
    };

    return (
      <div className={`ui-resource-renderer ${className}`}>
        <iframe
          srcDoc={wrapperHTML}
          className="w-full h-96 border border-borderSubtle rounded-lg"
          sandbox="allow-scripts allow-forms allow-popups allow-modals"
          onLoad={(e) => {
            console.log('Remote DOM iframe loaded successfully');
            setupPostMessageListener(e.currentTarget);
          }}
          onError={(e: React.SyntheticEvent<HTMLIFrameElement, Event>) => {
            console.error('Remote DOM iframe failed to load:', e);
            setError('Failed to load Remote DOM content');
          }}
        />
        {error && <div className="text-red-500 mt-2">Error: {error}</div>}
      </div>
    );
  }

  // Use the MCP-UI client library for other mimeTypes
  console.log('RENDERING: Using @mcp-ui/client library for mimeType:', effectiveResource.mimeType);
  try {
    return (
      <div className={`ui-resource-renderer ${className}`}>
        <McpUIResourceRenderer
          resource={effectiveResource}
          onUIAction={handleUIAction}
          htmlProps={{
            iframeProps: {
              title: effectiveResource.name || 'MCP UI Resource',
              className: 'w-full h-96 border border-borderSubtle rounded-lg',
              onLoad: () => console.log('MCP UI iframe loaded successfully'),
              onError: (e: React.SyntheticEvent<HTMLIFrameElement, Event>) => {
                console.error('MCP UI iframe failed to load:', e);
                setError('Failed to load UI content');
              }
            },
          }}
        />
        {error && <div className="text-red-500 mt-2">Error: {error}</div>}
      </div>
    );
  } catch (e) {
    console.error('Failed to render with MCP-UI client:', e);
    console.log('RENDERING: Text fallback');
    return (
      <div className={`ui-resource-renderer ${className} p-4 border border-borderSubtle rounded-lg`}>
        <h3 className="font-semibold text-lg mb-2">{effectiveResource.name || 'MCP UI Resource'}</h3>
        <p className="text-sm text-gray-600 mb-2">URI: {effectiveResource.uri}</p>
        <p className="text-sm text-gray-600 mb-2">Type: {effectiveResource.mimeType}</p>
        {effectiveResource.text && (
          <pre className="text-xs bg-gray-100 p-2 rounded overflow-auto max-h-40">
            {effectiveResource.text.substring(0, 500)}
            {effectiveResource.text.length > 500 && '...'}
          </pre>
        )}
        {error && <div className="text-red-500 mt-2">Error: {error}</div>}
      </div>
    );
  }
}

// Helper function to check if content contains a UI resource
export function isUIResource(content: Content): boolean {
  console.log('=== isUIResource called ===');
  console.log('Call stack:', new Error().stack);
  console.log('isUIResource check for content:', content);
  
  // Check if the content is a resource type (MCP-UI resource)
  if (content.type === 'resource') {
    console.log('Found resource type content:', content);
    
    // The resource might have nested structure from MCP response
    const actualResource = (content as any).resource || content;
    
    console.log('Resource details:', {
      uri: actualResource.uri,
      mimeType: actualResource.mimeType,
      text: actualResource.text?.substring(0, 100) + '...',
      name: actualResource.name,
      type: actualResource.type
    });
    
    const hasUIScheme = actualResource.uri?.startsWith('ui://') || false;
    const hasMimeType = actualResource.mimeType !== undefined;
    const hasUIContent = (actualResource.text?.includes('ui://') || actualResource.text?.includes('Remote DOM')) || false;
    
    console.log('Resource validation:', { hasUIScheme, hasMimeType, hasUIContent });
    
    // A resource is a UI resource if:
    // 1. It has a ui:// URI scheme, OR
    // 2. It has a mimeType that indicates UI content, OR  
    // 3. It contains UI-related content in the text
    const isUIResource = hasUIScheme || hasMimeType || hasUIContent;
    console.log('Is UI resource (resource type):', isUIResource);
    return isUIResource;
  }
  
  // Check if the content is a text content that contains a UI resource
  if (content.type === 'text' && content.text) {
    console.log('Checking text content:', content.text);
    try {
      // Try to parse the text as JSON to see if it's a resource object
      const parsed = JSON.parse(content.text);
      console.log('Parsed JSON:', parsed);
      const isResource = (
        parsed &&
        typeof parsed === 'object' &&
        parsed.uri &&
        parsed.uri.startsWith('ui://') &&
        parsed.mimeType &&
        (parsed.mimeType === 'text/html' ||
          parsed.mimeType === 'text/uri-list' ||
          parsed.mimeType === 'application/vnd.mcp-ui.remote-dom' ||
          parsed.mimeType === 'application/vnd.mcp-ui.remote-dom+javascript' ||
          parsed.mimeType.startsWith('application/vnd.mcp-ui.'))
      );
      console.log('Is UI resource (JSON path):', isResource);
      return isResource;
    } catch {
      // Not JSON, check if it looks like a resource response
      const fallbackCheck = content.text.includes('ui://') && content.text.includes('mimeType');
      console.log('Is UI resource (fallback path):', fallbackCheck);
      return fallbackCheck;
    }
  }

  console.log('Not a text content or resource type');
  return false;
}

// Helper function to extract UI resource from content
export function extractUIResource(content: Content): Resource | null {
  console.log('=== extractUIResource called ===');
  console.log('Input content:', content);
  
  // Handle ResourceContent type directly
  if (content.type === 'resource') {
    console.log('Processing resource type content');
    
    // The resource might have nested structure from MCP response
    const actualResource = (content as any).resource || content;
    
    console.log('Actual resource to process:', actualResource);
    console.log('Resource URI:', actualResource.uri);
    console.log('Resource mimeType:', actualResource.mimeType);
    
    const hasValidURI = actualResource.uri && actualResource.uri.startsWith('ui://');
    const hasMimeType = actualResource.mimeType !== undefined;
    
    console.log('Validation checks:', { hasValidURI, hasMimeType });
    console.log('actualResource.uri:', actualResource.uri);
    console.log('actualResource.mimeType:', actualResource.mimeType);
    console.log('actualResource.text:', actualResource.text?.substring(0, 200));
    
    if (hasValidURI || hasMimeType) {
      const extractedResource = {
        uri: actualResource.uri,
        mimeType: actualResource.mimeType || 'text/html',
        text: actualResource.text || '',
        name: actualResource.name || 'MCP UI Resource'
      } as Resource;
      
      console.log('Successfully extracted resource:', extractedResource);
      return extractedResource;
    } else {
      console.log('Resource validation failed, returning null');
      console.log('Failed validation details:', {
        uri: actualResource.uri,
        mimeType: actualResource.mimeType,
        hasValidURI,
        hasMimeType
      });
      return null;
    }
  }

  // Handle TextContent that contains JSON resource
  if (content.type === 'text' && content.text) {
    console.log('Processing text content for embedded JSON resource');
    try {
      const parsed = JSON.parse(content.text);
      if (parsed && typeof parsed === 'object' && parsed.uri && parsed.mimeType) {
        console.log('Found JSON resource in text:', parsed);
        return parsed as Resource;
      }
    } catch (e) {
      console.log('Text content is not valid JSON, treating as plain text');
      // Not JSON, treat as plain text
    }
    return null;
  }

  console.log('Content type not supported for UI resource extraction:', content.type);
  return null;
}
