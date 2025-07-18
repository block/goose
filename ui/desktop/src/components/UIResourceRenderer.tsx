/// <reference lib="dom" />
import { UIResourceRenderer as McpUIResourceRenderer } from '@mcp-ui/client';
import type { UIActionResult } from '@mcp-ui/client';
import { Content, getResourceText } from '../types/message';
import React from 'react';

// Resource interface compatible with @mcp-ui/client
export interface Resource {
  uri: string;
  mimeType: string;
  text?: string;
  blob?: string;
  name?: string;
  title?: string;
  description?: string;
  _meta?: { [x: string]: unknown };
  [x: string]: unknown; // Index signature for compatibility
}

interface UIResourceRendererProps {
  resource: Resource;
  onUIAction?: (action: UIActionResult) => Promise<unknown> | void;
  className?: string;
  supportedContentTypes?: ('rawHtml' | 'externalUrl' | 'remoteDom')[];
  htmlProps?: {
    style?: React.CSSProperties;
  };
  remoteDomProps?: Record<string, unknown>;
}

export function UIResourceRenderer({
  resource,
  onUIAction,
  className = '',
  supportedContentTypes,
  htmlProps,
  remoteDomProps,
}: UIResourceRendererProps) {
  console.log('=== UIResourceRenderer called ===');
  console.log('Raw resource object:', resource);
  console.log('Resource type:', typeof resource);
  console.log('Resource keys:', Object.keys(resource || {}));
  console.log('Resource.uri:', resource?.uri);
  console.log('Resource.uri type:', typeof resource?.uri);
  console.log('Resource.mimeType:', resource?.mimeType);
  console.log('Resource.mime_type:', (resource as Record<string, unknown>)?.mime_type);

  // Validate resource according to mcp-ui spec
  const mimeType = resource.mimeType || (resource as Record<string, unknown>).mime_type;
  const mimeTypeString = String(mimeType || 'unknown');

  if (!resource.uri || !mimeType) {
    console.error('❌ Invalid UI resource: missing uri or mimeType', {
      hasUri: !!resource.uri,
      uri: resource.uri,
      hasMimeType: !!mimeType,
      mimeType: mimeType,
      resourceKeys: Object.keys(resource || {}),
    });
    return <div className="text-red-500">Invalid UI resource: missing uri or mimeType</div>;
  }

  if (!resource.uri.startsWith('ui://')) {
    console.error('❌ Invalid UI resource: uri must start with ui://', {
      actualUri: resource.uri,
      startsWithUri: resource.uri.startsWith('uri://'),
      startsWithUi: resource.uri.startsWith('ui://'),
      uriLength: resource.uri.length,
      firstChars: resource.uri.substring(0, 10),
    });
    return <div className="text-red-500">Invalid UI resource: uri must start with ui://</div>;
  }

  const handleUIAction = async (action: UIActionResult): Promise<unknown> => {
    console.log('UI Action received:', action);

    if (onUIAction) {
      const result = await onUIAction(action);
      return result || { status: 'handled' };
    }

    // Default handling for common action types
    if (action.type === 'link' && 'url' in action.payload) {
      console.log('Opening link:', action.payload.url);
      if (window.electron?.openInChrome) {
        window.electron.openInChrome(action.payload.url as string);
      } else {
        window.open(action.payload.url as string, '_blank');
      }
      return { status: 'handled' };
    }

    return { status: 'unhandled' };
  };

  console.log('🔒 RENDERING: Using official @mcp-ui/client library for mimeType:', mimeTypeString);
  console.log('RENDERING: Resource details:', {
    uri: resource.uri,
    mimeType: mimeTypeString,
    hasText: !!resource.text,
    hasBlob: !!resource.blob,
    textLength: resource.text?.length,
    blobLength: resource.blob?.length,
  });

  try {
    return (
      <div className={`ui-resource-renderer ${className}`}>
        <McpUIResourceRenderer
          resource={resource}
          onUIAction={handleUIAction}
          supportedContentTypes={supportedContentTypes}
          htmlProps={htmlProps}
          remoteDomProps={remoteDomProps}
        />
      </div>
    );
  } catch (e) {
    console.error('Failed to render with @mcp-ui/client:', e);
    return (
      <div
        className={`ui-resource-renderer-fallback ${className} p-4 border border-red-300 rounded-lg bg-red-50`}
      >
        <h3 className="font-semibold text-lg mb-2 text-red-800">UI Resource Error</h3>
        <p className="text-sm text-red-600 mb-2">Failed to render UI resource</p>
        <p className="text-sm text-gray-600 mb-2">URI: {resource.uri}</p>
        <p className="text-sm text-gray-600 mb-2">Type: {mimeTypeString}</p>
        <p className="text-sm text-red-600">
          Error: {e instanceof Error ? e.message : 'Unknown error'}
        </p>
      </div>
    );
  }
}

// Helper function to check if content contains a UI resource (following mcp-ui spec)
export function isUIResource(content: Content): boolean {
  console.log('=== isUIResource called ===');
  console.log('Checking content:', content);

  // Handle resource type content (primary method)
  if (content.type === 'resource' && content.resource) {
    const resource = content.resource;

    // Ensure resource has required properties
    if (!resource || typeof resource !== 'object') {
      console.log('❌ Resource is not a valid object');
      return false;
    }

    // Must have ui:// scheme as per mcp-ui spec
    const hasUIScheme = Boolean(
      resource.uri && typeof resource.uri === 'string' && resource.uri.startsWith('ui://')
    );

    // Must have valid mimeType as per mcp-ui spec - handle both camelCase and snake_case
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const mimeType = (resource as any).mimeType || resource.mime_type;
    const hasValidMimeType = Boolean(
      mimeType &&
        typeof mimeType === 'string' &&
        (mimeType === 'text/html' ||
          mimeType === 'text/uri-list' ||
          mimeType.startsWith('application/vnd.mcp-ui.'))
    );

    const isValid = hasUIScheme && hasValidMimeType;
    console.log('isUIResource result (resource type):', {
      hasUIScheme,
      hasValidMimeType,
      isValid,
      uri: resource.uri,
      mimeType: mimeType,
    });

    return isValid;
  }

  // Handle text type content that might contain embedded UI resource (legacy fallback)
  if (content.type === 'text' && content.text) {
    console.log('Checking text content for embedded UI resource');
    try {
      // Try to parse the text as JSON to see if it's a resource object
      const parsed = JSON.parse(content.text);
      if (
        parsed &&
        typeof parsed === 'object' &&
        parsed.uri?.startsWith('ui://') &&
        parsed.mimeType &&
        (parsed.mimeType === 'text/html' ||
          parsed.mimeType === 'text/uri-list' ||
          parsed.mimeType.startsWith('application/vnd.mcp-ui.'))
      ) {
        console.log('Found valid UI resource in JSON text');
        return true;
      }
    } catch {
      // Not valid JSON, check for text patterns
      const hasUIPattern =
        content.text.includes('ui://') &&
        (content.text.includes('text/html') ||
          content.text.includes('text/uri-list') ||
          content.text.includes('application/vnd.mcp-ui.'));
      if (hasUIPattern) {
        console.log('Found UI resource pattern in text');
        return true;
      }
    }
  }

  console.log('❌ Not a UI resource');
  return false;
}

// Helper function to extract UI resource from content (following mcp-ui spec)
export function extractUIResource(content: Content): Resource | null {
  console.log('=== extractUIResource called ===');
  console.log('Input content:', content);

  // Handle resource type content (primary method)
  if (content.type === 'resource' && content.resource) {
    const resource = content.resource;
    console.log('Extracting from resource content:', resource);

    // Check if it's a valid UI resource according to mcp-ui spec
    if (isUIResource(content)) {
      // Safely extract text content from resource using type-safe helper
      const textContent = getResourceText(resource);
      const blobContent = 'blob' in resource ? resource.blob : undefined;

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const mimeType = (resource as any).mimeType || resource.mime_type;

      if (!resource.uri || !mimeType) {
        console.error('Resource missing required fields:', { uri: resource.uri, mimeType });
        return null;
      }

      const extractedResource: Resource = {
        uri: resource.uri,
        mimeType: mimeType,
        ...(textContent && { text: textContent }),
        ...(blobContent && { blob: blobContent }),
      };

      console.log('Successfully extracted resource:', extractedResource);
      return extractedResource;
    }
  }

  // Handle text type content that might contain embedded UI resource (legacy fallback)
  if (content.type === 'text' && content.text) {
    console.log('Checking text content for embedded UI resource');
    try {
      // Try to parse the text as JSON to see if it's a resource object
      const parsed = JSON.parse(content.text);
      if (
        parsed &&
        typeof parsed === 'object' &&
        parsed.uri?.startsWith('ui://') &&
        parsed.mimeType &&
        (parsed.mimeType === 'text/html' ||
          parsed.mimeType === 'text/uri-list' ||
          parsed.mimeType.startsWith('application/vnd.mcp-ui.'))
      ) {
        console.log('Successfully extracted resource from JSON text:', parsed);
        const extractedResource: Resource = {
          uri: parsed.uri,
          mimeType: parsed.mimeType,
          ...(parsed.text && { text: parsed.text }),
          ...(parsed.blob && { blob: parsed.blob }),
        };
        return extractedResource;
      }
    } catch {
      // Not valid JSON, skip
      console.log('Text content is not valid JSON, cannot extract resource');
    }
  }

  console.log('❌ No valid UI resource found');
  return null;
}
