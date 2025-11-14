import React from 'react';
import { matrixService } from '../services/MatrixService';

interface AvatarImageProps {
  avatarUrl?: string;
  displayName?: string;
  className?: string;
  size?: 'sm' | 'md' | 'lg';
  onError?: () => void;
}

const AvatarImage: React.FC<AvatarImageProps> = ({ 
  avatarUrl, 
  displayName, 
  className = '', 
  size = 'md',
  onError 
}) => {
  const [blobUrl, setBlobUrl] = React.useState<string | null>(null);
  const [showInitials, setShowInitials] = React.useState(false);
  const [isLoading, setIsLoading] = React.useState(false);
  const blobUrlRef = React.useRef<string | null>(null);

  // Size classes
  const sizeClasses = {
    sm: 'w-6 h-6 text-xs',
    md: 'w-8 h-8 text-sm',
    lg: 'w-12 h-12 text-base'
  };

  // Fetch authenticated blob when avatarUrl changes
  React.useEffect(() => {
    if (!avatarUrl || !avatarUrl.startsWith('mxc://')) {
      // If it's not an MXC URL, use it directly
      setBlobUrl(avatarUrl || null);
      setShowInitials(false);
      return;
    }

    setIsLoading(true);
    setShowInitials(false);
    
    // Clean up previous blob URL
    if (blobUrlRef.current && blobUrlRef.current.startsWith('blob:')) {
      URL.revokeObjectURL(blobUrlRef.current);
      blobUrlRef.current = null;
    }
    setBlobUrl(null);

    // Fetch authenticated blob
    matrixService.getAuthenticatedMediaBlob(avatarUrl)
      .then((url) => {
        if (url) {
          blobUrlRef.current = url;
          setBlobUrl(url);
        } else {
          setShowInitials(true);
          onError?.();
        }
      })
      .catch((error) => {
        console.error('AvatarImage - error getting authenticated blob:', error);
        setShowInitials(true);
        onError?.();
      })
      .finally(() => {
        setIsLoading(false);
      });

    // Cleanup function
    return () => {
      if (blobUrlRef.current && blobUrlRef.current.startsWith('blob:')) {
        URL.revokeObjectURL(blobUrlRef.current);
        blobUrlRef.current = null;
      }
    };
  }, [avatarUrl, onError]);

  // Cleanup on unmount
  React.useEffect(() => {
    return () => {
      if (blobUrlRef.current && blobUrlRef.current.startsWith('blob:')) {
        URL.revokeObjectURL(blobUrlRef.current);
        blobUrlRef.current = null;
      }
    };
  }, []);

  const handleImageError = () => {
    setShowInitials(true);
    onError?.();
  };

  const handleImageLoad = () => {
    setShowInitials(false);
  };

  if (!avatarUrl || showInitials || isLoading || !blobUrl) {
    const displayText = isLoading ? '...' : (displayName || 'U').charAt(0).toUpperCase();
    return (
      <div className={`${sizeClasses[size]} bg-background-accent rounded-full flex items-center justify-center ${className}`}>
        <span className="font-medium text-text-on-accent">
          {displayText}
        </span>
      </div>
    );
  }

  return (
    <img
      src={blobUrl}
      alt={displayName}
      className={`${sizeClasses[size]} rounded-full object-cover ${className}`}
      onLoad={handleImageLoad}
      onError={handleImageError}
    />
  );
};

export default AvatarImage;
