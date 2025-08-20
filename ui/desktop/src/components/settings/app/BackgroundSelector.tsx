import React, { useState, useEffect, useCallback } from 'react';
import { Button } from '../../ui/button';
import { Card } from '../../ui/card';
import { Upload, X, RotateCcw } from 'lucide-react';
import { Switch } from '../../ui/switch';

interface BackgroundOption {
  id: string;
  name: string;
  type: 'gradient' | 'image' | 'solid';
  value: string;
  preview?: string;
}

const DEFAULT_BACKGROUNDS: BackgroundOption[] = [
  {
    id: 'default-gradient',
    name: 'Default Gradient',
    type: 'gradient',
    value: `
      radial-gradient(circle at 20% 80%, rgba(100, 100, 110, 0.25) 0%, transparent 50%),
      radial-gradient(circle at 80% 20%, rgba(120, 120, 125, 0.22) 0%, transparent 50%),
      radial-gradient(circle at 40% 40%, rgba(90, 95, 100, 0.18) 0%, transparent 50%),
      linear-gradient(135deg, 
        rgba(0, 0, 0, 0.02) 0%, 
        rgba(0, 0, 0, 0.04) 25%, 
        rgba(0, 0, 0, 0.02) 50%, 
        rgba(0, 0, 0, 0.06) 75%, 
        rgba(0, 0, 0, 0.03) 100%
      )
    `,
  },
  {
    id: 'blue-gradient',
    name: 'Ocean Blue',
    type: 'gradient',
    value: `
      radial-gradient(circle at 20% 80%, rgba(59, 130, 246, 0.15) 0%, transparent 50%),
      radial-gradient(circle at 80% 20%, rgba(99, 102, 241, 0.12) 0%, transparent 50%),
      radial-gradient(circle at 40% 40%, rgba(147, 197, 253, 0.08) 0%, transparent 50%),
      linear-gradient(135deg, 
        rgba(59, 130, 246, 0.02) 0%, 
        rgba(99, 102, 241, 0.04) 25%, 
        rgba(147, 197, 253, 0.02) 50%, 
        rgba(59, 130, 246, 0.06) 75%, 
        rgba(99, 102, 241, 0.03) 100%
      )
    `,
  },
  {
    id: 'purple-gradient',
    name: 'Purple Haze',
    type: 'gradient',
    value: `
      radial-gradient(circle at 20% 80%, rgba(147, 51, 234, 0.15) 0%, transparent 50%),
      radial-gradient(circle at 80% 20%, rgba(168, 85, 247, 0.12) 0%, transparent 50%),
      radial-gradient(circle at 40% 40%, rgba(196, 181, 253, 0.08) 0%, transparent 50%),
      linear-gradient(135deg, 
        rgba(147, 51, 234, 0.02) 0%, 
        rgba(168, 85, 247, 0.04) 25%, 
        rgba(196, 181, 253, 0.02) 50%, 
        rgba(147, 51, 234, 0.06) 75%, 
        rgba(168, 85, 247, 0.03) 100%
      )
    `,
  },
  {
    id: 'green-gradient',
    name: 'Forest Green',
    type: 'gradient',
    value: `
      radial-gradient(circle at 20% 80%, rgba(34, 197, 94, 0.15) 0%, transparent 50%),
      radial-gradient(circle at 80% 20%, rgba(22, 163, 74, 0.12) 0%, transparent 50%),
      radial-gradient(circle at 40% 40%, rgba(134, 239, 172, 0.08) 0%, transparent 50%),
      linear-gradient(135deg, 
        rgba(34, 197, 94, 0.02) 0%, 
        rgba(22, 163, 74, 0.04) 25%, 
        rgba(134, 239, 172, 0.02) 50%, 
        rgba(34, 197, 94, 0.06) 75%, 
        rgba(22, 163, 74, 0.03) 100%
      )
    `,
  },
  {
    id: 'solid-dark',
    name: 'Dark',
    type: 'solid',
    value: 'rgba(0, 0, 0, 0.1)',
  },
  {
    id: 'solid-light',
    name: 'Light',
    type: 'solid',
    value: 'rgba(255, 255, 255, 0.05)',
  },
];

const STORAGE_KEY = 'dashboard-background';
const CUSTOM_IMAGE_KEY = 'dashboard-custom-image';
const DOT_OVERLAY_KEY = 'dashboard-dot-overlay';
const MAX_IMAGE_SIZE_MB = 5; // 5MB limit for images

export default function BackgroundSelector() {
  const [selectedBackground, setSelectedBackground] = useState<string>('default-gradient');
  const [customImage, setCustomImage] = useState<string | null>(null);
  const [showDotOverlay, setShowDotOverlay] = useState(true);
  const [isDragging, setIsDragging] = useState(false);
  const [isUpdating, setIsUpdating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Safe event dispatch function
  const safeDispatchEvent = useCallback((eventName: string, detail: any) => {
    try {
      // Use requestAnimationFrame to ensure UI updates before dispatching event
      requestAnimationFrame(() => {
        window.dispatchEvent(new CustomEvent(eventName, { detail }));
      });
      return true;
    } catch (error) {
      console.error(`Error dispatching ${eventName} event:`, error);
      setError(`Failed to update background. Please try again.`);
      return false;
    }
  }, []);

  // Load saved settings with error handling
  useEffect(() => {
    try {
      const savedBackground = localStorage.getItem(STORAGE_KEY);
      const savedCustomImage = localStorage.getItem(CUSTOM_IMAGE_KEY);
      const savedDotOverlay = localStorage.getItem(DOT_OVERLAY_KEY);

      if (savedBackground) {
        setSelectedBackground(savedBackground);
      }
      if (savedCustomImage) {
        setCustomImage(savedCustomImage);
      }
      if (savedDotOverlay !== null) {
        setShowDotOverlay(savedDotOverlay === 'true');
      }
    } catch (error) {
      console.error('Error loading background settings:', error);
      // Continue with defaults if loading fails
    }
  }, []);

  // Save settings and trigger update with improved async handling
  const saveAndApplyBackground = useCallback((backgroundId: string, imageData?: string | null) => {
    // Clear any previous errors
    setError(null);
    
    // Set updating state to show loading indicator
    setIsUpdating(true);

    // Use requestAnimationFrame to ensure UI updates before potentially intensive operations
    requestAnimationFrame(() => {
      setTimeout(() => {
        try {
          // Update state first
          setSelectedBackground(backgroundId);
          
          // Then update storage
          localStorage.setItem(STORAGE_KEY, backgroundId);

          if (imageData !== undefined) {
            if (imageData) {
              localStorage.setItem(CUSTOM_IMAGE_KEY, imageData);
            } else {
              localStorage.removeItem(CUSTOM_IMAGE_KEY);
            }
            setCustomImage(imageData);
          }

          // Dispatch event after state and storage are updated
          safeDispatchEvent('dashboard-background-changed', {
            backgroundId, 
            customImage: imageData !== undefined ? imageData : customImage
          });
        } catch (error) {
          console.error('Error saving background settings:', error);
          setError('Failed to save background settings. Please try again.');
        } finally {
          // Clear updating state after a short delay
          setTimeout(() => {
            setIsUpdating(false);
          }, 300);
        }
      }, 0);
    });
  }, [customImage, safeDispatchEvent]);

  const handleDotOverlayToggle = useCallback((enabled: boolean) => {
    setIsUpdating(true);
    setError(null);
    
    requestAnimationFrame(() => {
      setTimeout(() => {
        try {
          setShowDotOverlay(enabled);
          localStorage.setItem(DOT_OVERLAY_KEY, String(enabled));
          
          safeDispatchEvent('dashboard-dot-overlay-changed', { enabled });
        } catch (error) {
          console.error('Error toggling dot overlay:', error);
          setError('Failed to update overlay settings. Please try again.');
        } finally {
          setTimeout(() => {
            setIsUpdating(false);
          }, 300);
        }
      }, 0);
    });
  }, [safeDispatchEvent]);

  // Validate and process image with size limits
  const handleFileUpload = useCallback((file: File) => {
    if (!file.type.startsWith('image/')) {
      setError('Please select a valid image file');
      return;
    }

    // Check file size (limit to 5MB)
    if (file.size > MAX_IMAGE_SIZE_MB * 1024 * 1024) {
      setError(`Image size exceeds ${MAX_IMAGE_SIZE_MB}MB limit. Please choose a smaller file.`);
      return;
    }

    setIsUpdating(true);
    setError(null);

    // Process image in a non-blocking way
    const reader = new FileReader();
    
    reader.onload = (e) => {
      try {
        const imageData = e.target?.result as string;
        
        // Validate image data
        if (!imageData || !imageData.startsWith('data:image/')) {
          throw new Error('Invalid image data');
        }
        
        // Create an image to verify it loads correctly
        const img = new Image();
        img.onload = () => {
          // Image loaded successfully, now safe to use
          saveAndApplyBackground('custom-image', imageData);
        };
        img.onerror = () => {
          setError('Failed to process image. The file may be corrupted.');
          setIsUpdating(false);
        };
        img.src = imageData;
      } catch (error) {
        console.error('Error processing image:', error);
        setError('Failed to process image. Please try another file.');
        setIsUpdating(false);
      }
    };
    
    reader.onerror = () => {
      console.error('Error reading file');
      setError('Failed to read image file. Please try again.');
      setIsUpdating(false);
    };
    
    reader.readAsDataURL(file);
  }, [saveAndApplyBackground]);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);

    const files = Array.from(e.dataTransfer.files);
    const imageFile = files.find(file => file.type.startsWith('image/'));
    
    if (imageFile) {
      handleFileUpload(imageFile);
    } else if (files.length > 0) {
      setError('Please drop an image file (JPG, PNG, GIF, etc.)');
    }
  }, [handleFileUpload]);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
  }, []);

  const handleFileInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      handleFileUpload(file);
    }
  }, [handleFileUpload]);

  const removeCustomImage = useCallback(() => {
    saveAndApplyBackground('default-gradient', null);
  }, [saveAndApplyBackground]);

  const resetToDefault = useCallback(() => {
    setIsUpdating(true);
    setError(null);
    
    requestAnimationFrame(() => {
      setTimeout(() => {
        try {
          // Update background
          saveAndApplyBackground('default-gradient', null);
          
          // Update overlay
          setShowDotOverlay(true);
          localStorage.setItem(DOT_OVERLAY_KEY, 'true');
          
          safeDispatchEvent('dashboard-dot-overlay-changed', { enabled: true });
        } catch (error) {
          console.error('Error resetting to default:', error);
          setError('Failed to reset settings. Please try again.');
        } finally {
          setTimeout(() => {
            setIsUpdating(false);
          }, 300);
        }
      }, 0);
    });
  }, [saveAndApplyBackground, safeDispatchEvent]);

  // Loading overlay that blocks interaction during updates
  const LoadingOverlay = () => (
    isUpdating ? (
      <div className="fixed inset-0 bg-background-default/50 backdrop-blur-sm flex items-center justify-center z-50">
        <div className="bg-background-default p-4 rounded-lg shadow-lg flex flex-col items-center">
          <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textStandard mb-2"></div>
          <p className="text-sm text-textStandard">Updating background...</p>
        </div>
      </div>
    ) : null
  );

  // Error message display
  const ErrorMessage = () => (
    error ? (
      <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md p-3 mb-4">
        <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
      </div>
    ) : null
  );

  return (
    <div className="space-y-6">
      {/* Loading overlay */}
      <LoadingOverlay />
      
      {/* Error message */}
      <ErrorMessage />
      
      {/* Dot Overlay Toggle */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium text-textStandard">Dot Pattern Overlay</h3>
          <p className="text-xs text-textSubtle mt-1">
            Show subtle dot pattern over the background
          </p>
        </div>
        <Switch
          checked={showDotOverlay}
          onCheckedChange={handleDotOverlayToggle}
          variant="mono"
          disabled={isUpdating}
        />
      </div>

      {/* Preset Backgrounds */}
      <div>
        <h3 className="text-sm font-medium text-textStandard mb-3">Preset Backgrounds</h3>
        <div className="grid grid-cols-2 gap-3">
          {DEFAULT_BACKGROUNDS.map((bg) => (
            <Card
              key={bg.id}
              className={`relative cursor-pointer transition-all duration-200 overflow-hidden ${
                selectedBackground === bg.id
                  ? 'ring-2 ring-blue-500 shadow-lg'
                  : 'hover:shadow-md'
              } ${isUpdating ? 'pointer-events-none opacity-50' : ''}`}
              onClick={() => !isUpdating && saveAndApplyBackground(bg.id)}
            >
              <div className="aspect-video relative">
                <div
                  className="absolute inset-0"
                  style={{
                    background: bg.type === 'gradient' ? bg.value : bg.value,
                    backgroundSize: bg.type === 'gradient' ? '400% 400%' : 'auto',
                  }}
                />
                {/* Dot overlay preview */}
                {showDotOverlay && (
                  <div
                    className="absolute inset-0 opacity-20"
                    style={{
                      backgroundImage: `radial-gradient(circle, rgba(0, 0, 0, 0.4) 1px, transparent 1px)`,
                      backgroundSize: '12px 12px',
                      backgroundPosition: '6px 6px',
                    }}
                  />
                )}
                <div className="absolute inset-0 flex items-center justify-center">
                  <span className="text-xs font-medium text-textStandard bg-background-default/80 px-2 py-1 rounded">
                    {bg.name}
                  </span>
                </div>
              </div>
            </Card>
          ))}
        </div>
      </div>

      {/* Custom Image Upload */}
      <div>
        <h3 className="text-sm font-medium text-textStandard mb-3">Custom Image</h3>
        
        {customImage ? (
          <div className="space-y-3">
            <Card
              className={`relative cursor-pointer transition-all duration-200 overflow-hidden ${
                selectedBackground === 'custom-image'
                  ? 'ring-2 ring-blue-500 shadow-lg'
                  : 'hover:shadow-md'
              } ${isUpdating ? 'pointer-events-none opacity-50' : ''}`}
              onClick={() => !isUpdating && saveAndApplyBackground('custom-image')}
            >
              <div className="aspect-video relative">
                <img
                  src={customImage}
                  alt="Custom background"
                  className="absolute inset-0 w-full h-full object-cover opacity-30"
                />
                {/* Dot overlay preview */}
                {showDotOverlay && (
                  <div
                    className="absolute inset-0 opacity-20"
                    style={{
                      backgroundImage: `radial-gradient(circle, rgba(0, 0, 0, 0.4) 1px, transparent 1px)`,
                      backgroundSize: '12px 12px',
                      backgroundPosition: '6px 6px',
                    }}
                  />
                )}
                <div className="absolute inset-0 flex items-center justify-center">
                  <span className="text-xs font-medium text-textStandard bg-background-default/80 px-2 py-1 rounded">
                    Custom Image
                  </span>
                </div>
              </div>
            </Card>
            
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={removeCustomImage}
                className="flex items-center gap-2"
                disabled={isUpdating}
              >
                <X size={14} />
                Remove
              </Button>
              <label className={isUpdating ? 'pointer-events-none opacity-50' : ''}>
                <Button
                  variant="outline"
                  size="sm"
                  className="flex items-center gap-2 cursor-pointer"
                  asChild
                  disabled={isUpdating}
                >
                  <span>
                    <Upload size={14} />
                    Replace
                  </span>
                </Button>
                <input
                  type="file"
                  accept="image/*"
                  onChange={handleFileInputChange}
                  className="hidden"
                  disabled={isUpdating}
                />
              </label>
            </div>
          </div>
        ) : (
          <div
            className={`border-2 border-dashed rounded-lg p-8 text-center transition-colors ${
              isDragging
                ? 'border-blue-500 bg-blue-50 dark:bg-blue-950/20'
                : 'border-borderSubtle hover:border-borderStandard'
            } ${isUpdating ? 'pointer-events-none opacity-50' : ''}`}
            onDrop={handleDrop}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
          >
            <Upload className="mx-auto h-8 w-8 text-textSubtle mb-3" />
            <p className="text-sm text-textStandard mb-2">
              Drop an image here or click to upload
            </p>
            <p className="text-xs text-textSubtle mb-4">
              Supports JPG, PNG, GIF, WebP (max {MAX_IMAGE_SIZE_MB}MB)
            </p>
            <label className={isUpdating ? 'pointer-events-none opacity-50' : ''}>
              <Button 
                variant="outline" 
                size="sm" 
                className="cursor-pointer" 
                asChild
                disabled={isUpdating}
              >
                <span>Choose File</span>
              </Button>
              <input
                type="file"
                accept="image/*"
                onChange={handleFileInputChange}
                className="hidden"
                disabled={isUpdating}
              />
            </label>
          </div>
        )}
      </div>

      {/* Reset Button */}
      <div className="pt-4 border-t border-borderSubtle">
        <Button
          variant="outline"
          size="sm"
          onClick={resetToDefault}
          className="flex items-center gap-2"
          disabled={isUpdating}
        >
          <RotateCcw size={14} />
          Reset to Default
        </Button>
      </div>
    </div>
  );
}
