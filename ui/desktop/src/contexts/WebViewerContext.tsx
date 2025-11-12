import React, { createContext, useContext, useRef, useCallback } from 'react';

export interface WebViewerInfo {
  id: string;
  url: string;
  title: string;
  domain: string;
  isSecure: boolean;
  isLocalhost: boolean;
  lastUpdated: Date;
  type: 'sidecar' | 'main'; // Type of webviewer (sidecar for sidebar, main for full screen)
}

interface WebViewerContextType {
  getActiveWebViewers: () => WebViewerInfo[];
  registerWebViewer: (info: WebViewerInfo) => void;
  updateWebViewer: (id: string, updates: Partial<WebViewerInfo>) => void;
  unregisterWebViewer: (id: string) => void;
  getWebViewerContext: () => string;
}

const WebViewerContext = createContext<WebViewerContextType | null>(null);

export const useWebViewerContext = () => {
  const context = useContext(WebViewerContext);
  if (!context) {
    throw new Error('useWebViewerContext must be used within a WebViewerProvider');
  }
  return context;
};

export const useWebViewerContextOptional = () => {
  return useContext(WebViewerContext);
};

export const WebViewerProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  // Use ref instead of state to avoid triggering re-renders
  const activeWebViewersRef = useRef<WebViewerInfo[]>([]);

  const getActiveWebViewers = useCallback(() => {
    return [...activeWebViewersRef.current]; // Return a copy to prevent external mutation
  }, []);

  const registerWebViewer = useCallback((info: WebViewerInfo) => {
    // Remove any existing webviewer with the same ID
    activeWebViewersRef.current = activeWebViewersRef.current.filter(viewer => viewer.id !== info.id);
    // Add the new webviewer
    activeWebViewersRef.current.push(info);
    console.log('[WebViewerContext] Registered:', info.id, 'Total:', activeWebViewersRef.current.length);
  }, []);

  const updateWebViewer = useCallback((id: string, updates: Partial<WebViewerInfo>) => {
    const index = activeWebViewersRef.current.findIndex(viewer => viewer.id === id);
    if (index !== -1) {
      activeWebViewersRef.current[index] = {
        ...activeWebViewersRef.current[index],
        ...updates,
        lastUpdated: new Date()
      };
      console.log('[WebViewerContext] Updated:', id);
    }
  }, []);

  const unregisterWebViewer = useCallback((id: string) => {
    const initialLength = activeWebViewersRef.current.length;
    activeWebViewersRef.current = activeWebViewersRef.current.filter(viewer => viewer.id !== id);
    console.log('[WebViewerContext] Unregistered:', id, 'Removed:', initialLength - activeWebViewersRef.current.length);
  }, []);

  const getWebViewerContext = useCallback((): string => {
    const activeWebViewers = activeWebViewersRef.current;
    
    if (activeWebViewers.length === 0) {
      return '';
    }

    const contextParts: string[] = [];
    
    // Group webviewers by type
    const sidecarViewers = activeWebViewers.filter(v => v.type === 'sidecar');
    const mainViewers = activeWebViewers.filter(v => v.type === 'main');

    if (sidecarViewers.length > 0) {
      contextParts.push('\n## Currently Open Sidecar Web Tools');
      contextParts.push('The user has the following web tools open in the sidecar (sidebar) that you can reference and help with:');
      
      sidecarViewers.forEach((viewer, index) => {
        const securityInfo = viewer.isSecure ? 'HTTPS (secure)' : 'HTTP (insecure)';
        const locationInfo = viewer.isLocalhost ? 'Local development server' : 'External website';
        
        contextParts.push(`\n${index + 1}. **${viewer.title || 'Untitled Page'}**`);
        contextParts.push(`   - URL: ${viewer.url}`);
        contextParts.push(`   - Domain: ${viewer.domain}`);
        contextParts.push(`   - Type: ${locationInfo} (${securityInfo})`);
        contextParts.push(`   - Last updated: ${viewer.lastUpdated.toLocaleTimeString()}`);
        
        // Add context-specific suggestions based on the URL/domain
        if (viewer.isLocalhost) {
          if (viewer.url.includes(':3000') || viewer.url.includes('localhost:3000')) {
            contextParts.push(`   - Context: This appears to be a React development server. You can help with React development, debugging, component issues, etc.`);
          } else if (viewer.url.includes(':8000') || viewer.url.includes(':8080')) {
            contextParts.push(`   - Context: This appears to be a local web server. You can help with web development, API testing, server configuration, etc.`);
          } else if (viewer.url.includes(':5000')) {
            contextParts.push(`   - Context: This might be a Flask/Python development server. You can help with Python web development, API debugging, etc.`);
          } else {
            contextParts.push(`   - Context: This is a local development server. You can help with development tasks, debugging, testing, etc.`);
          }
        } else {
          // External websites - provide context based on domain
          if (viewer.domain.includes('github.com')) {
            contextParts.push(`   - Context: GitHub repository or page. You can help with Git operations, code review, repository management, etc.`);
          } else if (viewer.domain.includes('stackoverflow.com')) {
            contextParts.push(`   - Context: Stack Overflow page. You can help explain solutions, debug similar issues, or provide alternative approaches.`);
          } else if (viewer.domain.includes('docs.') || viewer.domain.includes('documentation')) {
            contextParts.push(`   - Context: Documentation page. You can help explain concepts, provide examples, or clarify usage.`);
          } else {
            contextParts.push(`   - Context: External website. You can help with web-related tasks or provide information about this site.`);
          }
        }
      });
      
      contextParts.push('\n**How to help with sidecar tools:**');
      contextParts.push('- You can reference these tools when providing assistance');
      contextParts.push('- For localhost servers, you can help with development, debugging, and testing');
      contextParts.push('- For external sites, you can provide context-aware help based on what the user is viewing');
      contextParts.push('- Always mention when you\'re referencing information from their open sidecar tools');
    }

    if (mainViewers.length > 0) {
      contextParts.push('\n## Currently Open Main Web Tools');
      contextParts.push('The user has the following web tools open in the main view:');
      
      mainViewers.forEach((viewer, index) => {
        const securityInfo = viewer.isSecure ? 'HTTPS (secure)' : 'HTTP (insecure)';
        const locationInfo = viewer.isLocalhost ? 'Local development server' : 'External website';
        
        contextParts.push(`\n${index + 1}. **${viewer.title || 'Untitled Page'}**`);
        contextParts.push(`   - URL: ${viewer.url}`);
        contextParts.push(`   - Domain: ${viewer.domain}`);
        contextParts.push(`   - Type: ${locationInfo} (${securityInfo})`);
        contextParts.push(`   - Last updated: ${viewer.lastUpdated.toLocaleTimeString()}`);
      });
    }

    if (contextParts.length > 0) {
      contextParts.push('\n---\n');
    }

    return contextParts.join('\n');
  }, []);

  // Create stable context value that doesn't change on every render
  const contextValue = useRef<WebViewerContextType>({
    getActiveWebViewers,
    registerWebViewer,
    updateWebViewer,
    unregisterWebViewer,
    getWebViewerContext,
  });

  // Expose context globally so useMessageStream can access it
  React.useEffect(() => {
    (window as any).__webViewerContext = contextValue.current;
    return () => {
      delete (window as any).__webViewerContext;
    };
  }, []);

  return (
    <WebViewerContext.Provider value={contextValue.current}>
      {children}
    </WebViewerContext.Provider>
  );
};
