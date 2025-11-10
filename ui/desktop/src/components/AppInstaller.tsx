import { useState, useRef } from 'react';
import { Download, GitBranch, Folder, Play, Settings, Trash2, ExternalLink } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from './ui/Tooltip';

interface InstalledApp {
  id: string;
  name: string;
  description: string;
  gitUrl: string;
  localPath: string;
  projectType: 'web' | 'electron' | 'cli' | 'library' | 'unknown';
  buildCommand?: string;
  startCommand?: string;
  port?: number;
  status: 'installing' | 'ready' | 'running' | 'error';
  lastUpdated: Date;
}

interface AppInstallerProps {
  onAppInstalled?: (app: InstalledApp) => void;
}

function isValidGitUrl(url: string): boolean {
  // Basic Git URL validation
  const gitUrlPatterns = [
    /^https:\/\/github\.com\/[\w\-\.]+\/[\w\-\.]+(?:\.git)?$/,
    /^https:\/\/gitlab\.com\/[\w\-\.]+\/[\w\-\.]+(?:\.git)?$/,
    /^https:\/\/bitbucket\.org\/[\w\-\.]+\/[\w\-\.]+(?:\.git)?$/,
    /^git@github\.com:[\w\-\.]+\/[\w\-\.]+\.git$/,
    /^git@gitlab\.com:[\w\-\.]+\/[\w\-\.]+\.git$/,
  ];
  
  return gitUrlPatterns.some(pattern => pattern.test(url));
}

function extractRepoInfo(gitUrl: string): { owner: string; repo: string; platform: string } | null {
  try {
    const url = new URL(gitUrl.replace('git@', 'https://').replace(':', '/'));
    const pathParts = url.pathname.split('/').filter(Boolean);
    
    if (pathParts.length >= 2) {
      return {
        owner: pathParts[0],
        repo: pathParts[1].replace('.git', ''),
        platform: url.hostname
      };
    }
  } catch {
    // Handle SSH URLs
    const sshMatch = gitUrl.match(/git@([^:]+):([\w\-\.]+)\/([\w\-\.]+)\.git$/);
    if (sshMatch) {
      return {
        owner: sshMatch[2],
        repo: sshMatch[3],
        platform: sshMatch[1]
      };
    }
  }
  
  return null;
}

export function AppInstaller({ onAppInstalled }: AppInstallerProps) {
  const [gitUrl, setGitUrl] = useState('');
  const [isInstalling, setIsInstalling] = useState(false);
  const [installProgress, setInstallProgress] = useState('');
  const [installedApps, setInstalledApps] = useState<InstalledApp[]>([]);
  const [error, setError] = useState<string | null>(null);
  
  const installationId = useRef<string | null>(null);

  const handleInstallApp = async () => {
    if (!gitUrl.trim()) {
      setError('Please enter a Git URL');
      return;
    }

    if (!isValidGitUrl(gitUrl)) {
      setError('Please enter a valid Git URL (GitHub, GitLab, or Bitbucket)');
      return;
    }

    const repoInfo = extractRepoInfo(gitUrl);
    if (!repoInfo) {
      setError('Could not parse repository information from URL');
      return;
    }

    setIsInstalling(true);
    setError(null);
    setInstallProgress('Initializing installation...');
    
    const appId = `${repoInfo.owner}-${repoInfo.repo}-${Date.now()}`;
    installationId.current = appId;

    try {
      // Step 1: Clone the repository
      setInstallProgress('Cloning repository...');
      const cloneResult = await window.electron.cloneRepository(gitUrl, appId);
      
      if (!cloneResult.success) {
        throw new Error(cloneResult.error || 'Failed to clone repository');
      }

      // Step 2: Analyze the project
      setInstallProgress('Analyzing project structure...');
      const analysisResult = await window.electron.analyzeProject(cloneResult.localPath);
      
      if (!analysisResult.success) {
        throw new Error(analysisResult.error || 'Failed to analyze project');
      }

      // Step 3: Install dependencies if needed
      if (analysisResult.requiresInstall) {
        setInstallProgress('Installing dependencies...');
        const installResult = await window.electron.installProjectDependencies(cloneResult.localPath, analysisResult.packageManager);
        
        if (!installResult.success) {
          console.warn('Dependency installation failed:', installResult.error);
          // Continue anyway, some projects might work without full dependency installation
        }
      }

      // Step 4: Create app configuration
      const newApp: InstalledApp = {
        id: appId,
        name: analysisResult.name || repoInfo.repo,
        description: analysisResult.description || `${repoInfo.owner}/${repoInfo.repo}`,
        gitUrl,
        localPath: cloneResult.localPath,
        projectType: analysisResult.projectType,
        buildCommand: analysisResult.buildCommand,
        startCommand: analysisResult.startCommand,
        port: analysisResult.port,
        status: 'ready',
        lastUpdated: new Date(),
      };

      // Step 5: Save app configuration
      setInstallProgress('Saving app configuration...');
      await window.electron.saveAppConfiguration(newApp);

      // Step 6: Add to installed apps
      setInstalledApps(prev => [...prev, newApp]);
      setInstallProgress('Installation complete!');
      
      // Notify parent component
      if (onAppInstalled) {
        onAppInstalled(newApp);
      }

      // Clear form
      setGitUrl('');
      
      setTimeout(() => {
        setInstallProgress('');
        setIsInstalling(false);
      }, 2000);

    } catch (err) {
      console.error('App installation failed:', err);
      setError(err instanceof Error ? err.message : 'Installation failed');
      setInstallProgress('');
      setIsInstalling(false);
    }
  };

  const handleLaunchApp = async (app: InstalledApp) => {
    try {
      setInstalledApps(prev => 
        prev.map(a => a.id === app.id ? { ...a, status: 'running' } : a)
      );

      const launchResult = await window.electron.launchApp(app);
      
      if (!launchResult.success) {
        throw new Error(launchResult.error || 'Failed to launch app');
      }

      // If it's a web app, we might want to open it in the WebViewer
      if (app.projectType === 'web' && app.port) {
        // Dispatch event to open in WebViewer
        const event = new CustomEvent('add-container', {
          detail: { 
            contentType: 'web-viewer',
            url: `http://localhost:${app.port}`,
            title: app.name
          }
        });
        window.dispatchEvent(event);
      }

    } catch (err) {
      console.error('App launch failed:', err);
      setInstalledApps(prev => 
        prev.map(a => a.id === app.id ? { ...a, status: 'error' } : a)
      );
    }
  };

  const handleRemoveApp = async (app: InstalledApp) => {
    try {
      await window.electron.removeApp(app.id);
      setInstalledApps(prev => prev.filter(a => a.id !== app.id));
    } catch (err) {
      console.error('Failed to remove app:', err);
    }
  };

  const handleOpenInFinder = (app: InstalledApp) => {
    window.electron.showItemInFolder(app.localPath);
  };

  const handleOpenRepo = (app: InstalledApp) => {
    window.electron.openExternal(app.gitUrl);
  };

  return (
    <div className="h-full flex flex-col bg-background-default rounded-lg border border-border-subtle">
      {/* Header */}
      <div className="flex items-center gap-2 p-3 border-b border-border-subtle bg-background-muted rounded-t-lg">
        <Download size={16} className="text-primary" />
        <h3 className="font-medium text-text-standard">App Installer</h3>
      </div>

      {/* Install Form */}
      <div className="p-4 border-b border-border-subtle">
        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-2">
            <GitBranch size={14} className="text-text-subtle" />
            <span className="text-sm font-medium text-text-standard">Install from Git Repository</span>
          </div>
          
          <div className="flex gap-2">
            <input
              type="text"
              value={gitUrl}
              onChange={(e) => setGitUrl(e.target.value)}
              placeholder="https://github.com/owner/repo or git@github.com:owner/repo.git"
              className="flex-1 px-3 py-2 text-sm border border-border-subtle rounded-md bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-border-prominent focus:border-transparent"
              disabled={isInstalling}
            />
            <Button
              onClick={handleInstallApp}
              disabled={isInstalling || !gitUrl.trim()}
              className="px-4"
            >
              {isInstalling ? 'Installing...' : 'Install'}
            </Button>
          </div>

          {installProgress && (
            <div className="text-sm text-text-subtle bg-blue-50 dark:bg-blue-900/20 px-3 py-2 rounded-md">
              {installProgress}
            </div>
          )}

          {error && (
            <div className="text-sm text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20 px-3 py-2 rounded-md">
              {error}
            </div>
          )}
        </div>
      </div>

      {/* Installed Apps */}
      <div className="flex-1 overflow-auto">
        {installedApps.length === 0 ? (
          <div className="flex items-center justify-center h-full text-text-subtle">
            <div className="text-center">
              <Download size={24} className="mx-auto mb-2 opacity-50" />
              <p className="text-sm">No apps installed yet</p>
              <p className="text-xs mt-1">Enter a Git URL above to install your first app</p>
            </div>
          </div>
        ) : (
          <div className="p-4 space-y-3">
            {installedApps.map((app) => (
              <div
                key={app.id}
                className="p-3 border border-border-subtle rounded-lg bg-background-default hover:bg-background-muted transition-colors"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <h4 className="font-medium text-text-standard truncate">{app.name}</h4>
                      <span className={`px-2 py-0.5 text-xs rounded-full ${
                        app.status === 'ready' ? 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400' :
                        app.status === 'running' ? 'bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-400' :
                        app.status === 'installing' ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-400' :
                        'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400'
                      }`}>
                        {app.status}
                      </span>
                      <span className="px-2 py-0.5 text-xs bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300 rounded-full">
                        {app.projectType}
                      </span>
                    </div>
                    <p className="text-sm text-text-subtle truncate mb-2">{app.description}</p>
                    <div className="flex items-center gap-4 text-xs text-text-subtle">
                      {app.port && <span>Port: {app.port}</span>}
                      <span>Updated: {app.lastUpdated.toLocaleDateString()}</span>
                    </div>
                  </div>
                  
                  <div className="flex items-center gap-1 ml-3">
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleLaunchApp(app)}
                          disabled={app.status === 'installing' || app.status === 'running'}
                          className="p-1 h-8 w-8"
                        >
                          <Play size={14} />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>Launch App</TooltipContent>
                    </Tooltip>

                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleOpenInFinder(app)}
                          className="p-1 h-8 w-8"
                        >
                          <Folder size={14} />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>Show in Finder</TooltipContent>
                    </Tooltip>

                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleOpenRepo(app)}
                          className="p-1 h-8 w-8"
                        >
                          <ExternalLink size={14} />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>Open Repository</TooltipContent>
                    </Tooltip>

                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleRemoveApp(app)}
                          className="p-1 h-8 w-8 text-red-500 hover:text-red-700"
                        >
                          <Trash2 size={14} />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>Remove App</TooltipContent>
                    </Tooltip>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default AppInstaller;
