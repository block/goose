import { useRef, useState, useEffect, useCallback } from 'react';
import { RefreshCw, ExternalLink, ChevronLeft, ChevronRight, Home, Globe, Shield, ShieldOff } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from './ui/Tooltip';
import { useWebViewerContextOptional } from '../contexts/WebViewerContext';
import { useUnifiedSidecarContextOptional } from '../contexts/UnifiedSidecarContext';

// Global registry to persist child windows across component re-mounts
class ChildWindowRegistry {
  private static instance: ChildWindowRegistry;
  private windows = new Map<string, {
    windowId: string;
    url: string;
    refCount: number;
    lastUsed: number;
    componentId: string; // Track which component instance owns this window
  }>();

  static getInstance(): ChildWindowRegistry {
    if (!ChildWindowRegistry.instance) {
      ChildWindowRegistry.instance = new ChildWindowRegistry();
    }
    return ChildWindowRegistry.instance;
  }

  // Generate a stable key for a WebViewer instance using component ID to ensure uniqueness
  private getKey(componentId: string): string {
    return `window-${componentId}`;
  }

  // Register a new window for a specific component instance
  registerWindow(componentId: string, initialUrl: string, allowAllSites: boolean, windowId?: string): string {
    const key = this.getKey(componentId);
    const existing = this.windows.get(key);

    if (existing) {
      // This component already has a window registered, just update ref count
      existing.refCount++;
      existing.lastUsed = Date.now();
      existing.url = initialUrl; // Update URL in case it changed
      console.log(`[ChildWindowRegistry] Reusing existing window for component ${componentId}, refCount: ${existing.refCount}`);
      return existing.windowId;
    } else {
      // Register new window for this component
      const newWindowId = windowId || `webviewer-window-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      this.windows.set(key, {
        windowId: newWindowId,
        url: initialUrl,
        refCount: 1,
        lastUsed: Date.now(),
        componentId: componentId
      });
      console.log(`[ChildWindowRegistry] Registered new window for component ${componentId}, windowId: ${newWindowId}`);
      return newWindowId;
    }
  }

  // Unregister a window (decrease ref count) - NEVER auto-destroy
  unregisterWindow(componentId: string): boolean {
    const key = this.getKey(componentId);
    const existing = this.windows.get(key);

    if (existing) {
      existing.refCount--;
      console.log(`[ChildWindowRegistry] Unregistered window for component ${componentId}, refCount: ${existing.refCount}`);
      
      // NEVER auto-destroy windows - they should only be destroyed by explicit user action
      // Keep the window in registry even with 0 refs for potential reuse
      console.log(`[ChildWindowRegistry] Window for component ${componentId} kept alive (refCount: ${existing.refCount})`);
    }
    
    return false; // NEVER auto-destroy
  }

  // Check if a window exists for a component
  hasWindow(componentId: string): boolean {
    const key = this.getKey(componentId);
    return this.windows.has(key);
  }

  // Get window ID if it exists for a component
  getWindowId(componentId: string): string | null {
    const key = this.getKey(componentId);
    const existing = this.windows.get(key);
    return existing ? existing.windowId : null;
  }

  // Cleanup old unused windows (called periodically) - VERY conservative cleanup
  cleanup(): void {
    const now = Date.now();
    const maxAge = 30 * 60 * 1000; // 30 minutes - much longer to account for navigation
    
    console.log(`[ChildWindowRegistry] Running cleanup check for ${this.windows.size} windows`);

    for (const [key, windowInfo] of this.windows.entries()) {
      const age = now - windowInfo.lastUsed;
      
      // Only cleanup windows that have been unused for a very long time AND have no references
      // This prevents cleanup during normal navigation
      if (windowInfo.refCount <= 0 && age > maxAge) {
        console.log(`[ChildWindowRegistry] Cleaning up old window ${key} (unused for ${Math.round(age / 60000)} minutes)`);
        this.windows.delete(key);
        // Destroy the actual window
        if (typeof window !== 'undefined' && (window as any).electron) {
          (window as any).electron.destroyChildWebViewer(windowInfo.windowId).catch(console.error);
        }
      } else if (windowInfo.refCount <= 0) {
        console.log(`[ChildWindowRegistry] Keeping window ${key} (unused for ${Math.round(age / 60000)} minutes, under ${maxAge / 60000} minute threshold)`);
      }
    }
  }

  // Force cleanup all windows (called when parent window is closing)
  forceCleanupAll(): void {
    console.log(`[ChildWindowRegistry] Force cleaning up all ${this.windows.size} windows`);
    
    for (const [key, windowInfo] of this.windows.entries()) {
      console.log(`[ChildWindowRegistry] Force destroying window ${key} (${windowInfo.windowId})`);
      
      // Destroy the actual window
      if (typeof window !== 'undefined' && (window as any).electron) {
        (window as any).electron.destroyChildWebViewer(windowInfo.windowId).catch(console.error);
      }
    }
    
    // Clear the registry
    this.windows.clear();
    console.log(`[ChildWindowRegistry] Force cleanup completed`);
  }

  // Destroy a specific window by component ID (called when user explicitly removes container)
  destroyWindowByComponentId(componentId: string): void {
    const key = this.getKey(componentId);
    const existing = this.windows.get(key);

    if (existing) {
      console.log(`[ChildWindowRegistry] Explicitly destroying window ${key} (${existing.windowId})`);
      
      // Remove from registry
      this.windows.delete(key);
      
      // Destroy the actual window
      if (typeof window !== 'undefined' && (window as any).electron) {
        (window as any).electron.destroyChildWebViewer(existing.windowId).catch(console.error);
      }
      
      console.log(`[ChildWindowRegistry] Window ${key} explicitly destroyed`);
    } else {
      console.log(`[ChildWindowRegistry] No window found for ${key} to destroy`);
    }
  }

  // Find and destroy window by URL (for backward compatibility with container removal)
  destroyWindowByUrl(initialUrl: string, allowAllSites: boolean): void {
    // Find the window with matching URL
    for (const [key, windowInfo] of this.windows.entries()) {
      if (windowInfo.url === initialUrl) {
        console.log(`[ChildWindowRegistry] Found window with URL ${initialUrl}, destroying component ${windowInfo.componentId}`);
        this.destroyWindowByComponentId(windowInfo.componentId);
        return;
      }
    }
    console.log(`[ChildWindowRegistry] No window found with URL ${initialUrl} to destroy`);
  }
}

const childWindowRegistry = ChildWindowRegistry.getInstance();

// Expose the registry globally for container removal
if (typeof window !== 'undefined') {
  (window as any).childWindowRegistry = childWindowRegistry;
}

interface WebViewerProps {
  initialUrl?: string;
  onUrlChange?: (url: string) => void;
  allowAllSites?: boolean; // If false, restricts to localhost only
}

function isValidUrl(url: string): boolean {
  try {
    new URL(url);
    return true;
  } catch {
    return false;
  }
}

function isLocalhostUrl(url: string): boolean {
  try {
    const parsedUrl = new URL(url);
    return (
      parsedUrl.protocol === 'http:' &&
      (parsedUrl.hostname === 'localhost' || parsedUrl.hostname === '127.0.0.1')
    );
  } catch {
    return false;
  }
}

function formatUrl(input: string, allowAllSites: boolean = true): string {
  const trimmed = input.trim();
  
  // If it's just a port number, prepend localhost
  if (/^\d+$/.test(trimmed)) {
    return `http://localhost:${trimmed}`;
  }

  // If it starts with localhost: or 127.0.0.1: without protocol
  if (/^(localhost|127\.0\.0\.1):\d+/.test(trimmed)) {
    return `http://${trimmed}`;
  }

  // If it's already a complete URL, return as-is
  if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
    return trimmed;
  }

  if (allowAllSites) {
    // Check if it looks like a search query (contains spaces or no dots)
    if (trimmed.includes(' ') || (!trimmed.includes('.') && !trimmed.includes('localhost'))) {
      // Convert to Google search
      return `https://www.google.com/search?q=${encodeURIComponent(trimmed)}`;
    }
    
    // Check if it looks like a domain
    if (trimmed.includes('.')) {
      return `https://${trimmed}`;
    }
    
    // If it's a single word, try as a .com domain first
    return `https://${trimmed}.com`;
  }

  return trimmed;
}

function getDomainFromUrl(url: string): string {
  try {
    const parsedUrl = new URL(url);
    return parsedUrl.hostname;
  } catch {
    return url;
  }
}

export function WebViewer({
  initialUrl = 'http://localhost:3000',
  onUrlChange,
  allowAllSites = true,
}: WebViewerProps) {
  // Use registry to get or create a stable window ID
  const childWindowId = useRef<string>('');
  const containerRef = useRef<HTMLDivElement>(null);
  const updateBoundsTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  
  // Calculate the actual URL consistently
  const storageKey = allowAllSites ? 'goose-webviewer-url' : 'goose-sidecar-url';
  const actualRegistryUrl = (typeof window !== 'undefined' && localStorage.getItem(storageKey)) || initialUrl;
  
  // Initialize window ID from registry on mount
  useEffect(() => {
    // Generate a window ID but don't register it yet - wait until window is actually created
    const windowId = `webviewer-window-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    childWindowId.current = windowId;
    console.log(`[WebViewer-${windowId}] Component MOUNTED - generated window ID for URL: ${actualRegistryUrl}`);
    
    // No cleanup function - we want child windows to persist across component lifecycles
  }, [actualRegistryUrl, allowAllSites]);
  
  // WebViewer context for AI prompt injection
  const webViewerContext = useWebViewerContextOptional();
  
  // Unified sidecar context for comprehensive AI context
  const unifiedSidecarContext = useUnifiedSidecarContextOptional();
  const [url, setUrl] = useState(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem(storageKey) || initialUrl;
    }
    return initialUrl;
  });

  const [inputUrl, setInputUrl] = useState(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem(storageKey) || initialUrl;
    }
    return initialUrl;
  });

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [canGoBack, setCanGoBack] = useState(false);
  const [canGoForward, setCanGoForward] = useState(false);
  const [isSecure, setIsSecure] = useState(false);
  const [actualUrl, setActualUrl] = useState(url);
  const [pageTitle, setPageTitle] = useState('');
  const [childWindowCreated, setChildWindowCreated] = useState(false);
  const [isVisible, setIsVisible] = useState(true);

  // Create or reuse child window on mount with timeout and retry logic
  useEffect(() => {
    const createOrReuseChildWindow = async (retryCount = 0) => {
      if (!containerRef.current || !childWindowId.current) {
        console.log(`[WebViewer-${childWindowId.current}] Skipping window creation - missing container or windowId`);
        return;
      }

      console.log(`[WebViewer-${childWindowId.current}] Starting createOrReuseChildWindow (attempt ${retryCount + 1})`);

      try {
        // Check if this component already has a window (for reuse case)
        const existingWindow = childWindowRegistry.hasWindow(childWindowId.current);
        
        console.log(`[WebViewer-${childWindowId.current}] Checking for existing window for component: ${childWindowId.current}, exists: ${existingWindow}`);
        
        if (existingWindow) {
          console.log(`[WebViewer-${childWindowId.current}] Found existing window in registry, attempting to reuse`);
          
          // Get the existing window ID from the registry
          const existingWindowId = childWindowRegistry.getWindowId(childWindowId.current);
          if (existingWindowId) {
            // Update our window ID to match the existing one
            childWindowId.current = existingWindowId;
            console.log(`[WebViewer-${childWindowId.current}] Updated window ID to match existing registry entry`);
            
            // Register ourselves as another user of this window
            childWindowRegistry.registerWindow(childWindowId.current, actualRegistryUrl, allowAllSites, existingWindowId);
            console.log(`[WebViewer-${childWindowId.current}] Registered as additional user of existing window`);
            
            setChildWindowCreated(true);
            
            // Update bounds for the existing window
            const rect = containerRef.current.getBoundingClientRect();
            const adjustedBounds = {
              x: Math.round(rect.x),
              y: Math.round(rect.y), 
              width: Math.round(Math.max(rect.width, 300)),
              height: Math.round(Math.max(rect.height, 200)),
            };
            
            // Show and update bounds of existing window
            await window.electron.showChildWebViewer(childWindowId.current);
            await window.electron.updateChildWebViewerBounds(childWindowId.current, adjustedBounds);
            return;
          } else {
            console.warn(`[WebViewer-${childWindowId.current}] Registry says window exists but couldn't get window ID - creating new window`);
          }
        }

        console.log(`[WebViewer-${childWindowId.current}] Creating new child window with URL:`, url, `(attempt ${retryCount + 1})`);
        
        const rect = containerRef.current.getBoundingClientRect();
        console.log(`[WebViewer-${childWindowId.current}] Container bounds:`, rect);
        
        // Child window expects coordinates relative to the main window
        const adjustedBounds = {
          x: Math.round(rect.x),
          y: Math.round(rect.y), 
          width: Math.round(Math.max(rect.width, 300)), // Minimum width for child window
          height: Math.round(Math.max(rect.height, 200)), // Minimum height for child window
        };
        
        console.log(`[WebViewer-${childWindowId.current}] Adjusted child window bounds:`, adjustedBounds);
        
        // Add timeout to child window creation
        const createWithTimeout = Promise.race([
          window.electron.createChildWebViewer(url, adjustedBounds, childWindowId.current),
          new Promise((_, reject) => 
            setTimeout(() => reject(new Error('Child window creation timeout')), 10000)
          )
        ]);
        
        const result = await createWithTimeout as any;
        
        console.log(`[WebViewer-${childWindowId.current}] Child window creation result:`, result);
        
        if (result.success && result.viewerId) {
          // Ensure the window ID matches what we expected
          if (result.viewerId !== childWindowId.current) {
            console.warn(`[WebViewer] Window ID mismatch: expected ${childWindowId.current}, got ${result.viewerId}`);
            childWindowId.current = result.viewerId;
          }
          
          // NOW register the window in the registry since it was successfully created
          childWindowRegistry.registerWindow(childWindowId.current, actualRegistryUrl, allowAllSites, childWindowId.current);
          console.log(`[WebViewer-${childWindowId.current}] Registered successfully created window in registry`);
          
          setChildWindowCreated(true);
          console.log(`[WebViewer-${childWindowId.current}] Child window created successfully:`, result.viewerId);
          
          // Show the child window if we're visible
          if (isVisible) {
            await window.electron.showChildWebViewer(childWindowId.current);
          }
        } else {
          throw new Error(result.error || 'Unknown error');
        }
      } catch (err) {
        console.error(`[WebViewer-${childWindowId.current}] Error creating child window:`, err);
        
        // Retry logic for transient failures
        if (retryCount < 2 && (err.message?.includes('timeout') || err.message?.includes('ECONNREFUSED'))) {
          console.log(`[WebViewer-${childWindowId.current}] Retrying child window creation in 1 second...`);
          setTimeout(() => createOrReuseChildWindow(retryCount + 1), 1000);
          return;
        }
        
        setError(`Failed to initialize child window: ${err.message || 'Unknown error'}`);
      }
    };

    // Add a small delay to ensure the container is properly rendered and window ID is set
    const timer = setTimeout(() => {
      if (containerRef.current && childWindowId.current) {
        createOrReuseChildWindow();
      }
    }, 100);

    return () => {
      clearTimeout(timer);
    };
  }, [url, isVisible, actualRegistryUrl, allowAllSites]);

  // Consolidated context registration and updates
  useEffect(() => {
    if (!childWindowCreated) return;

    console.log(`[WebViewer-${childWindowId.current}] Starting context registration process`);
    
    // Prepare common info
    const commonInfo = {
      id: childWindowId.current,
      url: actualUrl || url,
      title: pageTitle || 'Loading...',
      domain: getDomainFromUrl(actualUrl || url),
      isSecure: isSecure,
      isLocalhost: isLocalhostUrl(actualUrl || url),
    };

    // Register with WebViewer context if available
    if (webViewerContext) {
      const webViewerInfo = {
        ...commonInfo,
        lastUpdated: new Date(),
        type: (allowAllSites ? 'main' : 'sidecar') as 'sidecar' | 'main',
      };

      console.log(`[WebViewer-${childWindowId.current}] Registering with WebViewer context:`, webViewerInfo);
      webViewerContext.registerWebViewer(webViewerInfo);
    } else {
      console.warn(`[WebViewer-${childWindowId.current}] WebViewer context not available for registration`);
    }

    // Register with Unified Sidecar context if available
    if (unifiedSidecarContext) {
      const sidecarInfo = {
        ...commonInfo,
        type: 'web-viewer' as const,
        canGoBack: canGoBack,
        canGoForward: canGoForward,
        isLoading: isLoading,
        timestamp: Date.now(),
      };

      console.log(`[WebViewer-${childWindowId.current}] Registering with Unified Sidecar context:`, sidecarInfo);
      unifiedSidecarContext.registerSidecar(sidecarInfo);
    } else {
      console.warn(`[WebViewer-${childWindowId.current}] Unified Sidecar context not available for registration`);
    }

    // Cleanup function
    return () => {
      console.log(`[WebViewer-${childWindowId.current}] Unregistering from contexts`);
      
      if (webViewerContext) {
        webViewerContext.unregisterWebViewer(childWindowId.current);
        console.log(`[WebViewer-${childWindowId.current}] Unregistered from WebViewer context`);
      }
      
      if (unifiedSidecarContext) {
        unifiedSidecarContext.unregisterSidecar(childWindowId.current);
        console.log(`[WebViewer-${childWindowId.current}] Unregistered from Unified Sidecar context`);
      }
    };
  }, [webViewerContext, unifiedSidecarContext, childWindowCreated, allowAllSites]);

  // Consolidated context updates (debounced)
  useEffect(() => {
    if (!childWindowCreated) return;

    const updateTimeout = setTimeout(() => {
      console.log(`[WebViewer-${childWindowId.current}] Updating contexts with latest state`);
      
      // Common update info
      const commonUpdates = {
        url: actualUrl || url,
        title: pageTitle || 'Loading...',
        domain: getDomainFromUrl(actualUrl || url),
        isSecure: isSecure,
        isLocalhost: isLocalhostUrl(actualUrl || url),
      };

      // Update WebViewer context
      if (webViewerContext) {
        webViewerContext.updateWebViewer(childWindowId.current, commonUpdates);
        console.log(`[WebViewer-${childWindowId.current}] Updated WebViewer context`);
      }

      // Update Unified Sidecar context
      if (unifiedSidecarContext) {
        const sidecarUpdates = {
          ...commonUpdates,
          canGoBack: canGoBack,
          canGoForward: canGoForward,
          isLoading: isLoading,
          timestamp: Date.now(),
        };
        
        unifiedSidecarContext.updateSidecar(childWindowId.current, sidecarUpdates);
        console.log(`[WebViewer-${childWindowId.current}] Updated Unified Sidecar context`);
      }
    }, 100); // Debounce updates to prevent rapid re-renders

    return () => clearTimeout(updateTimeout);
  }, [
    webViewerContext, 
    unifiedSidecarContext, 
    childWindowCreated, 
    actualUrl, 
    url,
    pageTitle, 
    isSecure, 
    canGoBack, 
    canGoForward, 
    isLoading
  ]);

  // Handle cleanup on component unmount - ONLY unregister from contexts, NEVER destroy windows
  useEffect(() => {
    const cleanup = async () => {
      console.log(`[WebViewer-${childWindowId.current}] Component unmounting - unregistering from contexts only`);
      
      // Unregister from contexts first
      if (webViewerContext) {
        webViewerContext.unregisterWebViewer(childWindowId.current);
      }
      if (unifiedSidecarContext) {
        unifiedSidecarContext.unregisterSidecar(childWindowId.current);
      }
      
      // Unregister from the registry but NEVER destroy the window automatically
      childWindowRegistry.unregisterWindow(childWindowId.current);
      console.log(`[WebViewer-${childWindowId.current}] Unregistered from registry - window kept alive`);
    };

    return () => {
      cleanup();
    };
  }, [childWindowCreated, webViewerContext, unifiedSidecarContext, actualRegistryUrl, allowAllSites]);

  // Update child window bounds when container resizes or window moves (enhanced for drag operations)
  useEffect(() => {
    if (!childWindowCreated || !containerRef.current) return;

    let lastKnownBounds = { x: 0, y: 0, width: 0, height: 0 };
    let isUpdating = false;

    const immediateUpdateBounds = (force = false) => {
      if (isUpdating || !containerRef.current || !childWindowId.current) return;
      
      const rect = containerRef.current.getBoundingClientRect();
      
      // Use exact container bounds for child window positioning
      const adjustedBounds = {
        x: Math.round(rect.x),
        y: Math.round(rect.y),
        width: Math.round(Math.max(rect.width, 300)), // Minimum width
        height: Math.round(Math.max(rect.height, 200)), // Minimum height
      };
      
      // Only update if bounds actually changed (avoid unnecessary calls) unless forced
      if (!force && 
          adjustedBounds.x === lastKnownBounds.x && 
          adjustedBounds.y === lastKnownBounds.y &&
          adjustedBounds.width === lastKnownBounds.width &&
          adjustedBounds.height === lastKnownBounds.height) {
        return;
      }
      
      // Validate bounds are positive and reasonable
      if (adjustedBounds.width > 0 && adjustedBounds.height > 0 && 
          adjustedBounds.x >= 0 && adjustedBounds.y >= 0) {
        
        isUpdating = true;
        lastKnownBounds = { ...adjustedBounds };
        
        console.log(`[WebViewer-${childWindowId.current}] ${force ? 'FORCED' : 'Immediate'} bounds update:`, adjustedBounds);
        
        window.electron.updateChildWebViewerBounds(childWindowId.current, adjustedBounds)
          .catch(console.error)
          .finally(() => {
            isUpdating = false;
          });
      } else {
        console.warn(`[WebViewer-${childWindowId.current}] Invalid bounds calculated:`, adjustedBounds);
      }
    };

    // Force initial bounds sync - this helps with positioning issues on mount
    const forceInitialSync = () => {
      console.log(`[WebViewer-${childWindowId.current}] Forcing initial bounds synchronization`);
      immediateUpdateBounds(true);
      
      // Double-check after a short delay to handle any layout settling
      setTimeout(() => {
        console.log(`[WebViewer-${childWindowId.current}] Double-checking initial bounds after layout settle`);
        immediateUpdateBounds(true);
      }, 250);
    };

    const throttledUpdateBounds = () => {
      // Clear any pending update
      if (updateBoundsTimeoutRef.current) {
        clearTimeout(updateBoundsTimeoutRef.current);
      }
      
      // For drag operations, use immediate updates
      updateBoundsTimeoutRef.current = setTimeout(immediateUpdateBounds, 8); // ~120fps for smoother drag
    };

    // Force initial bounds synchronization
    forceInitialSync();

    // Set up resize observer for the container
    const resizeObserver = new ResizeObserver(() => {
      immediateUpdateBounds(); // Immediate for resize
    });
    resizeObserver.observe(containerRef.current);

    // Listen for window resize, scroll, and focus events
    window.addEventListener('resize', immediateUpdateBounds);
    window.addEventListener('scroll', throttledUpdateBounds);
    window.addEventListener('focus', throttledUpdateBounds);
    window.addEventListener('blur', throttledUpdateBounds);
    
    // Enhanced MutationObserver to catch all positioning changes
    const mutationObserver = new MutationObserver((mutations) => {
      let shouldUpdate = false;
      
      for (const mutation of mutations) {
        // Check for style changes that might affect positioning
        if (mutation.type === 'attributes') {
          const attrName = mutation.attributeName;
          if (attrName === 'style' || attrName === 'class' || attrName === 'data-x' || attrName === 'data-y') {
            shouldUpdate = true;
            break;
          }
        }
        // Check for DOM structure changes that might affect layout
        if (mutation.type === 'childList') {
          shouldUpdate = true;
          break;
        }
      }
      
      if (shouldUpdate) {
        immediateUpdateBounds(); // Immediate for DOM changes
      }
    });
    
    // Observe the container and all its ancestors for positioning changes
    mutationObserver.observe(containerRef.current, {
      attributes: true,
      attributeFilter: ['style', 'class', 'data-x', 'data-y', 'transform'],
      childList: true,
      subtree: false
    });
    
    // Walk up the DOM tree and observe all ancestors that might affect positioning
    let ancestorElement = containerRef.current.parentElement;
    let depth = 0;
    const maxDepth = 10; // Increased depth to catch more layout changes
    const observedElements = [containerRef.current];
    
    while (ancestorElement && depth < maxDepth) {
      // Observe this ancestor
      mutationObserver.observe(ancestorElement, {
        childList: true,
        attributes: true,
        attributeFilter: ['style', 'class', 'transform', 'data-x', 'data-y'],
        subtree: false
      });
      
      // Also observe with ResizeObserver for layout changes
      resizeObserver.observe(ancestorElement);
      observedElements.push(ancestorElement);
      
      // Stop if we reach the document body or a major layout boundary
      if (ancestorElement.tagName === 'BODY' || 
          ancestorElement.classList.contains('bento') || 
          ancestorElement.classList.contains('enhanced-bento') ||
          ancestorElement.getAttribute('data-container-type') ||
          ancestorElement.id === 'root') {
        break;
      }
      
      ancestorElement = ancestorElement.parentElement;
      depth++;
    }
    
    // Enhanced interval-based fallback with more frequent checks during potential drag operations
    let intervalFrequency = 50; // Start with 50ms checks
    let consecutiveNoChanges = 0;
    
    const boundsCheckInterval = setInterval(() => {
      if (!containerRef.current || !childWindowId.current) return;
      
      const rect = containerRef.current.getBoundingClientRect();
      const currentBounds = {
        x: Math.round(rect.x),
        y: Math.round(rect.y),
        width: Math.round(rect.width),
        height: Math.round(Math.max(rect.height, 200)),
      };
      
      // Check if position changed
      if (currentBounds.x !== lastKnownBounds.x || 
          currentBounds.y !== lastKnownBounds.y ||
          currentBounds.width !== lastKnownBounds.width ||
          currentBounds.height !== lastKnownBounds.height) {
        
        console.log(`[WebViewer-${childWindowId.current}] Interval detected position change:`, currentBounds);
        immediateUpdateBounds();
        consecutiveNoChanges = 0;
        intervalFrequency = 16; // Speed up during active changes (60fps)
      } else {
        consecutiveNoChanges++;
        // Slow down if no changes detected for a while
        if (consecutiveNoChanges > 20) {
          intervalFrequency = Math.min(200, intervalFrequency * 1.1); // Slow down to max 200ms
        }
      }
    }, intervalFrequency);

    // Listen for mouse events that might indicate drag operations
    const handleMouseMove = throttledUpdateBounds;
    const handleMouseUp = immediateUpdateBounds;
    
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    
    // Listen for touch events for mobile drag operations
    const handleTouchMove = throttledUpdateBounds;
    const handleTouchEnd = immediateUpdateBounds;
    
    document.addEventListener('touchmove', handleTouchMove);
    document.addEventListener('touchend', handleTouchEnd);

    return () => {
      // Clear any pending bounds update
      if (updateBoundsTimeoutRef.current) {
        clearTimeout(updateBoundsTimeoutRef.current);
        updateBoundsTimeoutRef.current = null;
      }
      
      // Clear the bounds check interval
      clearInterval(boundsCheckInterval);
      
      // Disconnect observers
      resizeObserver.disconnect();
      mutationObserver.disconnect();
      
      // Remove event listeners
      window.removeEventListener('resize', immediateUpdateBounds);
      window.removeEventListener('scroll', throttledUpdateBounds);
      window.removeEventListener('focus', throttledUpdateBounds);
      window.removeEventListener('blur', throttledUpdateBounds);
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      document.removeEventListener('touchmove', handleTouchMove);
      document.removeEventListener('touchend', handleTouchEnd);
    };
  }, [childWindowCreated]);

  // Navigate to initial URL when child window is ready
  useEffect(() => {
    if (childWindowCreated && url) {
      navigateToUrl(url);
    }
  }, [childWindowCreated]);

  // Update navigation state periodically
  useEffect(() => {
    if (!childWindowCreated) return;

    const updateNavigationState = async () => {
      try {
        const state = await window.electron.getChildWebViewerNavigationState(childWindowId.current);
        if (state) {
          setCanGoBack(state.canGoBack);
          setCanGoForward(state.canGoForward);
          setIsLoading(state.isLoading);
          
          if (state.url && state.url !== actualUrl) {
            setActualUrl(state.url);
            setInputUrl(state.url);
            setIsSecure(state.url.startsWith('https://'));
          }
          
          if (state.title) {
            setPageTitle(state.title);
          }
        }
      } catch (err) {
        console.error('Error getting navigation state:', err);
      }
    };

    // Update immediately and then periodically
    updateNavigationState();
    const interval = setInterval(updateNavigationState, 1000);

    return () => clearInterval(interval);
  }, [childWindowCreated, actualUrl]);

  // Handle visibility changes (show/hide child window) with bounds synchronization
  useEffect(() => {
    if (!childWindowCreated) return;

    console.log(`[WebViewer-${childWindowId.current}] Visibility changed to:`, isVisible);
    
    if (isVisible) {
      console.log(`[WebViewer-${childWindowId.current}] Showing child window`);
      
      // Force bounds synchronization when showing the window
      const syncBoundsAndShow = async () => {
        try {
          // First, ensure we have the correct bounds
          if (containerRef.current) {
            const rect = containerRef.current.getBoundingClientRect();
            const adjustedBounds = {
              x: Math.round(rect.x),
              y: Math.round(rect.y),
              width: Math.round(Math.max(rect.width, 300)),
              height: Math.round(Math.max(rect.height, 200)),
            };
            
            console.log(`[WebViewer-${childWindowId.current}] Syncing bounds before show:`, adjustedBounds);
            
            // Update bounds first, then show
            await window.electron.updateChildWebViewerBounds(childWindowId.current, adjustedBounds);
            await window.electron.showChildWebViewer(childWindowId.current);
            
            // Double-check bounds after showing (some window managers need this)
            setTimeout(async () => {
              if (containerRef.current) {
                const newRect = containerRef.current.getBoundingClientRect();
                const newBounds = {
                  x: Math.round(newRect.x),
                  y: Math.round(newRect.y),
                  width: Math.round(Math.max(newRect.width, 300)),
                  height: Math.round(Math.max(newRect.height, 200)),
                };
                
                console.log(`[WebViewer-${childWindowId.current}] Double-checking bounds after show:`, newBounds);
                await window.electron.updateChildWebViewerBounds(childWindowId.current, newBounds);
              }
            }, 100);
          } else {
            // Fallback if no container ref
            await window.electron.showChildWebViewer(childWindowId.current);
          }
        } catch (error) {
          console.error(`[WebViewer-${childWindowId.current}] Error syncing bounds and showing:`, error);
        }
      };
      
      syncBoundsAndShow();
    } else {
      console.log(`[WebViewer-${childWindowId.current}] Hiding child window`);
      window.electron.hideChildWebViewer(childWindowId.current);
    }
  }, [isVisible, childWindowCreated]);

  // Listen for child window events from main process
  useEffect(() => {
    const handleChildWindowLoading = (event: any, viewerId: string, loading: boolean) => {
      if (viewerId === childWindowId.current) {
        setIsLoading(loading);
      }
    };

    const handleChildWindowNavigation = (event: any, viewerId: string, navData: any) => {
      if (viewerId === childWindowId.current) {
        setActualUrl(navData.url);
        setInputUrl(navData.url);
        setPageTitle(navData.title);
        setCanGoBack(navData.canGoBack);
        setCanGoForward(navData.canGoForward);
        setIsSecure(navData.url.startsWith('https://'));
      }
    };

    const handleChildWindowError = (event: any, viewerId: string, errorMessage: string) => {
      if (viewerId === childWindowId.current) {
        setError(errorMessage);
        setIsLoading(false);
      }
    };

    const handleChildWindowTitle = (event: any, viewerId: string, title: string) => {
      if (viewerId === childWindowId.current) {
        setPageTitle(title);
      }
    };

    // Register event listeners
    window.electron.on('child-webviewer-loading', handleChildWindowLoading);
    window.electron.on('child-webviewer-navigation', handleChildWindowNavigation);
    window.electron.on('child-webviewer-error', handleChildWindowError);
    window.electron.on('child-webviewer-title', handleChildWindowTitle);

    return () => {
      // Cleanup event listeners
      window.electron.off('child-webviewer-loading', handleChildWindowLoading);
      window.electron.off('child-webviewer-navigation', handleChildWindowNavigation);
      window.electron.off('child-webviewer-error', handleChildWindowError);
      window.electron.off('child-webviewer-title', handleChildWindowTitle);
    };
  }, []);

  const navigateToUrl = async (targetUrl: string) => {
    if (!childWindowCreated) return;

    setIsLoading(true);
    setError(null);

    try {
      const success = await window.electron.childWebViewerNavigate(childWindowId.current, targetUrl);
      if (!success) {
        setError('Failed to navigate to URL');
        setIsLoading(false);
      }
      // Loading state will be cleared by navigation state updates
    } catch (err) {
      console.error('Navigation error:', err);
      setError('Navigation failed');
      setIsLoading(false);
    }
  };

  useEffect(() => {
    if (onUrlChange) {
      onUrlChange(actualUrl);
    }
  }, [actualUrl, onUrlChange]);

  const handleUrlSubmit = (newUrl: string) => {
    const formattedUrl = formatUrl(newUrl, allowAllSites);

    if (!isValidUrl(formattedUrl)) {
      setError('Please enter a valid URL');
      return;
    }

    if (!allowAllSites && !isLocalhostUrl(formattedUrl)) {
      setError('Only localhost URLs are allowed (e.g., http://localhost:3000)');
      return;
    }

    setError(null);
    setUrl(formattedUrl);
    setInputUrl(formattedUrl);

    // Save to localStorage
    if (typeof window !== 'undefined') {
      localStorage.setItem(storageKey, formattedUrl);
    }

    navigateToUrl(formattedUrl);
  };

  const handleKeyPress = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      handleUrlSubmit(inputUrl);
    }
  };

  const handleRefresh = async () => {
    if (!childWindowCreated) return;
    
    try {
      await window.electron.childWebViewerRefresh(childWindowId.current);
    } catch (err) {
      console.error('Refresh error:', err);
    }
  };

  const handleOpenInBrowser = () => {
    window.electron.openExternal(actualUrl).catch(console.error);
  };

  const handleGoBack = async () => {
    if (!childWindowCreated || !canGoBack) return;
    
    try {
      await window.electron.childWebViewerGoBack(childWindowId.current);
    } catch (err) {
      console.error('Go back error:', err);
    }
  };

  const handleGoForward = async () => {
    if (!childWindowCreated || !canGoForward) return;
    
    try {
      await window.electron.childWebViewerGoForward(childWindowId.current);
    } catch (err) {
      console.error('Go forward error:', err);
    }
  };

  const handleHome = () => {
    const homeUrl = allowAllSites ? 'https://google.com' : 'http://localhost:3000';
    handleUrlSubmit(homeUrl);
  };

  const domain = getDomainFromUrl(actualUrl);
  const isLocalhost = isLocalhostUrl(actualUrl);

  return (
    <div className="h-full flex flex-col bg-background-default rounded-lg">
      {/* URL Bar and Controls */}
      <div className="flex items-center gap-2 p-3 bg-background-muted rounded-t-lg">
        {/* Navigation buttons */}
        <div className="flex items-center gap-1">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleGoBack}
                disabled={!canGoBack}
                className="p-1 h-8 w-8"
              >
                <ChevronLeft size={14} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Go Back</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleGoForward}
                disabled={!canGoForward}
                className="p-1 h-8 w-8"
              >
                <ChevronRight size={14} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Go Forward</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="sm" onClick={handleRefresh} className="p-1 h-8 w-8">
                <RefreshCw size={14} className={isLoading ? 'animate-spin' : ''} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Refresh</TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="sm" onClick={handleHome} className="p-1 h-8 w-8">
                <Home size={14} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{allowAllSites ? 'Go to Google' : 'Go to localhost:3000'}</TooltipContent>
          </Tooltip>
        </div>

        {/* URL Input with security indicator */}
        <div className="flex-1 flex items-center">
          <div className="flex-1 relative flex items-center">
            {/* Security/Type indicator */}
            <div className="absolute left-2 z-10 flex items-center">
              {isLocalhost ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <div className="flex items-center">
                      <Globe size={12} className="text-blue-500" />
                    </div>
                  </TooltipTrigger>
                  <TooltipContent>Local development server</TooltipContent>
                </Tooltip>
              ) : isSecure ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <div className="flex items-center">
                      <Shield size={12} className="text-green-500" />
                    </div>
                  </TooltipTrigger>
                  <TooltipContent>Secure connection (HTTPS)</TooltipContent>
                </Tooltip>
              ) : (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <div className="flex items-center">
                      <ShieldOff size={12} className="text-orange-500" />
                    </div>
                  </TooltipTrigger>
                  <TooltipContent>Insecure connection (HTTP)</TooltipContent>
                </Tooltip>
              )}
            </div>
            
            <input
              type="text"
              value={inputUrl}
              onChange={(e) => setInputUrl(e.target.value)}
              onKeyPress={handleKeyPress}
              placeholder={allowAllSites ? "Enter URL or search term" : "http://localhost:3000"}
              className="w-full pl-8 pr-3 py-1 text-sm border border-border-subtle rounded-md bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-border-prominent focus:border-transparent"
            />
          </div>
          
          <Button
            variant="ghost"
            size="sm"
            onClick={() => handleUrlSubmit(inputUrl)}
            className="ml-2 px-3 py-1 text-xs"
          >
            Go
          </Button>
        </div>

        {/* External link button */}
        <div className="flex items-center gap-2">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="sm" onClick={handleOpenInBrowser} className="p-1 h-8 w-8">
                <ExternalLink size={14} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Open in Browser</TooltipContent>
          </Tooltip>
        </div>
      </div>

      {/* Error Display */}
      {error && (
        <div className="p-3 bg-red-50 dark:bg-red-900/20 border-b border-red-200 dark:border-red-800">
          <p className="text-red-800 dark:text-red-200 text-sm">{error}</p>
        </div>
      )}

      {/* Child Window Container */}
      <div 
        ref={containerRef}
        className="flex-1 relative overflow-hidden bg-background-default rounded-b-lg"
        style={{ 
          isolation: 'isolate', // Create new stacking context
          zIndex: 1 // Ensure it's below UI controls
        }}
      >
        {!childWindowCreated && (
          <div className="absolute inset-0 bg-background-default flex items-center justify-center">
            <div className="text-center">
              <RefreshCw className="h-6 w-6 animate-spin mx-auto mb-2 text-primary" />
              <p className="text-text-subtle text-sm">Initializing child window...</p>
            </div>
          </div>
        )}
        
        {childWindowCreated && isLoading && (
          <div className="absolute top-0 left-0 right-0 h-1 bg-blue-200">
            <div className="h-full bg-blue-500 animate-pulse"></div>
          </div>
        )}
        
        {pageTitle && (
          <div className="absolute top-2 left-2 bg-black/70 text-white text-xs px-2 py-1 rounded opacity-0 hover:opacity-100 transition-opacity pointer-events-none">
            {pageTitle}
          </div>
        )}

        {/* Placeholder content - the actual web content is rendered in the child window */}
        {childWindowCreated && (
          <div className="absolute inset-0 bg-transparent flex items-center justify-center pointer-events-none">
            <div className="text-center text-text-subtle text-sm opacity-50">
              <p>Web content is displayed in a separate window</p>
              <p className="text-xs mt-1">Child window ID: {childWindowId.current}</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// Set up periodic cleanup for the registry
setInterval(() => {
  childWindowRegistry.cleanup();
}, 60000); // Cleanup every minute

// Global cleanup handler for when the parent window closes
if (typeof window !== 'undefined') {
  let globalCleanupRegistered = false;
  
  const registerGlobalCleanup = () => {
    if (globalCleanupRegistered) return;
    globalCleanupRegistered = true;
    
    console.log('[ChildWindowRegistry] Registering global cleanup handlers');
    
    const forceCleanupAll = () => {
      console.log('[ChildWindowRegistry] Global cleanup triggered - parent window closing');
      childWindowRegistry.forceCleanupAll();
    };
    
    // Listen for window close events
    window.addEventListener('beforeunload', forceCleanupAll);
    window.addEventListener('unload', forceCleanupAll);
    
    // Also listen for page visibility changes as a backup
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'hidden') {
        // Small delay to see if it's just a tab switch or actual close
        setTimeout(() => {
          if (document.visibilityState === 'hidden') {
            console.log('[ChildWindowRegistry] Page hidden for extended time, triggering cleanup');
            forceCleanupAll();
          }
        }, 1000);
      }
    });
  };
  
  // Register cleanup when the first WebViewer is loaded
  registerGlobalCleanup();
}

export default WebViewer;
