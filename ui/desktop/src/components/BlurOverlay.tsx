import React, { useState, useEffect } from 'react';

interface BlurOverlayProps {
  isActive: boolean;
  isDarkMode?: boolean;
  intensity?: number;
}

const BlurOverlay: React.FC<BlurOverlayProps> = ({ 
  isActive, 
  isDarkMode = true,
  intensity = 20 // Default blur intensity
}) => {
  const [isVisible, setIsVisible] = useState(false);
  const [opacity, setOpacity] = useState(0);

  // Use two-step animation for smoother transition
  useEffect(() => {
    let timeout: NodeJS.Timeout;
    
    if (isActive) {
      setIsVisible(true);
      // Small delay before starting the opacity transition
      timeout = setTimeout(() => {
        setOpacity(1);
      }, 10);
    } else {
      setOpacity(0);
      // Wait for transition to complete before hiding
      timeout = setTimeout(() => {
        setIsVisible(false);
      }, 300); // Match the transition duration
    }
    
    return () => {
      if (timeout) clearTimeout(timeout);
    };
  }, [isActive]);

  if (!isVisible) return null;

  return (
    <div 
      className="fixed inset-0 pointer-events-none z-40 transition-all duration-300 ease-in-out"
      style={{ 
        backdropFilter: `blur(${opacity * intensity}px)`,
        backgroundColor: isDarkMode 
          ? `rgba(0, 0, 0, ${opacity * 0.7})` 
          : `rgba(255, 255, 255, ${opacity * 0.7})`,
        opacity: opacity
      }}
    />
  );
};

export default BlurOverlay;
