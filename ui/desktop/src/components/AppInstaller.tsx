import { useState, useRef, useEffect } from 'react';
import { Download, GitBranch, Folder, Play, Settings, Trash2, ExternalLink, Edit3, Save, X } from 'lucide-react';
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
  // More flexible Git URL validation that supports any Git hosting service
  const gitUrlPatterns = [
    // HTTPS URLs - support any domain with git-like structure
    /^https:\/\/[a-zA-Z0-9\-\.]+\/[\w\-\.\/]+\/[\w\-\.]+(?:\.git)?(?:\/)?$/,
    // SSH URLs - support any domain
    /^git@[a-zA-Z0-9\-\.]+:[\w\-\.\/]+\/[\w\-\.]+\.git$/,
    // Git protocol
    /^git:\/\/[a-zA-Z0-9\-\.]+\/[\w\-\.\/]+\/[\w\-\.]+(?:\.git)?(?:\/)?$/,
  ];
  
  // Additional basic checks
  try {
    const parsedUrl = new URL(url);
    // Must be a reasonable protocol
    if (!['https:', 'http:', 'git:'].includes(parsedUrl.protocol)) {
      return false;
    }
    // Must have a path that looks like a repository
    if (parsedUrl.pathname.split('/').filter(Boolean).length < 2) {
      return false;
    }
    return true;
  } catch {
    // If URL parsing fails, try SSH format
    return /^git@[a-zA-Z0-9\-\.]+:[\w\-\.\/]+\/[\w\-\.]+\.git$/.test(url);
  }
}

function extractRepoInfo(gitUrl: string): { owner: string; repo: string; platform: string } | null {
  try {
    // Handle SSH URLs first
    const sshMatch = gitUrl.match(/git@([^:]+):([\w\-\.\/]+)\/([\w\-\.]+)\.git$/);
    if (sshMatch) {
      const pathParts = sshMatch[2].split('/');
      return {
        owner: pathParts[pathParts.length - 1], // Last part before repo name
        repo: sshMatch[3],
        platform: sshMatch[1]
      };
    }

    // Handle HTTPS/HTTP URLs
    const url = new URL(gitUrl);
    const pathParts = url.pathname.split('/').filter(Boolean);
    
    if (pathParts.length >= 2) {
      // For URLs like https://github.com/owner/repo or https://gitlab.com/group/subgroup/repo
      // Take the last two parts as owner/repo
      const repo = pathParts[pathParts.length - 1].replace('.git', '');
      const owner = pathParts[pathParts.length - 2];
      
      return {
        owner,
        repo,
        platform: url.hostname
      };
    }
  } catch (error) {
    console.error('Error parsing Git URL:', error);
  }
  
  return null;
}

export function AppInstaller({ onAppInstalled }: AppInstallerProps) {
  const [gitUrl, setGitUrl] = useState('');
  const [isInstalling, setIsInstalling] = useState(false);
  const [installProgress, setInstallProgress] = useState('');
  const [installedApps, setInstalledApps] = useState<InstalledApp[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [editingApp, setEditingApp] = useState<string | null>(null);
  const [editForm, setEditForm] = useState<{
    name: string;
    description: string;
    startCommand: string;
    port: string;
    projectType: string;
  }>({
    name: '',
    description: '',
    startCommand: '',
    port: '',
    projectType: 'unknown'
  });
  
  const installationId = useRef<string | null>(null);

  // Load saved apps on component mount
  useEffect(() => {
    const loadSavedApps = async () => {
      try {
        const result = await window.electron.loadSavedApps();
        if (result.success) {
          setInstalledApps(result.apps);
          console.log('Loaded', result.apps.length, 'saved apps');
        }
      } catch (error) {
        console.error('Failed to load saved apps:', error);
      }
    };

    loadSavedApps();
  }, []);

  const handleInstallApp = async () => {
    if (!gitUrl.trim()) {
      setError('Please enter a Git URL');
      return;
    }

    if (!isValidGitUrl(gitUrl)) {
      setError('Please enter a valid Git URL (supports GitHub, GitLab, Bitbucket, and other Git hosting services)');
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

      console.log('Basic project analysis result:', analysisResult);

      // Step 2.5: Use LLM for enhanced analysis if basic analysis is incomplete
      let finalAnalysis = analysisResult;
      if (!analysisResult.startCommand || analysisResult.projectType === 'unknown') {
        setInstallProgress('Running AI-powered project analysis...');
        try {
          const llmResult = await window.electron.analyzeProjectWithLLM(cloneResult.localPath, analysisResult);
          console.log('LLM analysis result:', llmResult);
          
          if (llmResult.success && llmResult.analysis) {
            // Merge LLM analysis with basic analysis
            finalAnalysis = {
              ...analysisResult,
              projectType: llmResult.analysis.projectType || analysisResult.projectType,
              startCommand: llmResult.analysis.startCommand || analysisResult.startCommand,
              port: llmResult.analysis.port || analysisResult.port,
              description: llmResult.analysis.description || analysisResult.description,
              // Keep original package manager and build info
              packageManager: analysisResult.packageManager,
              buildCommand: analysisResult.buildCommand,
              requiresInstall: analysisResult.requiresInstall
            };
            console.log('Enhanced analysis with LLM:', finalAnalysis);
          } else {
            console.warn('LLM analysis failed or returned no results:', llmResult);
          }
        } catch (llmError) {
          console.warn('LLM analysis failed, using basic analysis:', llmError);
          // Continue with basic analysis
        }
      }

      console.log('Final analysis to be used:', finalAnalysis);

      // Step 3: Install dependencies if needed
      if (finalAnalysis.requiresInstall) {
        setInstallProgress('Installing dependencies...');
        const installResult = await window.electron.installProjectDependencies(cloneResult.localPath, finalAnalysis.packageManager);
        
        if (!installResult.success) {
          console.warn('Dependency installation failed:', installResult.error);
          // Continue anyway, some projects might work without full dependency installation
        }
      }

      // Step 4: Create app configuration
      const newApp: InstalledApp = {
        id: appId,
        name: finalAnalysis.name || repoInfo.repo,
        description: finalAnalysis.description || `${repoInfo.owner}/${repoInfo.repo}`,
        gitUrl,
        localPath: cloneResult.localPath,
        projectType: finalAnalysis.projectType,
        buildCommand: finalAnalysis.buildCommand,
        startCommand: finalAnalysis.startCommand,
        port: finalAnalysis.port,
        status: 'ready',
        lastUpdated: new Date(),
      };

      // Step 5: Save app configuration
      setInstallProgress('Saving app configuration...');
      await window.electron.saveAppConfiguration(newApp);

      // Step 6: Add to installed apps
      setInstalledApps(prev => [...prev, newApp]);
      setInstallProgress('Installation complete!');
      
      // Step 7: Auto-launch the app if it's a web app
      if (newApp.projectType === 'web' && newApp.startCommand) {
        setInstallProgress('Launching app...');
        try {
          const launchResult = await window.electron.launchApp(newApp);
          if (launchResult.success && newApp.port) {
            // Open in WebViewer after a short delay to let the server start
            setTimeout(() => {
              const event = new CustomEvent('add-container', {
                detail: { 
                  contentType: 'web-viewer',
                  url: `http://localhost:${newApp.port}`,
                  title: newApp.name
                }
              });
              window.dispatchEvent(event);
            }, 3000); // 3 second delay for server startup
            
            // Update app status to running
            setInstalledApps(prev => 
              prev.map(a => a.id === newApp.id ? { ...a, status: 'running' } : a)
            );
          }
        } catch (launchError) {
          console.warn('Auto-launch failed:', launchError);
          // Don't fail the installation if launch fails
        }
      }
      
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
      // Check if app has a start command before attempting to launch
      if (!app.startCommand) {
        setError(`Cannot launch ${app.name}: No start command defined. This app may need manual configuration.`);
        setInstalledApps(prev => 
          prev.map(a => a.id === app.id ? { ...a, status: 'error' } : a)
        );
        return;
      }

      console.log('Launching app with config:', {
        name: app.name,
        startCommand: app.startCommand,
        projectType: app.projectType,
        port: app.port,
        localPath: app.localPath
      });

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
      setError(`Failed to launch ${app.name}: ${err instanceof Error ? err.message : 'Unknown error'}`);
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

  const handleEditApp = (app: InstalledApp) => {
    setEditingApp(app.id);
    setEditForm({
      name: app.name,
      description: app.description,
      startCommand: app.startCommand || '',
      port: app.port?.toString() || '',
      projectType: app.projectType
    });
    setError(null); // Clear any existing errors
  };

  const handleCancelEdit = () => {
    setEditingApp(null);
    setEditForm({
      name: '',
      description: '',
      startCommand: '',
      port: '',
      projectType: 'unknown'
    });
  };

  const handleSaveEdit = async () => {
    if (!editingApp) return;

    try {
      const app = installedApps.find(a => a.id === editingApp);
      if (!app) return;

      // Validate required fields
      if (!editForm.name.trim()) {
        setError('App name is required');
        return;
      }

      if (!editForm.startCommand.trim()) {
        setError('Start command is required');
        return;
      }

      // Parse port number
      let port: number | undefined;
      if (editForm.port.trim()) {
        const portNum = parseInt(editForm.port.trim());
        if (isNaN(portNum) || portNum < 1 || portNum > 65535) {
          setError('Port must be a valid number between 1 and 65535');
          return;
        }
        port = portNum;
      }

      // Create updated app config
      const updatedApp: InstalledApp = {
        ...app,
        name: editForm.name.trim(),
        description: editForm.description.trim(),
        startCommand: editForm.startCommand.trim(),
        port: port,
        projectType: editForm.projectType as InstalledApp['projectType'],
        lastUpdated: new Date()
      };

      // Save to electron backend
      const saveResult = await window.electron.saveAppConfiguration(updatedApp);
      if (!saveResult.success) {
        throw new Error(saveResult.error || 'Failed to save app configuration');
      }

      // Update local state
      setInstalledApps(prev => 
        prev.map(a => a.id === editingApp ? updatedApp : a)
      );

      // Clear edit state
      setEditingApp(null);
      setError(null);

      console.log('App configuration updated successfully:', updatedApp);

    } catch (err) {
      console.error('Failed to save app configuration:', err);
      setError(`Failed to save configuration: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
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
              placeholder="https://gitlab.gnome.org/GNOME/gimp or any Git repository URL"
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
                {editingApp === app.id ? (
                  // Edit Mode
                  <div className="space-y-3">
                    <div className="flex items-center gap-2 mb-3">
                      <Edit3 size={16} className="text-blue-600" />
                      <span className="font-medium text-text-standard">Edit App Configuration</span>
                    </div>
                    
                    <div className="grid grid-cols-1 gap-3">
                      <div>
                        <label className="block text-xs font-medium text-text-standard mb-1">Name</label>
                        <input
                          type="text"
                          value={editForm.name}
                          onChange={(e) => setEditForm(prev => ({ ...prev, name: e.target.value }))}
                          className="w-full px-2 py-1 text-sm border border-border-subtle rounded bg-background-default text-text-standard focus:outline-none focus:ring-1 focus:ring-border-prominent"
                          placeholder="App name"
                        />
                      </div>
                      
                      <div>
                        <label className="block text-xs font-medium text-text-standard mb-1">Description</label>
                        <input
                          type="text"
                          value={editForm.description}
                          onChange={(e) => setEditForm(prev => ({ ...prev, description: e.target.value }))}
                          className="w-full px-2 py-1 text-sm border border-border-subtle rounded bg-background-default text-text-standard focus:outline-none focus:ring-1 focus:ring-border-prominent"
                          placeholder="App description"
                        />
                      </div>
                      
                      <div>
                        <label className="block text-xs font-medium text-text-standard mb-1">Start Command *</label>
                        <input
                          type="text"
                          value={editForm.startCommand}
                          onChange={(e) => setEditForm(prev => ({ ...prev, startCommand: e.target.value }))}
                          className="w-full px-2 py-1 text-sm border border-border-subtle rounded bg-background-default text-text-standard focus:outline-none focus:ring-1 focus:ring-border-prominent"
                          placeholder="e.g., npm start, python app.py, cargo run"
                        />
                      </div>
                      
                      <div className="grid grid-cols-2 gap-2">
                        <div>
                          <label className="block text-xs font-medium text-text-standard mb-1">Port (optional)</label>
                          <input
                            type="number"
                            value={editForm.port}
                            onChange={(e) => setEditForm(prev => ({ ...prev, port: e.target.value }))}
                            className="w-full px-2 py-1 text-sm border border-border-subtle rounded bg-background-default text-text-standard focus:outline-none focus:ring-1 focus:ring-border-prominent"
                            placeholder="3000"
                            min="1"
                            max="65535"
                          />
                        </div>
                        
                        <div>
                          <label className="block text-xs font-medium text-text-standard mb-1">Project Type</label>
                          <select
                            value={editForm.projectType}
                            onChange={(e) => setEditForm(prev => ({ ...prev, projectType: e.target.value }))}
                            className="w-full px-2 py-1 text-sm border border-border-subtle rounded bg-background-default text-text-standard focus:outline-none focus:ring-1 focus:ring-border-prominent"
                          >
                            <option value="web">Web</option>
                            <option value="cli">CLI</option>
                            <option value="electron">Electron</option>
                            <option value="library">Library</option>
                            <option value="unknown">Unknown</option>
                          </select>
                        </div>
                      </div>
                    </div>
                    
                    <div className="flex items-center gap-2 pt-2">
                      <Button
                        onClick={handleSaveEdit}
                        size="sm"
                        className="px-3 py-1 text-xs"
                      >
                        <Save size={12} className="mr-1" />
                        Save
                      </Button>
                      <Button
                        onClick={handleCancelEdit}
                        variant="ghost"
                        size="sm"
                        className="px-3 py-1 text-xs"
                      >
                        <X size={12} className="mr-1" />
                        Cancel
                      </Button>
                    </div>
                  </div>
                ) : (
                  // View Mode
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
                        {app.startCommand && <span>Start: {app.startCommand}</span>}
                        {!app.startCommand && <span className="text-orange-600 dark:text-orange-400">⚠️ No start command</span>}
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
                            onClick={() => handleEditApp(app)}
                            className="p-1 h-8 w-8"
                          >
                            <Edit3 size={14} />
                          </Button>
                        </TooltipTrigger>
                        <TooltipContent>Edit Configuration</TooltipContent>
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
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default AppInstaller;
