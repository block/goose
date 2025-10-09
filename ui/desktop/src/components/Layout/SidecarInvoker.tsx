import React, { useState, useRef, useEffect } from 'react';
import { Plus, Globe, FileText } from 'lucide-react';
import { Button } from '../ui/button';

interface SidecarInvokerProps {
  onShowLocalhost: () => void;
  onShowFileViewer: (filePath: string) => void;
  isVisible: boolean;
}

export const SidecarInvoker: React.FC<SidecarInvokerProps> = ({ 
  onShowLocalhost, 
  onShowFileViewer, 
  isVisible 
}) => {
  const [isHovering, setIsHovering] = useState(false);
  const [showMenu, setShowMenu] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Handle click outside to close menu
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setShowMenu(false);
        setIsHovering(false);
      }
    };

    if (showMenu) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [showMenu]);

  if (!isVisible) return null;

  const handlePlusClick = () => {
    setShowMenu(!showMenu);
  };

  const handleLocalhostClick = () => {
    onShowLocalhost();
    setShowMenu(false);
    setIsHovering(false);
  };

  const handleFileViewerClick = async () => {
    try {
      console.log('File viewer button clicked');
      
      // Use Electron's selectFileOrDirectory API
      const filePath = await window.electron.selectFileOrDirectory();
      
      console.log('Selected file path:', filePath);
      
      if (filePath) {
        onShowFileViewer(filePath);
      } else {
        console.log('No file selected');
      }
    } catch (error) {
      console.error('Error opening file dialog:', error);
    }

    setShowMenu(false);
    setIsHovering(false);
  };

  const handleMouseEnter = () => {
    setIsHovering(true);
  };

  const handleMouseLeave = () => {
    // Only hide if menu is not open
    if (!showMenu) {
      setIsHovering(false);
    }
  };

  return (
    <div
      ref={containerRef}
      className="fixed top-0 right-0 z-50 pointer-events-none"
      style={{ width: showMenu ? '200px' : '16px', height: '100%' }}
    >
      {/* Hover detection zone - extends to cover menu area when open */}
      <div
        className="absolute top-0 right-0 h-full pointer-events-auto"
        style={{ width: showMenu ? '200px' : '16px' }}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
      >
        {/* Plus button container - positioned relative to hover zone */}
        <div className="absolute top-1/2 right-2 transform -translate-y-1/2">
          {/* Plus button - appears on hover or when menu is open */}
          <div
            className={`transition-all duration-300 ease-out ${
              isHovering || showMenu ? 'opacity-100 translate-x-0' : 'opacity-0 translate-x-2'
            }`}
          >
            <Button
              onClick={handlePlusClick}
              className="w-8 h-8 rounded-full bg-background-default border border-border-subtle shadow-lg hover:shadow-xl hover:scale-105 transition-all duration-200 pointer-events-auto"
              variant="ghost"
              size="sm"
            >
              <Plus className="w-4 h-4" />
            </Button>
          </div>

          {/* Floating menu - positioned to the left of the plus button */}
          {showMenu && (
            <div
              className="absolute right-full mr-3 top-1/2 transform -translate-y-1/2 bg-background-default border border-border-subtle rounded-lg shadow-xl p-2 min-w-[160px] pointer-events-auto animate-in fade-in slide-in-from-right-2 duration-200"
            >
              <div className="space-y-1">
                <Button
                  onClick={handleLocalhostClick}
                  className="w-full justify-start text-left hover:bg-background-medium transition-colors duration-150"
                  variant="ghost"
                  size="sm"
                >
                  <Globe className="w-4 h-4 mr-2" />
                  Localhost Viewer
                </Button>
                
                <Button
                  onClick={handleFileViewerClick}
                  className="w-full justify-start text-left hover:bg-background-medium transition-colors duration-150"
                  variant="ghost"
                  size="sm"
                >
                  <FileText className="w-4 h-4 mr-2" />
                  Open File
                </Button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
