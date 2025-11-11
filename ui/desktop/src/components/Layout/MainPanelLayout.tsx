import React, { useState, useCallback, useRef, useEffect } from 'react';
import { Plus, X, Globe, FileText, Edit, ExternalLink } from 'lucide-react';
import { Button } from '../ui/button';
import SidecarTabs from '../SidecarTabs';
import { FileViewer } from '../FileViewer';
import DocumentEditor from '../DocumentEditor';
import WebViewer from '../WebViewer';
import AppInstaller from '../AppInstaller';
import { EnhancedBentoBox, SidecarContainer } from './EnhancedBentoBox';



// ResizeHandle component for horizontal resizing between panels
const ResizeHandle: React.FC<{
  onResize: (delta: number) => void;
  isResizing: boolean;
}> = ({ onResize, isResizing }) => {
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    let startX = e.clientX;
    
    const handleMouseMove = (e: MouseEvent) => {
      const delta = e.clientX - startX;
      onResize(delta);
      startX = e.clientX;
    };

    const handleMouseUp = () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [onResize]);

  return (
    <div 
      className={`w-1 cursor-col-resize hover:bg-borderSubtle transition-colors group ${
        isResizing ? 'bg-borderProminent' : ''
      }`}
      onMouseDown={handleMouseDown}
    >
      <div 
        className={`h-8 w-0.5 bg-border-subtle group-hover:bg-border-strong rounded-full transition-colors my-auto ml-0.5 ${
          isResizing ? 'bg-border-strong' : ''
        }`} 
      />
    </div>
  );
};

export const MainPanelLayout: React.FC<{
  children: React.ReactNode;
  removeTopPadding?: boolean;
  backgroundColor?: string;
}> = ({ children, removeTopPadding = false, backgroundColor = 'bg-background-default' }) => {
  
  // Simplified state - just track if we have a bento box and what's in it
  const [hasBentoBox, setHasBentoBox] = useState(false);
  const [bentoBoxContainers, setBentoBoxContainers] = useState<SidecarContainer[]>([]);
  const [chatWidth, setChatWidth] = useState(600);

  // Create or show the bento box
  const createBentoBox = useCallback(() => {
    if (!hasBentoBox) {
      setHasBentoBox(true);
      // Start with one container
      const initialContainer: SidecarContainer = {
        id: `bento-${Date.now()}`,
        content: (
          <div className="h-full w-full flex items-center justify-center text-text-muted bg-background-muted border border-border-subtle rounded-lg">
            <p>Sidecar content will go here</p>
          </div>
        ),
        contentType: 'sidecar',
        title: 'Sidecar'
      };
      setBentoBoxContainers([initialContainer]);
    }
  }, [hasBentoBox]);

  // Add content to bento box
  const addToBentoBox = useCallback((contentType: 'sidecar' | 'localhost' | 'file' | 'document-editor' | 'web-viewer' | 'app-installer', filePath?: string, url?: string, title?: string) => {
    const newContainer: SidecarContainer = {
      id: `bento-${Date.now()}`,
      content: null,
      contentType: null
    };

    // Create content based on type
    if (contentType === 'sidecar') {
      newContainer.content = (
        <div className="h-full w-full flex items-center justify-center text-text-muted bg-background-muted border border-border-subtle rounded-lg">
          <p>Sidecar content will go here</p>
        </div>
      );
      newContainer.contentType = 'sidecar';
      newContainer.title = 'Sidecar';
    } else if (contentType === 'localhost') {
      newContainer.content = <SidecarTabs initialUrl="http://localhost:3000" />;
      newContainer.contentType = 'localhost';
      newContainer.title = 'Localhost Viewer';
    } else if (contentType === 'file' && filePath) {
      newContainer.content = <FileViewer filePath={filePath} />;
      newContainer.contentType = 'file';
      newContainer.title = filePath?.split('/').pop() || 'File Viewer';
    } else if (contentType === 'document-editor') {
      const fileName = filePath ? filePath.split('/').pop() || filePath : 'Untitled Document';
      newContainer.content = <DocumentEditor filePath={filePath} placeholder="Start writing your document..." />;
      newContainer.contentType = 'document-editor';
      newContainer.title = fileName;
    } else if (contentType === 'web-viewer') {
      // Use the URL from the event if provided, otherwise default to Google
      const initialUrl = url || "https://google.com";
      const containerTitle = title || 'Web Viewer';
      console.log('🔍 Creating WebViewer with URL:', initialUrl, 'and title:', containerTitle);
      newContainer.content = <WebViewer initialUrl={initialUrl} allowAllSites={true} />;
      newContainer.contentType = 'web-viewer';
      newContainer.title = containerTitle;
    } else if (contentType === 'app-installer') {
      newContainer.content = <AppInstaller />;
      newContainer.contentType = 'app-installer';
      newContainer.title = 'App Installer';
    }

    // If no bento box exists, create it first
    if (!hasBentoBox) {
      setHasBentoBox(true);
      setBentoBoxContainers([newContainer]);
    } else {
      // Add to existing bento box
      setBentoBoxContainers(prev => [...prev, newContainer]);
    }
  }, [hasBentoBox]);

  // Remove from bento box
  const removeFromBentoBox = useCallback((containerId: string) => {
    console.log('🔍 MainPanelLayout: removeFromBentoBox called with ID:', containerId);
    setBentoBoxContainers(prev => {
      console.log('🔍 MainPanelLayout: Current containers before removal:', prev.length);
      const updated = prev.filter(c => c.id !== containerId);
      console.log('🔍 MainPanelLayout: Containers after removal:', updated.length);
      
      // If no containers left, hide the bento box
      if (updated.length === 0) {
        console.log('🔍 MainPanelLayout: No containers left, hiding bento box');
        setHasBentoBox(false);
      }
      return updated;
    });
  }, []);

  // Handle chat panel resize
  const updateChatWidth = useCallback((delta: number) => {
    setChatWidth(prev => Math.max(300, Math.min(1000, prev + delta)));
  }, []);

  // Listen for add-container events from SidecarInvoker
  useEffect(() => {
    const handleAddContainer = (e: CustomEvent<{ type: 'sidecar' | 'localhost' | 'file' | 'document-editor' | 'web-viewer' | 'app-installer'; filePath?: string; url?: string; title?: string }>) => {
      console.log('🔍 MainPanelLayout: Received add-container event:', e.detail.type, e.detail.filePath, e.detail.url);
      addToBentoBox(e.detail.type, e.detail.filePath, e.detail.url, e.detail.title);
    };

    window.addEventListener('add-container', handleAddContainer as EventListener);
    return () => window.removeEventListener('add-container', handleAddContainer as EventListener);
  }, [addToBentoBox]);

  return (
    <div className="h-dvh">
      <div
        className={`flex ${backgroundColor} flex-1 min-w-0 h-full min-h-0 ${removeTopPadding ? '' : 'pt-[32px]'}`}
      >
        {/* Chat Panel - Full width when no bento box, fixed width when bento box exists */}
        <div 
          className={hasBentoBox ? "flex flex-col flex-shrink-0" : "flex flex-col flex-1"}
          style={hasBentoBox ? { width: `${chatWidth}px` } : {}}
        >
          {children}
        </div>

        {/* Chat Resize Handle - only show when bento box exists */}
        {hasBentoBox && (
          <ResizeHandle
            onResize={updateChatWidth}
            isResizing={false}
          />
        )}

        {/* Enhanced Bento Box - Single container that holds all sidecars */}
        {hasBentoBox && (
          <EnhancedBentoBox
            containers={bentoBoxContainers}
            onRemoveContainer={removeFromBentoBox}
            onAddContainer={addToBentoBox}
            onReorderContainers={setBentoBoxContainers}
          />
        )}
      </div>
    </div>
  );
};
