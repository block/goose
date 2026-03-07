import React from 'react';
import {
  FolderTree,
  MessageSquare,
  Code,
  BookOpen,
  Terminal,
  Zap,
  Search,
  FileText,
  LayoutDashboard,
  CalendarCheck,
  type LucideIcon,
} from 'lucide-react';
import { useWhiteLabel } from '../whitelabel/WhiteLabelContext';

interface PopularChatTopicsProps {
  append: (text: string) => void;
}

interface ChatTopic {
  id: string;
  icon: React.ReactNode;
  description: string;
  prompt: string;
}

const ICON_MAP: Record<string, LucideIcon> = {
  'folder-tree': FolderTree,
  'message-square': MessageSquare,
  code: Code,
  'book-open': BookOpen,
  terminal: Terminal,
  zap: Zap,
  search: Search,
  'file-text': FileText,
  'layout-dashboard': LayoutDashboard,
  'calendar-check': CalendarCheck,
};

const DEFAULT_TOPICS: ChatTopic[] = [
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

function useTopics(): ChatTopic[] {
  const { branding } = useWhiteLabel();

  if (!branding.starterPrompts || branding.starterPrompts.length === 0) {
    return DEFAULT_TOPICS;
  }

  return branding.starterPrompts.map((sp, i) => {
    const IconComponent = ICON_MAP[sp.icon] ?? Code;
    return {
      id: `starter-${i}`,
      icon: <IconComponent className="w-5 h-5" />,
      description: sp.label,
      prompt: sp.prompt,
    };
  });
}

export default function PopularChatTopics({ append }: PopularChatTopicsProps) {
  const topics = useTopics();

  const handleTopicClick = (prompt: string) => {
    append(prompt);
  };

  return (
    <div className="absolute bottom-0 left-0 p-6 max-w-md">
      <h3 className="text-text-secondary text-sm mb-1">Popular chat topics</h3>
      <div className="space-y-1">
        {topics.map((topic) => (
          <div
            key={topic.id}
            className="flex items-center justify-between py-1.5 hover:bg-background-secondary rounded-md cursor-pointer transition-colors"
            onClick={() => handleTopicClick(topic.prompt)}
          >
            <div className="flex items-center gap-3 flex-1 min-w-0">
              <div className="flex-shrink-0 text-text-secondary">{topic.icon}</div>
              <div className="flex-1 min-w-0">
                <p className="text-text-primary text-sm leading-tight">{topic.description}</p>
              </div>
            </div>
            <div className="flex-shrink-0 ml-4">
              <button
                className="text-sm text-text-secondary hover:text-text-primary transition-colors cursor-pointer"
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
