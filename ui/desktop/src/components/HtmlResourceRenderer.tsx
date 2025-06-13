import { HtmlResource, type UiActionResult } from '@mcp-ui/client';
import { ResourceContent } from '../types/message';
import { useState } from 'react';

interface HtmlResourceRendererProps {
  content: ResourceContent;
}

export default function HtmlResourceRenderer({ content }: HtmlResourceRendererProps) {
  const { resource } = content;
  const [minIframeHeight, setMinIframeHeight] = useState('50vh'); // Default minimum height for the iframe

  function handleUiActionTool(result: UiActionResult) {
    if (result.type === 'tool') {
      console.log('Tool action received:', result);
    }
  }

  function handleUiActionIntent(result: UiActionResult) {
    if (result.type === 'intent') {
      if (result.payload.intent === 'resizeIframe') {
        setMinIframeHeight((result.payload.params.minHeight as string) || '50vh');
      }
    }
  }

  async function handleUiAction(result: UiActionResult): Promise<{ status: string }> {
    // if the type is not a UiActionResult, return an error, return an error response
    if (!result || typeof result !== 'object' || !('type' in result)) {
      console.error('Invalid onUiAction result:', result);
      return { status: 'error' };
    }

    // Handle the UI action result based on its type
    switch (result.type) {
      case 'tool':
        handleUiActionTool(result);
        break;
      case 'prompt':
        break;
      case 'link':
        break;
      case 'intent':
        handleUiActionIntent(result);
        break;
      case 'notification':
        break;
      default:
        break;
    }
    return { status: 'ok' };
  }

  // Check if this is a UI resource that should be rendered as HTML
  if (!resource.uri.startsWith('ui://')) {
    return null;
  }

  return (
    <HtmlResource
      resource={resource}
      style={{ minHeight: minIframeHeight }}
      onUiAction={handleUiAction}
    />
  );
}
