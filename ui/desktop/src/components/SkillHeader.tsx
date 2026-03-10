import { useState } from 'react';
import { Sparkles, ChevronDown, ChevronRight } from 'lucide-react';
import MarkdownContent from './MarkdownContent';

interface SkillHeaderProps {
  name: string;
  content: string;
}

export function SkillHeader({ name, content }: SkillHeaderProps) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="border-b border-border-primary">
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex items-center justify-between w-full px-4 py-2 hover:bg-background-secondary transition-colors"
      >
        <div className="flex items-center ml-6">
          <Sparkles className="w-3.5 h-3.5 text-purple-500 mr-2" />
          <span className="text-sm">
            <span className="text-text-secondary">Skill</span>{' '}
            <span className="text-text-primary">{name}</span>
          </span>
        </div>
        {expanded ? (
          <ChevronDown className="w-4 h-4 text-text-secondary" />
        ) : (
          <ChevronRight className="w-4 h-4 text-text-secondary" />
        )}
      </button>
      {expanded && (
        <div className="px-10 pb-3 text-sm">
          <MarkdownContent content={content} />
        </div>
      )}
    </div>
  );
}
