import { useState, useEffect } from 'react';
import { Globe, FileText } from 'lucide-react';
import LocalhostViewer from './LocalhostViewer';
import SpecsViewer from './SpecsViewer';

interface SidecarTabsProps {
  initialUrl?: string;
  projectPath?: string;
}

export function SidecarTabs({
  initialUrl = 'http://localhost:3000',
  projectPath,
}: SidecarTabsProps) {
  const [activeTab, setActiveTab] = useState<'localhost' | 'specs'>('localhost');
  const [currentProjectPath, setCurrentProjectPath] = useState<string | undefined>(projectPath);

  useEffect(() => {
    if (!projectPath) {
      const storedPath = localStorage.getItem('goose-current-project-path');
      if (storedPath) {
        setCurrentProjectPath(storedPath);
      }
    }
  }, [projectPath]);

  return (
    <div className="h-full flex flex-col bg-background-default">
      <div className="flex border-b border-borderSubtle bg-background-muted">
        <button
          onClick={() => setActiveTab('localhost')}
          className={`px-4 py-2 flex items-center gap-2 text-sm font-medium transition-colors ${
            activeTab === 'localhost'
              ? 'text-textStandard border-b-2 border-primary bg-background-default'
              : 'text-textMuted hover:text-textStandard hover:bg-background-default/50'
          }`}
        >
          <Globe size={16} />
          Localhost
        </button>

        {currentProjectPath && (
          <button
            onClick={() => setActiveTab('specs')}
            className={`px-4 py-2 flex items-center gap-2 text-sm font-medium transition-colors ${
              activeTab === 'specs'
                ? 'text-textStandard border-b-2 border-primary bg-background-default'
                : 'text-textMuted hover:text-textStandard hover:bg-background-default/50'
            }`}
          >
            <FileText size={16} />
            Specs
          </button>
        )}
      </div>

      <div className="flex-1 overflow-hidden">
        {activeTab === 'localhost' ? (
          <LocalhostViewer initialUrl={initialUrl} />
        ) : (
          currentProjectPath && <SpecsViewer projectPath={currentProjectPath} />
        )}
      </div>
    </div>
  );
}

export default SidecarTabs;
