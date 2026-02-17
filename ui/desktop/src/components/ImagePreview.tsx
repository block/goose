import { useState } from 'react';

interface ImagePreviewProps {
  src: string;
}

export default function ImagePreview({ src }: ImagePreviewProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [error, setError] = useState(false);

  if (error) {
    return <div className="text-red-500 text-xs italic mt-1 mb-1">Unable to load image</div>;
  }

  return (
    <div className={`image-preview mt-2 mb-2`}>
      <img
        src={src}
        alt="goose image"
        onError={() => setError(true)}
        onClick={() => setIsExpanded(!isExpanded)}
        className={`rounded border border-border-default cursor-pointer hover:border-border-default transition-all ${
          isExpanded ? 'max-w-full max-h-96' : 'max-h-40 max-w-40'
        }`}
        style={{ objectFit: 'contain' }}
      />
      <div className="text-xs text-text-muted mt-1">
        Click to {isExpanded ? 'collapse' : 'expand'}
      </div>
    </div>
  );
}
