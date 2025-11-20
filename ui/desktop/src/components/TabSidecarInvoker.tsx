import React, { useState, useRef, useEffect } from 'react';
import { Plus, Globe, FileText, Edit, ExternalLink, Download, Code, Folder, Terminal, Monitor } from 'lucide-react';
import { Button } from './ui/button';
import { useTabContext } from '../contexts/TabContext';

interface TabSidecarInvokerProps {
  tabId: string;
  isVisible: boolean;
}

export const TabSidecarInvoker: React.FC<TabSidecarInvokerProps> = ({ 
  tabId,
  isVisible 
}) => {
  // ALL HOOKS MUST BE CALLED BEFORE ANY CONDITIONAL LOGIC
  const [isHovering, setIsHovering] = useState(false);
  const [iframeBackdrops, setIframeBackdrops] = useState<any[]>([]);
  const containerRef = useRef<HTMLDivElement>(null);
  
  // Get tab-specific sidecar functions
  const {
    showLocalhostViewer,
    showWebViewer,
    showFileViewer,
    showDocumentEditor,
    showDiffViewer
  } = useTabContext();

  // Handle click outside to close dock
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsHovering(false);
      }
    };

    if (isHovering) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [isHovering]);

  // Component is always rendered but hidden/shown with CSS transitions

  const handleLocalhostClick = () => {
    console.log('🔍 TabSidecarInvoker: Localhost button clicked for tab:', tabId);
    showLocalhostViewer(tabId, 'http://localhost:3000', 'Localhost Viewer');
    setIsHovering(false);
  };

  const handleFileViewerClick = async () => {
    try {
      console.log('File viewer button clicked for tab:', tabId);
      
      // Use Electron's selectFileOrDirectory API
      const filePath = await window.electron.selectFileOrDirectory();
      
      console.log('Selected file path:', filePath);
      
      if (filePath) {
        showFileViewer(tabId, filePath);
      } else {
        console.log('No file selected');
      }
    } catch (error) {
      console.error('Error opening file dialog:', error);
    }

    setIsHovering(false);
  };

  const handleDocumentEditorClick = () => {
    console.log('🔍 TabSidecarInvoker: Document Editor button clicked for tab:', tabId);
    showDocumentEditor(tabId, undefined, 'Start writing your document...', 'new-doc');
    setIsHovering(false);
  };

  const handleEditFileClick = async () => {
    try {
      console.log('Edit file button clicked for tab:', tabId);
      
      // Use Electron's selectFileOrDirectory API
      const filePath = await window.electron.selectFileOrDirectory();
      
      console.log('Selected file path for editing:', filePath);
      
      if (filePath) {
        showDocumentEditor(tabId, filePath, undefined, 'edit-file');
      } else {
        console.log('No file selected for editing');
      }
    } catch (error) {
      console.error('Error opening file dialog for editing:', error);
    }

    setIsHovering(false);
  };

  const handleWebViewerClick = () => {
    console.log('🔍 TabSidecarInvoker: Web Viewer button clicked for tab:', tabId);
    showWebViewer(tabId, 'https://google.com', 'Web Browser', 'web-viewer');
    setIsHovering(false);
  };

  const handleDiffViewerClick = () => {
    console.log('🔍 TabSidecarInvoker: Diff Viewer button clicked for tab:', tabId);
    // For demo purposes, show a sample diff
    const sampleDiff = `--- a/example.js
+++ b/example.js
@@ -1,3 +1,4 @@
 function hello() {
+  console.log("Hello world!");
   return "Hello";
 }`;
    showDiffViewer(tabId, sampleDiff, 'example.js', 'sample-diff');
    setIsHovering(false);
  };

  const handleMouseEnter = async () => {
    setIsHovering(true);
    // Create iframe backdrops to show live content behind the dock
    try {
      const result = await window.electron.createIframeBackdrop();
      if (result.success && result.backdropData) {
        setIframeBackdrops(result.backdropData);
        console.log('🎬 Created iframe backdrops:', result.backdropData.length);
      }
    } catch (error) {
      console.error('Failed to create iframe backdrop:', error);
    }
  };

  const handleMouseLeave = async () => {
    setIsHovering(false);
    // Remove iframe backdrops and restore BrowserViews
    try {
      await window.electron.removeIframeBackdrop();
      setIframeBackdrops([]);
      console.log('🎬 Removed iframe backdrops');
    } catch (error) {
      console.error('Failed to remove iframe backdrop:', error);
    }
  };

  // Define dock apps with proper icons and colors
  const dockApps = [
    {
      id: 'document-editor',
      name: 'TextEdit',
      icon: Edit,
      color: 'from-gray-400 to-gray-600',
      onClick: handleDocumentEditorClick,
      description: 'Create new document'
    },
    {
      id: 'file-editor',
      name: 'Code Editor',
      icon: Code,
      color: 'from-indigo-500 to-purple-600',
      onClick: handleEditFileClick,
      description: 'Edit existing file'
    },
    {
      id: 'web-viewer',
      name: 'Safari',
      icon: Monitor,
      color: 'from-blue-400 to-cyan-500',
      onClick: handleWebViewerClick,
      description: 'Browse the web'
    },
    {
      id: 'localhost',
      name: 'Terminal',
      icon: Terminal,
      color: 'from-gray-800 to-black',
      onClick: handleLocalhostClick,
      description: 'View localhost apps'
    },
    {
      id: 'file-viewer',
      name: 'Finder',
      icon: Folder,
      color: 'from-blue-500 to-blue-700',
      onClick: handleFileViewerClick,
      description: 'Browse files'
    },
    {
      id: 'diff-viewer',
      name: 'Diff',
      icon: FileText,
      color: 'from-green-500 to-green-600',
      onClick: handleDiffViewerClick,
      description: 'View code diffs'
    }
  ];

  return (
    <div
      ref={containerRef}
      className={`absolute bottom-full left-0 right-0 z-50 transition-all duration-300 ease-out ${
        isVisible 
          ? 'opacity-100 translate-y-0' 
          : 'opacity-0 -translate-y-2 pointer-events-none'
      }`}
      style={{ marginBottom: '-16px' }} // Position dock closer to the floating input
    >
      {/* Screenshot backdrops - positioned behind the dock */}
      {iframeBackdrops.map((backdrop) => (
        <div
          key={backdrop.viewId}
          className="fixed pointer-events-none"
          style={{
            left: backdrop.bounds.x,
            top: backdrop.bounds.y,
            width: backdrop.bounds.width,
            height: backdrop.bounds.height,
            zIndex: 99998, // Just below the dock
            backgroundImage: `url(${backdrop.screenshot})`,
            backgroundSize: 'cover',
            backgroundPosition: 'center',
            backgroundRepeat: 'no-repeat',
          }}
          title={`Screenshot backdrop for ${backdrop.viewId}`}
        />
      ))}

      {/* Horizontal dock above chat input */}
      <div className="flex justify-start">
        <div
          className={`transition-all duration-300 ease-out ${
            isVisible 
              ? 'opacity-100 scale-100' 
              : 'opacity-80 scale-95'
          }`}
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
        >
          {/* Dock container - horizontal layout with proper padding */}
          <div className="px-6 py-3">
            <div className="flex flex-row space-x-2">
              {dockApps.map((app, index) => (
                <div
                  key={app.id}
                  className="group relative"
                >
                  {/* App icon with staggered animations */}
                  <button
                    onClick={app.onClick}
                    className={`
                      w-10 h-10 rounded-xl bg-gradient-to-br ${app.color} 
                      shadow-lg hover:shadow-xl 
                      transform hover:scale-110 hover:-translate-y-1
                      transition-all duration-200 ease-out
                      flex items-center justify-center
                      border border-white/20
                      ${isVisible 
                        ? 'animate-in slide-in-from-bottom-2 fade-in' 
                        : 'animate-out slide-out-to-bottom-2 fade-out'
                      }
                    `}
                    style={{
                      animationDelay: isVisible ? `${index * 50}ms` : `${(dockApps.length - index - 1) * 30}ms`,
                      animationFillMode: 'both'
                    }}
                    title={app.name}
                  >
                    <app.icon className="w-5 h-5 text-white drop-shadow-sm" />
                  </button>

                  {/* Tooltip */}
                  <div className="absolute bottom-full mb-2 left-1/2 transform -translate-x-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none">
                    <div className="bg-gray-900/90 backdrop-blur-sm text-white text-xs px-2 py-1 rounded-md whitespace-nowrap shadow-lg">
                      {app.description}
                      <div className="absolute top-full left-1/2 transform -translate-x-1/2 border-4 border-transparent border-t-gray-900/90"></div>
                    </div>
                  </div>

                  {/* Active indicator dot (like macOS dock) */}
                  <div className="absolute -bottom-1 left-1/2 transform -translate-x-1/2 w-1 h-1 bg-white/60 rounded-full opacity-0 group-hover:opacity-100 transition-opacity duration-200"></div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
