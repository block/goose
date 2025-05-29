import { useState, useEffect } from 'react';

interface ImagePreviewProps {
  src: string;
  alt?: string;
  className?: string;
}

export default function ImagePreview({
  src,
  alt = 'Pasted image',
  className = '',
}: ImagePreviewProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [error, setError] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [imageData, setImageData] = useState<string | null>(null);

  useEffect(() => {
    const loadImage = async () => {
      try {
        // Use the IPC handler to get the image data
        const data = await window.electron.getTempImage(src);
        if (data) {
          setImageData(data);
          setIsLoading(false);
        } else {
          setError(true);
          setIsLoading(false);
        }
      } catch (err) {
        console.error('Error loading image:', err);
        setError(true);
        setIsLoading(false);
      }
    };

    loadImage();
  }, [src]);

  const handleError = () => {
    setError(true);
    setIsLoading(false);
  };

  const toggleExpand = () => {
    if (!error) {
      setIsExpanded(!isExpanded);
    }
  };

  // Validate that this is a safe file path (should contain goose-pasted-images)
  if (!src.includes('goose-pasted-images')) {
    return <div className="text-red-500 text-xs italic mt-1 mb-1">Invalid image path: {src}</div>;
  }

  if (error) {
    return <div className="text-red-500 text-xs italic mt-1 mb-1">Unable to load image: {src}</div>;
  }

  return (
    <div className={`image-preview mt-2 mb-2 ${className}`}>
      {isLoading && (
        <div className="animate-pulse bg-gray-200 rounded w-40 h-40 flex items-center justify-center">
          <span className="text-gray-500 text-xs">Loading...</span>
        </div>
      )}
      {imageData && (
        <img
          src={imageData}
          alt={alt}
          onError={handleError}
          onClick={toggleExpand}
          className={`rounded border border-borderSubtle cursor-pointer hover:border-borderStandard transition-all ${
            isExpanded ? 'max-w-full max-h-96' : 'max-h-40 max-w-40'
          } ${isLoading ? 'hidden' : ''}`}
          style={{ objectFit: 'contain' }}
        />
      )}
      {isExpanded && !error && !isLoading && imageData && (
        <div className="text-xs text-textSubtle mt-1">Click to collapse</div>
      )}
      {!isExpanded && !error && !isLoading && imageData && (
        <div className="text-xs text-textSubtle mt-1">Click to expand</div>
      )}
    </div>
  );
}
