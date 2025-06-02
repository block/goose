import { ExternalLink } from 'lucide-react';
import { QUICKSTART_GUIDE_URL } from '../constants';

interface ProviderSetupHeaderProps {
  title: string;
  body: string;
}

/**
 * Renders the header (title + description + link to guide) for the modal.
 */
export default function ProviderSetupHeader({ title, body }: ProviderSetupHeaderProps) {
  return (
    <div className="text-center">
      <h2 className="text-xl font-medium text-textStandard mb-3">{title}</h2>
      <div className="text-lg text-gray-400 font-light mb-4">{body}</div>
      <a
        href={QUICKSTART_GUIDE_URL}
        target="_blank"
        rel="noopener noreferrer"
        className="flex items-center justify-center text-textProminent text-sm"
      >
        <ExternalLink size={16} className="mr-1" />
        View quick start guide
      </a>
    </div>
  );
}
