import React from 'react';

interface ImagePreviewProps {
  imageData?: string | null;
  onRemove: () => void;
}

export default function ImagePreview({ imageData, onRemove }: ImagePreviewProps) {
  if (!imageData) return null;
  
  return (
    <div className="relative inline-block">
      <div className="w-[120px] h-[120px] rounded-2xl overflow-hidden bg-gray-100 dark:bg-gray-700 ring-1 ring-black/5 dark:ring-white/5">
        <div className="w-full h-full relative">
          <img
            src={imageData}
            alt="Preview"
            className="absolute inset-0 w-full h-full object-cover"
            style={{
              imageRendering: 'auto',
              transform: 'translate3d(0,0,0)',
              backfaceVisibility: 'hidden',
              WebkitFontSmoothing: 'antialiased',
            }}
          />
          <button
            onClick={onRemove}
            className="absolute top-2 right-2 w-5 h-5 rounded-full flex items-center justify-center"
            style={{
              backgroundColor: 'rgba(143, 142, 147, 0.9)',
            }}
          >
            <svg
              className="w-3 h-3 text-white"
              fill="none"
              stroke="currentColor"
              strokeWidth={3}
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}
