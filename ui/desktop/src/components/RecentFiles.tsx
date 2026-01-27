import { useEffect, useState } from 'react';
import { Card, CardContent } from './ui/card';
import { FolderGit2, FileText, ImageIcon, FileSpreadsheet, FileType, File } from 'lucide-react';
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
    icon: typeof File;
    color: string;
    bgColor: string;
  }
> = {
  repo: {
    icon: FolderGit2,
    color: 'text-green-600 dark:text-green-400',
    bgColor: 'bg-green-100 dark:bg-green-900/50',
  },
  document: {
    icon: FileText,
    color: 'text-blue-600 dark:text-blue-400',
    bgColor: 'bg-blue-100 dark:bg-blue-900/50',
  },
  image: {
    icon: ImageIcon,
    color: 'text-purple-600 dark:text-purple-400',
    bgColor: 'bg-purple-100 dark:bg-purple-900/50',
  },
};

function getIconForFile(item: RecentItem) {
  if (item.type === 'repo') return FolderGit2;
  if (item.type === 'image') return ImageIcon;

  const ext = item.name.split('.').pop()?.toLowerCase() || '';
  if (['xlsx', 'xls', 'csv'].includes(ext)) return FileSpreadsheet;
  if (['pdf'].includes(ext)) return FileType;
  if (['md', 'txt', 'doc', 'docx'].includes(ext)) return FileText;
  return File;
}

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
      <div className="grid grid-cols-2 gap-3">
        {[1, 2, 3, 4, 5, 6].map((i) => (
          <div key={i} className="flex flex-col items-center gap-3 p-4">
            <Skeleton className="h-16 w-16 rounded-2xl" />
            <Skeleton className="h-4 w-24" />
          </div>
        ))}
      </div>
    );
  }

  if (recentFiles.length === 0) {
    return null;
  }

  return (
    <div className="grid grid-cols-2 gap-3">
      {recentFiles.map((item, index) => {
        const config = TYPE_CONFIG[item.type];
        const Icon = getIconForFile(item);

        return (
          <Card
            key={item.fullPath}
            className={`w-full py-5 px-4 border-none rounded-2xl bg-background-muted/50 cursor-pointer 
              transition-all duration-200 hover:bg-background-muted hover:scale-[1.02]
              ${startingItem === item.fullPath ? 'opacity-70 scale-[0.98]' : ''}
              ${startingItem && startingItem !== item.fullPath ? 'pointer-events-none opacity-50' : ''}`}
            onClick={() => handleItemClick(item)}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                handleItemClick(item);
              }
            }}
            style={{ animationDelay: `${index * 0.05}s` }}
          >
            <CardContent className="p-0">
              <div className="flex flex-col items-center text-center gap-3">
                <div
                  className={`w-16 h-16 rounded-2xl ${config.bgColor} flex items-center justify-center`}
                >
                  <Icon className={`w-8 h-8 ${config.color}`} strokeWidth={1.5} />
                </div>
                <div className="min-w-0 w-full">
                  <h3 className="text-sm font-medium truncate">{item.name}</h3>
                  <p className="text-xs text-text-muted truncate">{item.suggestion}</p>
                </div>
              </div>
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}
