import React, { createContext, useContext, useRef, useCallback, useEffect } from 'react';

// Base interface for all sidecar types
interface BaseSidecarInfo {
  id: string;
  type: 'web-viewer' | 'file-viewer' | 'document-editor' | 'localhost-viewer' | 'app-installer' | 'diff-viewer';
  title: string;
  timestamp: number;
}

// Specific interfaces for each sidecar type
interface WebViewerInfo extends BaseSidecarInfo {
  type: 'web-viewer';
  url: string;
  domain: string;
  isSecure: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  isLoading: boolean;
}

interface FileViewerInfo extends BaseSidecarInfo {
  type: 'file-viewer';
  filePath: string;
  fileName: string;
  fileSize: number;
  fileType: string;
  isReadable: boolean;
  lastModified: number;
}

interface DocumentEditorInfo extends BaseSidecarInfo {
  type: 'document-editor';
  filePath?: string;
  fileName: string;
  contentLength: number;
  hasUnsavedChanges: boolean;
  isNewDocument: boolean;
  language?: string;
}

interface LocalhostViewerInfo extends BaseSidecarInfo {
  type: 'localhost-viewer';
  url: string;
  port: number;
  protocol: 'http' | 'https';
  isLocal: boolean;
  serviceType?: string;
}

interface AppInstallerInfo extends BaseSidecarInfo {
  type: 'app-installer';
  availableAppsCount: number;
  installedAppsCount: number;
  currentView: 'browse' | 'installed' | 'search';
  searchQuery?: string;
}

interface DiffViewerInfo extends BaseSidecarInfo {
  type: 'diff-viewer';
  fileName: string;
  filePath?: string;
  addedLines: number;
  removedLines: number;
  totalChanges: number;
  viewMode: 'split' | 'unified';
}

// Union type for all sidecar info types
type SidecarInfo = WebViewerInfo | FileViewerInfo | DocumentEditorInfo | LocalhostViewerInfo | AppInstallerInfo | DiffViewerInfo;

interface UnifiedSidecarContextType {
  registerSidecar: (info: SidecarInfo) => void;
  updateSidecar: (id: string, updates: Partial<SidecarInfo>) => void;
  unregisterSidecar: (id: string) => void;
  getSidecarContext: () => string;
  getActiveSidecars: () => SidecarInfo[];
}

const UnifiedSidecarContext = createContext<UnifiedSidecarContextType | null>(null);

export const useUnifiedSidecarContext = () => {
  const context = useContext(UnifiedSidecarContext);
  return context;
};

export const useUnifiedSidecarContextOptional = () => {
  const context = useContext(UnifiedSidecarContext);
  return context || null;
};

interface UnifiedSidecarProviderProps {
  children: React.ReactNode;
}

export const UnifiedSidecarProvider: React.FC<UnifiedSidecarProviderProps> = ({ children }) => {
  // Debug component lifecycle
  React.useEffect(() => {
    console.log('🔍 UnifiedSidecarProvider: Component MOUNTED');
    return () => {
      console.log('🔍 UnifiedSidecarProvider: Component UNMOUNTING');
      console.trace('UnifiedSidecarProvider unmount stack trace');
    };
  }, []);
  
  // Use ref to store sidecar data to prevent re-renders
  const activeSidecarsRef = useRef<Map<string, SidecarInfo>>(new Map());
  
  // Use ref for the context value to maintain stable reference
  const contextValueRef = useRef<UnifiedSidecarContextType>({} as UnifiedSidecarContextType);

  const registerSidecar = useCallback((info: SidecarInfo) => {
    console.log('🔧 UnifiedSidecarContext: Registering sidecar:', info.id, info.type);
    activeSidecarsRef.current.set(info.id, { ...info, timestamp: Date.now() });
  }, []);

  const updateSidecar = useCallback((id: string, updates: Partial<SidecarInfo>) => {
    const existing = activeSidecarsRef.current.get(id);
    if (existing) {
      console.log('🔧 UnifiedSidecarContext: Updating sidecar:', id, updates);
      activeSidecarsRef.current.set(id, { ...existing, ...updates, timestamp: Date.now() });
    }
  }, []);

  const unregisterSidecar = useCallback((id: string) => {
    console.log('🔧 UnifiedSidecarContext: Unregistering sidecar:', id);
    activeSidecarsRef.current.delete(id);
  }, []);

  const getActiveSidecars = useCallback((): SidecarInfo[] => {
    return Array.from(activeSidecarsRef.current.values());
  }, []);

  const getSidecarContext = useCallback((): string => {
    const sidecars = Array.from(activeSidecarsRef.current.values());
    
    if (sidecars.length === 0) {
      return '';
    }

    // Sort by timestamp (most recent first)
    const sortedSidecars = sidecars.sort((a, b) => b.timestamp - a.timestamp);

    let contextParts: string[] = [];
    
    contextParts.push('## Active Tools & Context');
    contextParts.push('The user currently has the following tools and content open in sidecars:');
    contextParts.push('');

    sortedSidecars.forEach((sidecar, index) => {
      const contextInfo = generateSidecarContext(sidecar);
      if (contextInfo) {
        contextParts.push(`### ${index + 1}. ${contextInfo.title}`);
        contextParts.push(contextInfo.description);
        if (contextInfo.suggestions.length > 0) {
          contextParts.push('**Helpful actions:**');
          contextInfo.suggestions.forEach(suggestion => {
            contextParts.push(`- ${suggestion}`);
          });
        }
        contextParts.push('');
      }
    });

    contextParts.push('Use this context to provide more relevant assistance based on the tools and content the user is actively working with. Reference specific files, URLs, or content when relevant to help the user with their current workflow.');
    contextParts.push('');

    return contextParts.join('\n');
  }, []);

  // Initialize context value
  contextValueRef.current = {
    registerSidecar,
    updateSidecar,
    unregisterSidecar,
    getSidecarContext,
    getActiveSidecars,
  };

  // Expose globally for useMessageStream access
  (window as any).__unifiedSidecarContext = contextValueRef.current;

  return (
    <UnifiedSidecarContext.Provider value={contextValueRef.current}>
      {children}
    </UnifiedSidecarContext.Provider>
  );
};

// Helper function to generate context for each sidecar type
function generateSidecarContext(sidecar: SidecarInfo): {
  title: string;
  description: string;
  suggestions: string[];
} | null {
  switch (sidecar.type) {
    case 'web-viewer':
      const webViewer = sidecar as WebViewerInfo;
      return {
        title: `Web Browser - ${webViewer.title}`,
        description: `Currently viewing **${webViewer.url}** (${webViewer.domain}). ${webViewer.isSecure ? 'Secure HTTPS connection.' : 'HTTP connection.'} ${webViewer.isLoading ? 'Page is loading.' : 'Page loaded.'}`,
        suggestions: [
          'Help analyze or summarize the current webpage content',
          'Explain concepts or information from the current page',
          'Navigate to related resources or documentation',
          webViewer.canGoBack ? 'Go back to previous page' : '',
          webViewer.canGoForward ? 'Go forward to next page' : '',
        ].filter(Boolean),
      };

    case 'file-viewer':
      const fileViewer = sidecar as FileViewerInfo;
      const fileSizeKB = Math.round(fileViewer.fileSize / 1024);
      return {
        title: `File Viewer - ${fileViewer.fileName}`,
        description: `Viewing file **${fileViewer.filePath}** (${fileViewer.fileType}, ${fileSizeKB}KB). ${fileViewer.isReadable ? 'File is readable.' : 'File may be binary or unreadable.'} Last modified: ${new Date(fileViewer.lastModified).toLocaleDateString()}.`,
        suggestions: [
          'Analyze or explain the file content',
          'Suggest improvements or modifications',
          'Help with file format conversion',
          'Explain file structure or syntax',
          'Create related files or documentation',
        ],
      };

    case 'document-editor':
      const docEditor = sidecar as DocumentEditorInfo;
      return {
        title: `Document Editor - ${docEditor.fileName}`,
        description: `${docEditor.isNewDocument ? 'Creating new document' : `Editing **${docEditor.filePath}**`}. Document has ${docEditor.contentLength} characters. ${docEditor.hasUnsavedChanges ? '⚠️ Has unsaved changes.' : 'All changes saved.'} ${docEditor.language ? `Language: ${docEditor.language}.` : ''}`,
        suggestions: [
          'Help with writing, editing, or proofreading',
          'Suggest content structure or organization',
          'Generate related content or sections',
          docEditor.hasUnsavedChanges ? 'Remind to save changes' : '',
          'Format or style the document',
        ].filter(Boolean),
      };

    case 'localhost-viewer':
      const localhostViewer = sidecar as LocalhostViewerInfo;
      return {
        title: `Development Server - ${localhostViewer.title}`,
        description: `Viewing local development server at **${localhostViewer.url}** (port ${localhostViewer.port}). ${localhostViewer.protocol.toUpperCase()} connection. ${localhostViewer.serviceType ? `Service type: ${localhostViewer.serviceType}.` : ''}`,
        suggestions: [
          'Help debug or troubleshoot the application',
          'Suggest improvements to the UI or functionality',
          'Explain development server configuration',
          'Help with testing or development workflow',
          'Analyze application performance or behavior',
        ],
      };

    case 'app-installer':
      const appInstaller = sidecar as AppInstallerInfo;
      return {
        title: `App Store - ${appInstaller.title}`,
        description: `Browsing available applications. ${appInstaller.availableAppsCount} apps available, ${appInstaller.installedAppsCount} installed. Currently viewing: ${appInstaller.currentView}. ${appInstaller.searchQuery ? `Searching for: "${appInstaller.searchQuery}".` : ''}`,
        suggestions: [
          'Recommend useful apps for current workflow',
          'Help find specific tools or utilities',
          'Explain app installation process',
          'Compare different app options',
          'Suggest productivity or development tools',
        ],
      };

    case 'diff-viewer':
      const diffViewer = sidecar as DiffViewerInfo;
      return {
        title: `Code Diff - ${diffViewer.fileName}`,
        description: `Reviewing changes in **${diffViewer.filePath || diffViewer.fileName}**. ${diffViewer.addedLines} lines added, ${diffViewer.removedLines} lines removed (${diffViewer.totalChanges} total changes). Viewing in ${diffViewer.viewMode} mode.`,
        suggestions: [
          'Explain the changes being made',
          'Review code quality and suggest improvements',
          'Help understand the impact of changes',
          'Suggest additional tests or documentation',
          'Identify potential issues or conflicts',
        ],
      };

    default:
      return null;
  }
}

// Export types for use in other components
export type {
  SidecarInfo,
  WebViewerInfo,
  FileViewerInfo,
  DocumentEditorInfo,
  LocalhostViewerInfo,
  AppInstallerInfo,
  DiffViewerInfo,
};
