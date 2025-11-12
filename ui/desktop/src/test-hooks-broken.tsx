import React, { useState, useRef, useEffect } from 'react';

// Test component showing the BROKEN version (for comparison)
const BrokenSidecarInvoker: React.FC<{ isVisible: boolean }> = ({ isVisible }) => {
  // Some hooks called before conditional return
  const [isHovering, setIsHovering] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

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

  // PROBLEM: Early return before all hooks are called
  if (!isVisible) return null;

  // PROBLEM: This hook is called AFTER the conditional return above
  // This causes "Rendered more hooks than during the previous render" error
  const [iframeBackdrops, setIframeBackdrops] = useState<any[]>([]);

  return (
    <div ref={containerRef} style={{ padding: '20px', border: '1px solid #f00' }}>
      <h3>BROKEN SidecarInvoker - Hooks Issue</h3>
      <p>isHovering: {isHovering.toString()}</p>
      <p>iframeBackdrops count: {iframeBackdrops.length}</p>
      <button onClick={() => setIsHovering(!isHovering)}>
        Toggle Hovering
      </button>
      <button onClick={() => setIframeBackdrops([...iframeBackdrops, { id: Date.now() }])}>
        Add Backdrop
      </button>
    </div>
  );
};

// Test app that toggles visibility to trigger the hooks issue
const BrokenTestApp: React.FC = () => {
  const [isVisible, setIsVisible] = useState(true);

  return (
    <div style={{ padding: '20px' }}>
      <h2>React Hooks BROKEN Version Test</h2>
      <button onClick={() => setIsVisible(!isVisible)}>
        Toggle Visibility (This WILL cause "Rendered more hooks" error)
      </button>
      <p>Component visible: {isVisible.toString()}</p>
      <BrokenSidecarInvoker isVisible={isVisible} />
    </div>
  );
};

export default BrokenTestApp;
