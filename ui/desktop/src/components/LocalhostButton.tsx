import { useState } from 'react';
import { Globe } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from './ui/Tooltip';
import { useSidecar } from './SidecarLayout';

interface LocalhostButtonProps {
  className?: string;
  size?: 'xs' | 'sm' | 'lg' | 'default';
}

export function LocalhostButton({ className = '', size = 'sm' }: LocalhostButtonProps) {
  const sidecar = useSidecar();
  const [showInput, setShowInput] = useState(false);
  const [url, setUrl] = useState(() => {
    // Initialize from localStorage or default to '3000'
    if (typeof window !== 'undefined') {
      return localStorage.getItem('goose-localhost-url') || '3000';
    }
    return '3000';
  });

  // Save to localStorage whenever URL changes
  const handleUrlChange = (newUrl: string) => {
    setUrl(newUrl);
    if (typeof window !== 'undefined') {
      localStorage.setItem('goose-localhost-url', newUrl);
    }
  };

  if (!sidecar) return null;

  const handleOpenLocalhost = (inputUrl?: string) => {
    const urlToOpen = inputUrl || url;

    // Format the URL - if it's just a number, prepend localhost
    let formattedUrl = urlToOpen;
    if (/^\d+$/.test(urlToOpen.trim())) {
      formattedUrl = `http://localhost:${urlToOpen.trim()}`;
    } else if (/^(localhost|127\.0\.0\.1):\d+/.test(urlToOpen.trim())) {
      formattedUrl = `http://${urlToOpen.trim()}`;
    }

    console.log('LocalhostButton: Opening URL:', formattedUrl);
    console.log('LocalhostButton: Sidecar available:', !!sidecar);
    
    if (sidecar) {
      sidecar.showLocalhostViewer(formattedUrl, 'Localhost Viewer');
    } else {
      console.error('LocalhostButton: No sidecar available');
    }
    setShowInput(false);
  };

  const handleQuickOpen = (port: string) => {
    handleOpenLocalhost(port);
  };

  if (showInput) {
    return (
      <div className={`flex items-center gap-2 ${className}`}>
        <input
          type="text"
          value={url}
          onChange={(e) => handleUrlChange(e.target.value)}
          onKeyPress={(e) => {
            if (e.key === 'Enter') {
              handleOpenLocalhost();
            } else if (e.key === 'Escape') {
              setShowInput(false);
            }
          }}
          placeholder="3000 or localhost:8080"
          className="w-32 px-2 py-1 text-xs border border-borderSubtle rounded bg-background-default text-textStandard focus:outline-none focus:ring-1 focus:ring-borderProminent"
          autoFocus
          onBlur={() => {
            // Small delay to allow click events to fire
            setTimeout(() => setShowInput(false), 150);
          }}
        />
        <Button
          variant="ghost"
          size="sm"
          onClick={() => handleOpenLocalhost()}
          className="px-2 py-1 text-xs"
        >
          Open
        </Button>
      </div>
    );
  }

  return (
    <div className={`flex items-center ${className}`}>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size={size}
            onClick={() => setShowInput(true)}
            className="p-1 h-8 w-8 text-red-500 hover:text-red-600"
          >
            <Globe size={size === 'sm' ? 14 : 16} />
          </Button>
        </TooltipTrigger>
        <TooltipContent side="top">Open Localhost Site</TooltipContent>
      </Tooltip>

      {/* Quick access buttons for common ports */}
      <div className="ml-1 flex items-center opacity-0 group-hover:opacity-100 transition-opacity">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => handleQuickOpen('3000')}
          className="px-1 py-0 h-6 text-xs text-textSubtle hover:text-primary"
        >
          3000
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => handleQuickOpen('8080')}
          className="px-1 py-0 h-6 text-xs text-textSubtle hover:text-primary"
        >
          8080
        </Button>
      </div>
    </div>
  );
}

export default LocalhostButton;
