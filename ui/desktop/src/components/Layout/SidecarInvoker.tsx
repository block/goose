import React, { useState, useRef, useEffect } from 'react';
import { Plus, Globe, FileText, Edit, ExternalLink, Download, Code, Folder, Terminal, Monitor } from 'lucide-react';
import { Button } from '../ui/button';
import { useSidecar } from '../SidecarLayout';

interface SidecarInvokerProps {
  onShowLocalhost: () => void;
  onShowFileViewer: (filePath: string) => void;
  onAddContainer: (type: 'sidecar' | 'localhost' | 'file' | 'document-editor' | 'web-viewer' | 'app-installer', filePath?: string) => void;
  isVisible: boolean;
}

export const SidecarInvoker: React.FC<SidecarInvokerProps> = ({ 
  onShowLocalhost, 
  onShowFileViewer, 
  onAddContainer,
  isVisible 
}) => {
  // ALL HOOKS MUST BE CALLED BEFORE ANY CONDITIONAL LOGIC
  const [isHovering, setIsHovering] = useState(false);
  const [iframeBackdrops, setIframeBackdrops] = useState<any[]>([]);
  const containerRef = useRef<HTMLDivElement>(null);
  const sidecar = useSidecar();

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

  // NOW we can do conditional rendering after all hooks are called
  if (!isVisible) return null;

  const handleLocalhostClick = () => {
    console.log('🔍 SidecarInvoker: Localhost button clicked');
    onAddContainer('localhost');
    setIsHovering(false);
  };

  const handleFileViewerClick = async () => {
    try {
      console.log('File viewer button clicked');
      
      // Use Electron's selectFileOrDirectory API
      const filePath = await window.electron.selectFileOrDirectory();
      
      console.log('Selected file path:', filePath);
      
      if (filePath) {
        onAddContainer('file', filePath);
      } else {
        console.log('No file selected');
      }
    } catch (error) {
      console.error('Error opening file dialog:', error);
    }

    setIsHovering(false);
  };

  const handleSidecarClick = () => {
    console.log('🔍 SidecarInvoker: Sidecar button clicked');
    onAddContainer('sidecar');
    setIsHovering(false);
  };

  const handleDocumentEditorClick = () => {
    console.log('🔍 SidecarInvoker: Document Editor button clicked');
    onAddContainer('document-editor');
    setIsHovering(false);
  };

  const handleEditFileClick = async () => {
    try {
      console.log('Edit file button clicked');
      
      // Use Electron's selectFileOrDirectory API
      const filePath = await window.electron.selectFileOrDirectory();
      
      console.log('Selected file path for editing:', filePath);
      
      if (filePath) {
        onAddContainer('document-editor', filePath);
      } else {
        console.log('No file selected for editing');
      }
    } catch (error) {
      console.error('Error opening file dialog for editing:', error);
    }

    setIsHovering(false);
  };

  const handleWebViewerClick = () => {
    console.log('🔍 SidecarInvoker: Web Viewer button clicked');
    onAddContainer('web-viewer');
    setIsHovering(false);
  };

  const handleAppInstallerClick = () => {
    console.log('🔍 SidecarInvoker: App Installer button clicked');
    onAddContainer('app-installer');
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
      id: 'app-installer',
      name: 'App Store',
      icon: Download,
      color: 'from-blue-500 to-blue-600',
      onClick: handleAppInstallerClick,
      description: 'Install apps from GitHub'
    },
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
    }
  ];

  return (
    <>
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

      <div
        ref={containerRef}
        className="fixed top-0 left-0 z-[99999] pointer-events-none"
        style={{ width: isHovering ? '100px' : '20px', height: '100%' }}
      >
        {/* Hover detection zone */}
        <div
          className="absolute top-0 left-0 h-full pointer-events-auto"
          style={{ width: isHovering ? '100px' : '20px' }}
          onMouseEnter={handleMouseEnter}
          onMouseLeave={handleMouseLeave}
        >
        {/* macOS-style dock - with smooth enter/exit animations */}
        <div
          className={`absolute left-4 top-1/2 transform -translate-y-1/2 pointer-events-auto transition-all duration-300 ease-out ${
            isHovering 
              ? 'opacity-100 translate-x-0 scale-100' 
              : 'opacity-0 -translate-x-4 scale-95 pointer-events-none'
          }`}
          style={{ marginTop: '60px' }}
        >
          {/* Dock container with macOS styling */}
          <div className="bg-white/10 backdrop-blur-xl border border-white/20 rounded-2xl p-3 shadow-2xl">
            <div className="flex flex-col space-y-2">
              {dockApps.map((app, index) => (
                <div
                  key={app.id}
                  className="group relative"
                >
                  {/* App icon with staggered animations */}
                  <button
                    onClick={app.onClick}
                    className={`
                      w-12 h-12 rounded-xl bg-gradient-to-br ${app.color} 
                      shadow-lg hover:shadow-xl 
                      transform hover:scale-110 hover:-translate-y-1
                      transition-all duration-200 ease-out
                      flex items-center justify-center
                      border border-white/20
                      ${isHovering 
                        ? 'animate-in slide-in-from-left-2 fade-in' 
                        : 'animate-out slide-out-to-left-2 fade-out'
                      }
                    `}
                    style={{
                      animationDelay: isHovering ? `${index * 50}ms` : `${(dockApps.length - index - 1) * 30}ms`,
                      animationFillMode: 'both'
                    }}
                    title={app.name}
                  >
                    <app.icon className="w-6 h-6 text-white drop-shadow-sm" />
                  </button>

                  {/* Tooltip */}
                  <div className="absolute left-full ml-2 top-1/2 transform -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200 pointer-events-none">
                    <div className="bg-gray-900/90 backdrop-blur-sm text-white text-xs px-2 py-1 rounded-md whitespace-nowrap shadow-lg">
                      {app.description}
                      <div className="absolute right-full top-1/2 transform -translate-y-1/2 border-4 border-transparent border-r-gray-900/90"></div>
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
    </>
  );
};
