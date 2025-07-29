import React from 'react';
import { FolderTree, MessageSquare, Code } from 'lucide-react';

interface PopularChatTopicsProps {
  append: (text: string) => void;
}

interface ChatTopic {
  id: string;
  icon: React.ReactNode;
  description: string;
  prompt: string;
}

const POPULAR_TOPICS: ChatTopic[] = [
  {
    id: 'organize-photos',
    icon: <FolderTree className="w-5 h-5" />,
    description: 'Organize the photos on my desktop into neat little folders by subject matter',
    prompt: 'Organize the photos on my desktop into neat little folders by subject matter',
  },
  {
    id: 'government-forms',
    icon: <MessageSquare className="w-5 h-5" />,
    description:
      'Describe in detail how various forms of government works and rank each by units of geese',
    prompt:
      'Describe in detail how various forms of government works and rank each by units of geese',
  },
  {
    id: 'tamagotchi-game',
    icon: <Code className="w-5 h-5" />,
    description:
      'Develop a tamagotchi game that lives on my computer and follows a pixelated styling',
    prompt: 'Develop a tamagotchi game that lives on my computer and follows a pixelated styling',
  },
];

export default function PopularChatTopics({ append }: PopularChatTopicsProps) {
  const handleTopicClick = (prompt: string) => {
    append(prompt);
  };

  return (
    <div className="absolute bottom-0 left-0 p-6 max-w-md">
      <h3 className="text-text-muted text-sm mb-1">Popular chat topics</h3>
      <div className="space-y-1">
        {POPULAR_TOPICS.map((topic) => (
          <div
            key={topic.id}
            className="flex items-center justify-between py-1.5 hover:bg-bgSubtle rounded-md cursor-pointer transition-colors"
            onClick={() => handleTopicClick(topic.prompt)}
          >
            <div className="flex items-center gap-3 flex-1 min-w-0">
              <div className="flex-shrink-0 text-text-muted">{topic.icon}</div>
              <div className="flex-1 min-w-0">
                <p className="text-text-default text-sm leading-tight">{topic.description}</p>
              </div>
            </div>
            <div className="flex-shrink-0 ml-4">
              <button
                className="text-sm text-text-muted hover:text-text-default transition-colors cursor-pointer"
                onClick={(e) => {
                  e.stopPropagation();
                  handleTopicClick(topic.prompt);
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
