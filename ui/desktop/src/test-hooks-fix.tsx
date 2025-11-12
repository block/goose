import React, { useState, useRef, useEffect } from 'react';

// Test component to verify the hooks fix
const TestSidecarInvoker: React.FC<{ isVisible: boolean }> = ({ isVisible }) => {
  // ALL HOOKS MUST BE CALLED BEFORE ANY CONDITIONAL LOGIC - FIXED VERSION
  const [isHovering, setIsHovering] = useState(false);
  const [iframeBackdrops, setIframeBackdrops] = useState<any[]>([]);
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

  // NOW we can do conditional rendering after all hooks are called
  if (!isVisible) return null;

  return (
    <div ref={containerRef} style={{ padding: '20px', border: '1px solid #ccc' }}>
      <h3>Test SidecarInvoker - Hooks Fix Applied</h3>
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
const TestApp: React.FC = () => {
  const [isVisible, setIsVisible] = useState(true);

  return (
    <div style={{ padding: '20px' }}>
      <h2>React Hooks Fix Test</h2>
      <button onClick={() => setIsVisible(!isVisible)}>
        Toggle Visibility (This used to cause "Rendered more hooks" error)
      </button>
      <p>Component visible: {isVisible.toString()}</p>
      <TestSidecarInvoker isVisible={isVisible} />
    </div>
  );
};

export default TestApp;
