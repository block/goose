import React from 'react';
import { FolderTree, MessageSquare, Code } from 'lucide-react';
import { useWhiteLabel } from '../whitelabel/WhiteLabelContext';
import { InsightCards } from './sessions/InsightCards';

interface PopularChatTopicsProps {
  append: (text: string) => void;
}

interface DefaultTopic {
  id: string;
  icon: React.ReactNode;
  label: string;
  prompt: string;
}

const DEFAULT_TOPICS: DefaultTopic[] = [
  {
    id: 'organize-photos',
    icon: <FolderTree className="w-5 h-5" />,
    label: 'Organize the photos on my desktop into neat little folders by subject matter',
    prompt: 'Organize the photos on my desktop into neat little folders by subject matter',
  },
  {
    id: 'government-forms',
    icon: <MessageSquare className="w-5 h-5" />,
    label:
      'Describe in detail how various forms of government works and rank each by units of geese',
    prompt:
      'Describe in detail how various forms of government works and rank each by units of geese',
  },
  {
    id: 'tamagotchi-game',
    icon: <Code className="w-5 h-5" />,
    label: 'Develop a tamagotchi game that lives on my computer and follows a pixelated styling',
    prompt: 'Develop a tamagotchi game that lives on my computer and follows a pixelated styling',
  },
];

/** Simple list layout — default goose starter prompts */
function TopicList({
  topics,
  onSelect,
}: {
  topics: DefaultTopic[];
  onSelect: (prompt: string) => void;
}) {
  return (
    <div className="absolute bottom-0 left-0 p-6 max-w-md">
      <h3 className="text-text-secondary text-sm mb-1">Popular chat topics</h3>
      <div className="space-y-1">
        {topics.map((topic) => (
          <div
            key={topic.id}
            className="flex items-center justify-between py-1.5 hover:bg-background-secondary rounded-md cursor-pointer transition-colors"
            onClick={() => onSelect(topic.prompt)}
          >
            <div className="flex items-center gap-3 flex-1 min-w-0">
              <div className="flex-shrink-0 text-text-secondary">{topic.icon}</div>
              <div className="flex-1 min-w-0">
                <p className="text-text-primary text-sm leading-tight">{topic.label}</p>
              </div>
            </div>
            <div className="flex-shrink-0 ml-4">
              <button
                className="text-sm text-text-secondary hover:text-text-primary transition-colors cursor-pointer"
                onClick={(e) => {
                  e.stopPropagation();
                  onSelect(topic.prompt);
                }}
              >
                Start
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default function PopularChatTopics({ append }: PopularChatTopicsProps) {
  const { branding } = useWhiteLabel();

  if (branding.starterPrompts && branding.starterPrompts.length > 0) {
    return (
      <div className="p-6">
        <InsightCards prompts={branding.starterPrompts} onSelect={append} heading="Topics" />
      </div>
    );
  }

  return <TopicList topics={DEFAULT_TOPICS} onSelect={append} />;
}
