import { useEffect, useState } from 'react';
import { Card, CardContent } from './ui/card';
import { Folder, FileText, Image } from 'lucide-react';
import { createSession } from '../sessions';
import { useNavigation } from '../hooks/useNavigation';
import { useConfig } from './ConfigContext';
import { AppEvents } from '../constants/events';
import { Skeleton } from './ui/skeleton';
import type { RecentItem, RecentItemType } from '../preload';

interface RecentFilesProps {
  onSessionStarting?: () => void;
}

const TYPE_CONFIG: Record<
  RecentItemType,
  {
    icon: typeof Folder;
    color: string;
    bgColor: string;
  }
> = {
  repo: {
    icon: Folder,
    color: 'text-green-600',
    bgColor: 'bg-green-50 dark:bg-green-950',
  },
  document: {
    icon: FileText,
    color: 'text-blue-600',
    bgColor: 'bg-blue-50 dark:bg-blue-950',
  },
  image: {
    icon: Image,
    color: 'text-purple-600',
    bgColor: 'bg-purple-50 dark:bg-purple-950',
  },
};

export function RecentFiles({ onSessionStarting }: RecentFilesProps) {
  const [recentFiles, setRecentFiles] = useState<RecentItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [startingItem, setStartingItem] = useState<string | null>(null);
  const setView = useNavigation();
  const { extensionsList } = useConfig();

  useEffect(() => {
    const loadRecentFiles = async () => {
      try {
        const files = await window.electron.getRecentFiles(24);
        setRecentFiles(files.slice(0, 6));
      } catch (error) {
        console.error('Failed to load recent files:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadRecentFiles();
  }, []);

  const handleItemClick = async (item: RecentItem) => {
    if (startingItem) return;

    setStartingItem(item.fullPath);
    onSessionStarting?.();

    try {
      // Determine working directory based on item type
      const workingDir = item.type === 'repo' ? item.fullPath : undefined;

      const session = await createSession(workingDir || '', {
        allExtensions: extensionsList,
      });

      window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED));
      window.dispatchEvent(
        new CustomEvent(AppEvents.ADD_ACTIVE_SESSION, {
          detail: { sessionId: session.id },
        })
      );

      // Build the initial prompt based on item type
      let initialPrompt = '';
      if (item.type === 'image') {
        initialPrompt = `Please analyze this image: ${item.fullPath}`;
      } else if (item.type === 'document') {
        initialPrompt = `Let's discuss this document: ${item.fullPath}`;
      } else if (item.type === 'repo') {
        initialPrompt = `I'd like to work on this project. What would you like to help me with?`;
      }

      setView('pair', {
        disableAnimation: true,
        resumeSessionId: session.id,
        initialMessage: initialPrompt ? { msg: initialPrompt, images: [] } : undefined,
      });
    } catch (error) {
      console.error('Failed to create session:', error);
      setStartingItem(null);
    }
  };

  if (isLoading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
        {[1, 2, 3, 4].map((i) => (
          <Card
            key={i}
            className="w-full py-4 px-4 border-none rounded-xl bg-background-default"
          >
            <CardContent className="p-0">
              <div className="flex items-center gap-3">
                <Skeleton className="h-10 w-10 rounded-lg" />
                <div className="flex-1">
                  <Skeleton className="h-4 w-32 mb-2" />
                  <Skeleton className="h-3 w-48" />
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  if (recentFiles.length === 0) {
    return null;
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
      {recentFiles.map((item) => {
        const config = TYPE_CONFIG[item.type];
        const Icon = config.icon;

        return (
          <Card
            key={item.fullPath}
            className={`w-full py-4 px-4 border-none rounded-xl bg-background-default cursor-pointer 
              transition-all duration-200 hover:bg-background-muted hover:scale-[1.01]
              ${startingItem === item.fullPath ? 'opacity-70' : ''}
              ${startingItem && startingItem !== item.fullPath ? 'pointer-events-none opacity-50' : ''}`}
            onClick={() => handleItemClick(item)}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                handleItemClick(item);
              }
            }}
          >
            <CardContent className="p-0">
              <div className="flex items-start gap-3">
                <div
                  className={`flex-shrink-0 w-10 h-10 rounded-lg ${config.bgColor} flex items-center justify-center`}
                >
                  <Icon className={`w-5 h-5 ${config.color}`} />
                </div>
                <div className="flex-1 min-w-0">
                  <h4 className="text-sm font-medium truncate">{item.name}</h4>
                  <p className="text-xs text-text-muted truncate">{item.path}</p>
                  <p className="text-xs text-text-subtle mt-1">{item.suggestion}</p>
                </div>
              </div>
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}
