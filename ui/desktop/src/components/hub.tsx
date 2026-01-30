/**
 * Hub Component
 *
 * The Hub is the main landing page and entry point for the Goose Desktop application.
 * It serves as the welcome screen where users can start new conversations.
 *
 * Key Responsibilities:
 * - Displays SessionInsights to show session statistics and recent chats
 * - Provides a ChatInput for users to start new conversations
 * - Navigates to Pair with the submitted message to start a new conversation
 * - Ensures each submission from Hub always starts a fresh conversation
 *
 * Navigation Flow:
 * Hub (input submission) → Pair (new conversation with the submitted message)
 */

import { SessionInsights } from './sessions/SessionsInsights';
import ChatInput from './ChatInput';
import { ChatState } from '../types/chatState';
import { ContextManagerProvider } from './context_management/ContextManager';
import { Greeting } from './common/Greeting';
import React, { useEffect, useRef, useState } from 'react';
import 'react-toastify/dist/ReactToastify.css';
import { View, ViewOptions } from '../utils/navigationUtils';
import { useLocation } from 'react-router-dom';
import MatrixChat from './MatrixChat';
import { useMatrix } from '../contexts/MatrixContext';
import { HubThemeSelector, useHubTheme } from './HubThemeSelector';
import type { HubTheme } from '../types/hubTheme';
import { CyberpunkWidgets } from './CyberpunkWidgets';
import HackerASCIIText from './HackerASCIIText';
import { AmberGhostCursor } from './AmberGhostCursor';

// Helper functions for theme-based styling
const getWidthClass = (width: HubTheme['input']['width']) => {
  switch (width) {
    case 'narrow': return 'max-w-md';
    case 'medium': return 'max-w-2xl';
    case 'wide': return 'max-w-4xl';
    case 'full': return 'w-full max-w-none';
    default: return 'max-w-4xl';
  }
};

const getRoundedClass = (rounded: HubTheme['input']['rounded']) => {
  switch (rounded) {
    case 'none': return 'rounded-none';
    case 'sm': return 'rounded-sm';
    case 'md': return 'rounded-md';
    case 'lg': return 'rounded-lg';
    case 'xl': return 'rounded-xl';
    case '2xl': return 'rounded-2xl';
    case 'full': return 'rounded-full';
    default: return 'rounded-2xl';
  }
};

const getPaddingClass = (padding: HubTheme['layout']['padding']) => {
  switch (padding) {
    case 'none': return 'p-0';
    case 'sm': return 'p-4';
    case 'md': return 'p-6';
    case 'lg': return 'p-8';
    default: return 'p-8';
  }
};

const getPositionClasses = (position: HubTheme['input']['position']) => {
  switch (position) {
    case 'center':
      return {
        container: 'items-center justify-center',
        wrapper: 'w-full',
      };
    case 'bottom-left':
      return {
        container: 'items-end justify-start',
        wrapper: 'w-auto',
      };
    case 'bottom-right':
      return {
        container: 'items-end justify-end',
        wrapper: 'w-auto',
      };
    case 'top-left':
      return {
        container: 'items-start justify-start',
        wrapper: 'w-auto',
      };
    case 'top-right':
      return {
        container: 'items-start justify-end',
        wrapper: 'w-auto',
      };
    case 'bottom-full':
      return {
        container: 'items-end justify-center',
        wrapper: 'w-full',
      };
    default:
      return {
        container: 'items-center justify-center',
        wrapper: 'w-full',
      };
  }
};

const getGreetingPositionClasses = (position: HubTheme['greeting']['position']) => {
  switch (position) {
    case 'center':
      return 'items-center justify-center text-center';
    case 'top-left':
      return 'items-start justify-start text-left';
    case 'top-right':
      return 'items-start justify-end text-right';
    case 'bottom-left':
      return 'items-end justify-start text-left';
    case 'bottom-right':
      return 'items-end justify-end text-right';
    default:
      return 'items-center justify-center text-center';
  }
};

// ASCII Canvas Background Component
interface AsciiCanvasProps {
  className?: string;
  cellSize?: number;
  characters?: string;
  color?: string;
  backgroundColor?: string;
  animationSpeed?: number;
  glitchIntensity?: number;
}

const AsciiCanvas: React.FC<AsciiCanvasProps> = ({
  className = '',
  cellSize = 8,
  characters = ' .:-=+*#%@',
  color = '#00ff00',
  backgroundColor = 'transparent',
  animationSpeed = 0.02,
  glitchIntensity = 0.1,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number>();
  const timeRef = useRef(0);

  const drawAsciiFrame = React.useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set canvas size to match viewport
    const width = window.innerWidth;
    const height = window.innerHeight;
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }

    // Clear canvas
    if (backgroundColor !== 'transparent') {
      ctx.fillStyle = backgroundColor;
      ctx.fillRect(0, 0, canvas.width, canvas.height);
    } else {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
    }

    // Calculate grid dimensions
    const cols = Math.floor(canvas.width / cellSize);
    const rows = Math.floor(canvas.height / cellSize);

    // Set font for ASCII characters
    ctx.font = `${cellSize - 2}px monospace`;
    ctx.fillStyle = color;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';

    // Generate ASCII pattern
    for (let row = 0; row < rows; row++) {
      for (let col = 0; col < cols; col++) {
        // Create animated noise pattern
        const x = col * cellSize + cellSize / 2;
        const y = row * cellSize + cellSize / 2;
        
        // Generate noise value based on position and time
        const noiseX = (col + timeRef.current * 0.5) * 0.1;
        const noiseY = (row + timeRef.current * 0.3) * 0.1;
        const noise1 = Math.sin(noiseX) * Math.cos(noiseY);
        const noise2 = Math.sin(noiseX * 2.1 + timeRef.current) * Math.cos(noiseY * 1.7);
        const noise3 = Math.sin(noiseX * 0.8 + timeRef.current * 0.7) * Math.cos(noiseY * 2.3);
        
        // Combine noise values
        let intensity = (noise1 + noise2 * 0.5 + noise3 * 0.3) * 0.5 + 0.5;
        
        // Add some randomness for glitch effect
        if (Math.random() < glitchIntensity * 0.01) {
          intensity = Math.random();
        }
        
        // Map intensity to character
        const charIndex = Math.floor(intensity * characters.length);
        const char = characters[Math.min(charIndex, characters.length - 1)];
        
        // Vary opacity based on character
        const opacity = 0.1 + (intensity * 0.4);
        ctx.globalAlpha = opacity;
        
        // Add slight position jitter for glitch effect
        const jitterX = (Math.random() - 0.5) * glitchIntensity * 2;
        const jitterY = (Math.random() - 0.5) * glitchIntensity * 2;
        
        ctx.fillText(char, x + jitterX, y + jitterY);
      }
    }

    // Reset alpha
    ctx.globalAlpha = 1;

    // Add scanlines effect
    ctx.fillStyle = color;
    ctx.globalAlpha = 0.03;
    for (let y = 0; y < canvas.height; y += 4) {
      ctx.fillRect(0, y, canvas.width, 1);
    }
    
    ctx.globalAlpha = 1;

    // Update time
    timeRef.current += animationSpeed;
  }, [cellSize, characters, color, backgroundColor, animationSpeed, glitchIntensity]);

  const animate = React.useCallback(() => {
    drawAsciiFrame();
    animationRef.current = requestAnimationFrame(animate);
  }, [drawAsciiFrame]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    // Start animation
    animate();

    // Handle resize
    const handleResize = () => {
      canvas.width = window.innerWidth;
      canvas.height = window.innerHeight;
    };

    window.addEventListener('resize', handleResize);
    handleResize(); // Initial size

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
      window.removeEventListener('resize', handleResize);
    };
  }, [animate]);

  return (
    <canvas
      ref={canvasRef}
      className={`absolute inset-0 pointer-events-none ${className}`}
      style={{ 
        zIndex: 1,
        width: '100%',
        height: '100%'
      }}
    />
  );
};

export default function Hub({
  setView,
  setIsGoosehintsModalOpen,
  isExtensionsLoading,
  resetChat,
}: {
  setView: (view: View, viewOptions?: ViewOptions) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
  isExtensionsLoading: boolean;
  resetChat: () => void;
}) {
  const location = useLocation();
  const { isConnected, friends } = useMatrix();
  const [backgroundImage, setBackgroundImage] = React.useState<string | null>(null);
  const [showText, setShowText] = React.useState(true);
  const [blurOpacity, setBlurOpacity] = React.useState(1);
  
  // Theme management
  const { theme, themeId, setTheme } = useHubTheme();

  // Inject custom CSS from theme
  useEffect(() => {
    if (theme.customCSS) {
      const styleId = 'hub-theme-custom-css';
      let styleElement = document.getElementById(styleId) as HTMLStyleElement;
      
      if (!styleElement) {
        styleElement = document.createElement('style');
        styleElement.id = styleId;
        document.head.appendChild(styleElement);
      }
      
      styleElement.textContent = theme.customCSS;
      
      return () => {
        // Clean up when theme changes or component unmounts
        const element = document.getElementById(styleId);
        if (element) {
          element.remove();
        }
      };
    }
  }, [theme.customCSS]);

  // Load background image from localStorage
  useEffect(() => {
    const loadBackgroundImage = () => {
      const stored = localStorage.getItem('home_background_image');
      setBackgroundImage(stored);
    };

    // Load on mount
    loadBackgroundImage();

    // Listen for updates from settings
    const handleBackgroundUpdate = () => {
      loadBackgroundImage();
    };

    window.addEventListener('background-image-updated', handleBackgroundUpdate);
    return () => {
      window.removeEventListener('background-image-updated', handleBackgroundUpdate);
    };
  }, []);

  // Fade out blur when text disappears based on theme settings
  useEffect(() => {
    if (!theme.greeting.show) {
      setShowText(false);
      setBlurOpacity(0);
      return;
    }

    const hideTextTimer = setTimeout(() => {
      setShowText(false);
      setBlurOpacity(0);
    }, theme.greeting.fadeDelay);

    return () => {
      clearTimeout(hideTextTimer);
    };
  }, [theme]);

  // Check if we're in Matrix chat mode
  const routeState = location.state as ViewOptions | undefined;
  const isMatrixMode = routeState?.matrixMode || false;
  const matrixRoomId = routeState?.matrixRoomId;
  const matrixRecipientId = routeState?.matrixRecipientId;
  const useRegularChat = routeState?.useRegularChat || false;

  // Handle chat input submission - create new chat and navigate to pair
  const handleSubmit = (e: React.FormEvent) => {
    const customEvent = e as unknown as CustomEvent;
    const combinedTextFromInput = customEvent.detail?.value || '';

    if (combinedTextFromInput.trim()) {
      // Navigate to pair page with the message to be submitted
      // Pair will handle creating the new chat session
      resetChat();
      setView('pair', {
        disableAnimation: true,
        initialMessage: combinedTextFromInput,
      });
    }

    e.preventDefault();
  };

  // Handle closing Matrix chat and returning to normal Hub
  const handleCloseMatrixChat = () => {
    setView('chat', { resetChat: true });
  };

  // If we're in Matrix mode and have the required parameters
  if (isMatrixMode && matrixRoomId && isConnected) {
    // If useRegularChat is true, show a regular chat interface with Matrix integration
    if (useRegularChat) {
      console.log('🔄 Showing regular chat interface with Matrix integration');
      return (
        <ContextManagerProvider>
          <div className="relative flex flex-col h-full bg-background-default">
            {/* Header with back button and room info */}
            <div className="flex items-center gap-3 p-4 border-b border-border-default bg-background-muted">
              <button
                onClick={handleCloseMatrixChat}
                className="flex items-center gap-2 px-3 py-2 text-sm text-text-muted hover:text-text-default hover:bg-background-subtle rounded-lg transition-colors"
              >
                ← Back to Chat
              </button>
              <div className="flex-1">
                <h2 className="text-lg font-medium text-text-default">
                  Matrix Collaboration
                </h2>
                <p className="text-sm text-text-muted">
                  Room: {matrixRoomId} • Collaborating with {matrixRecipientId?.split(':')[0]?.substring(1) || 'Unknown'}
                </p>
              </div>
            </div>
            
            {/* Chat area */}
            <div className="flex-1 flex flex-col min-h-0">
              <div className="flex-1 p-6">
                <div className="text-center text-text-muted">
                  <p className="mb-4">🤝 You're now in a collaborative Matrix chat!</p>
                  <p className="text-sm">
                    Messages you send here will be shared with other participants in the Matrix room.
                    You can chat with both Goose and other collaborators.
                  </p>
                </div>
              </div>
              
              {/* Chat input at bottom */}
              <div className="p-4 border-t border-border-default">
                <ChatInput
                  sessionId={null}
                  handleSubmit={(e: React.FormEvent) => {
                    const customEvent = e as unknown as CustomEvent;
                    const message = customEvent.detail?.value || '';
                    console.log('📤 Sending Matrix message:', message);
                    // TODO: Integrate with Matrix sending logic
                    e.preventDefault();
                  }}
                  autoSubmit={false}
                  chatState={ChatState.Idle}
                  onStop={() => {}}
                  commandHistory={[]}
                  initialValue=""
                  setView={setView}
                  numTokens={0}
                  inputTokens={0}
                  outputTokens={0}
                  droppedFiles={[]}
                  onFilesProcessed={() => {}}
                  messages={[]}
                  setMessages={() => {}}
                  disableAnimation={false}
                  sessionCosts={undefined}
                  setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
                  isExtensionsLoading={isExtensionsLoading}
                  toolCount={0}
                />
              </div>
            </div>
          </div>
        </ContextManagerProvider>
      );
    }
    
    // Otherwise, render the Matrix chat popup (original behavior)
    return (
      <ContextManagerProvider>
        <div className="relative flex flex-col h-full bg-background-default">
          <MatrixChat
            roomId={matrixRoomId}
            recipientId={matrixRecipientId}
            onBack={handleCloseMatrixChat}
            className="h-full"
          />
        </div>
      </ContextManagerProvider>
    );
  }

  // Get position classes based on theme
  const inputPositionClasses = getPositionClasses(theme.input.position);
  const greetingPositionClasses = getGreetingPositionClasses(theme.greeting.position);
  const widthClass = getWidthClass(theme.input.width);
  const roundedClass = getRoundedClass(theme.input.rounded);
  const paddingClass = getPaddingClass(theme.layout.padding);

  return (
    <ContextManagerProvider>
      <div 
        className="relative flex flex-col h-full rounded-t-2xl overflow-hidden transition-colors duration-500"
        style={{
          background: theme.background.gradient || theme.background.color,
        }}
      >
        {/* Theme Selector Button - Top Right */}
        <div className="absolute top-6 right-6 z-50">
          <HubThemeSelector currentThemeId={themeId} onThemeChange={setTheme} />
        </div>

        {/* Background Image - Behind everything */}
        {backgroundImage && (
          <>
            <div 
              className="absolute inset-0 bg-cover bg-center bg-no-repeat"
              style={{ 
                backgroundImage: `url(${backgroundImage})`,
                zIndex: 0
              }}
            />
            {/* Radial blur overlay - creates pixelated blur in center with animation */}
            <div 
              className="absolute inset-0 backdrop-blur-md transition-opacity duration-1000"
              style={{ 
                maskImage: 'radial-gradient(circle at center, black 0%, black 30%, transparent 50%)',
                WebkitMaskImage: 'radial-gradient(circle at center, black 0%, black 30%, transparent 50%)',
                opacity: blurOpacity,
                zIndex: 0
              }}
            />
          </>
        )}
        
        {/* Animated ASCII Canvas Background */}
        {theme.ascii.enabled && (
          <AsciiCanvas 
            cellSize={theme.ascii.cellSize}
            characters={theme.ascii.characters}
            color={theme.ascii.color}
            backgroundColor="transparent"
            animationSpeed={theme.ascii.animationSpeed}
            glitchIntensity={theme.ascii.glitchIntensity}
          />
        )}
        

        
        {/* Cyberpunk Widgets - Only show for cyberpunk theme */}
        {themeId === 'cyberpunk' && <CyberpunkWidgets />}
        
        {/* Hacker ASCII Text - Only show for hacker theme */}
        {themeId === 'hacker' && <HackerASCIIText text="GOOSE" />}
        
        {/* Main Content Area - Theme-based layout */}
        <div className={`relative flex-1 flex flex-col ${paddingClass} ${theme.layout.contentAlignment === 'space-between' ? 'justify-between' : ''}`} style={{ zIndex: 10 }}>
          {/* Greeting - Theme-based position */}
          {theme.greeting.show && (
            <div className={`flex ${theme.layout.contentAlignment === 'space-between' ? 'justify-start' : greetingPositionClasses} transition-all duration-1000 ${theme.greeting.position === 'center' ? 'flex-1' : ''}`}>
              <div 
                className="transition-all duration-1000 overflow-hidden"
                style={{ 
                  opacity: showText ? 1 : 0,
                  maxHeight: showText ? '300px' : '0px',
                }}
              >
                {theme.greeting.position !== 'center' && theme.greeting.position.includes('top') && (
                  <div className="mb-2">
                    <img 
                      src="/logo.svg" 
                      alt="Logo" 
                      className="size-8 brightness-0 dark:invert"
                    />
                  </div>
                )}
                <Greeting className={theme.greeting.className} />
                {theme.greeting.position === 'center' && (
                  <p className="text-text-default text-lg mt-4">
                    Start a new conversation to get help with your projects
                  </p>
                )}
              </div>
            </div>
          )}

          {/* Chat Input - Theme-based position and styling */}
          <div className={`flex ${theme.layout.contentAlignment === 'space-between' ? (theme.input.position === 'bottom-left' ? 'justify-start' : 'justify-center') : inputPositionClasses.container} ${theme.greeting.show && theme.greeting.position === 'center' ? 'flex-1' : ''}`}>
            <div className={`${inputPositionClasses.wrapper} ${widthClass} ${theme.input.position === 'bottom-left' && theme.input.width === 'full' ? 'text-left' : ''}`}>
              {/* Custom prompt prefix for CLI themes */}
              {theme.input.prompt && (
                <div className="mb-2 font-mono text-sm" style={{ color: theme.input.borderColor || theme.ascii.color }}>
                  {theme.input.prompt}
                </div>
              )}
              
              <div className={`${theme.input.className} [&>div]:!${roundedClass} ${theme.input.position === 'bottom-left' && theme.input.width === 'full' ? '[&>div]:!mx-0 [&>div]:!ml-0' : ''}`}>
                <ChatInput
                  sessionId={null}
                  handleSubmit={handleSubmit}
                  autoSubmit={false}
                  chatState={ChatState.Idle}
                  onStop={() => {}}
                  commandHistory={[]}
                  initialValue=""
                  setView={setView}
                  numTokens={0}
                  inputTokens={0}
                  outputTokens={0}
                  droppedFiles={[]}
                  onFilesProcessed={() => {}}
                  messages={[]}
                  setMessages={() => {}}
                  disableAnimation={false}
                  sessionCosts={undefined}
                  setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
                  isExtensionsLoading={isExtensionsLoading}
                  toolCount={0}
                  themeStyles={theme.input.customStyles}
                  themeTypography={theme.typography}
                  themeAnimations={theme.animations}
                  themeEffects={theme.effects}
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </ContextManagerProvider>
  );
}
