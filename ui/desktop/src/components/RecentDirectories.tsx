import { useEffect, useState } from 'react';
import { Card, CardContent } from './ui/card';
import { Folder } from 'lucide-react';
import { createSession } from '../sessions';
import { useNavigation } from '../hooks/useNavigation';
import { useConfig } from './ConfigContext';
import { AppEvents } from '../constants/events';
import { Skeleton } from './ui/skeleton';

interface RecentDirectoriesProps {
  onSessionStarting?: () => void;
}

export function RecentDirectories({ onSessionStarting }: RecentDirectoriesProps) {
  const [recentDirs, setRecentDirs] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [startingDir, setStartingDir] = useState<string | null>(null);
  const setView = useNavigation();
  const { extensionsList } = useConfig();

  useEffect(() => {
    const loadRecentDirs = async () => {
      try {
        const dirs = await window.electron.getRecentDirs();
        setRecentDirs(dirs.slice(0, 3));
      } catch (error) {
        console.error('Failed to load recent directories:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadRecentDirs();
  }, []);

  const handleDirectoryClick = async (dir: string) => {
    if (startingDir) return;

    setStartingDir(dir);
    onSessionStarting?.();

    try {
      const session = await createSession(dir, {
        allExtensions: extensionsList,
      });

      window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED));
      window.dispatchEvent(
        new CustomEvent(AppEvents.ADD_ACTIVE_SESSION, {
          detail: { sessionId: session.id },
        })
      );

      setView('pair', {
        disableAnimation: true,
        resumeSessionId: session.id,
      });
    } catch (error) {
      console.error('Failed to create session:', error);
      setStartingDir(null);
    }
  };

  const getDirectoryName = (path: string) => {
    const parts = path.split('/').filter(Boolean);
    return parts[parts.length - 1] || path;
  };

  const getParentPath = (path: string) => {
    const parts = path.split('/').filter(Boolean);
    if (parts.length <= 1) return '';
    parts.pop();
    return '/' + parts.join('/');
  };

  if (isLoading) {
    return (
      <div className="grid grid-cols-1 gap-0.5">
        {[1, 2, 3].map((i) => (
          <Card key={i} className="w-full py-8 px-6 border-none rounded-2xl bg-background-default">
            <CardContent className="p-0">
              <div className="flex items-center gap-4">
                <Skeleton className="h-12 w-12 rounded-xl" />
                <div className="flex-1">
                  <Skeleton className="h-6 w-48 mb-2" />
                  <Skeleton className="h-4 w-64" />
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  if (recentDirs.length === 0) {
    return (
      <Card className="w-full py-8 px-6 border-none rounded-2xl bg-background-default">
        <CardContent className="p-0 text-center">
          <p className="text-text-muted">No recent directories</p>
          <p className="text-text-muted text-sm mt-1">Open a directory to get started</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-0.5">
      {recentDirs.map((dir, index) => (
        <Card
          key={dir}
          className={`w-full py-6 px-6 border-none rounded-2xl bg-background-default cursor-pointer 
            transition-all duration-200 hover:bg-background-muted
            ${startingDir === dir ? 'opacity-70' : ''}
            ${startingDir && startingDir !== dir ? 'pointer-events-none opacity-50' : ''}`}
          onClick={() => handleDirectoryClick(dir)}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              handleDirectoryClick(dir);
            }
          }}
          style={{ animationDelay: `${index * 0.1}s` }}
        >
          <CardContent className="p-0">
            <div className="flex items-center gap-4">
              <div className="flex-shrink-0 w-12 h-12 rounded-xl bg-background-muted flex items-center justify-center">
                <Folder className="w-6 h-6 text-text-muted" />
              </div>
              <div className="flex-1 min-w-0">
                <h3 className="text-lg font-medium truncate">{getDirectoryName(dir)}</h3>
                <p className="text-sm text-text-muted truncate">{getParentPath(dir)}</p>
              </div>
              <div className="flex-shrink-0 text-text-muted">
                <span className="text-sm">Start session →</span>
              </div>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
