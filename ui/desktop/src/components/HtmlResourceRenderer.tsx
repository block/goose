import { HtmlResource } from '@mcp-ui/client';
import { ResourceContent } from '../types/message';

interface HtmlResourceRendererProps {
  content: ResourceContent;
}

export default function HtmlResourceRenderer({ content }: HtmlResourceRendererProps) {
  const { resource } = content;

  // Check if this is a UI resource that should be rendered as HTML
  if (!resource.uri.startsWith('ui://')) {
    return null;
  }

  return (
    <div className="my-4 border border-borderSubtle rounded-lg overflow-hidden">
      <div className="bg-bgSubtle px-3 py-2 border-b border-borderSubtle">
        <p className="text-xs text-textSubtle font-medium">HTML Resource</p>
        <p className="text-xs text-textSubtle truncate">{resource.uri}</p>
      </div>
      <div className="p-4">
        <HtmlResource resource={resource} />
      </div>
    </div>
  );
}
