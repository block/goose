import React from 'react';

interface ImagePreviewProps {
  imageData: string;
  onRemove: () => void;
}

export default function ImagePreview({ imageData, onRemove }: ImagePreviewProps) {
  return (
    <div className="relative inline-block">
      <div className="w-[120px] h-[120px] rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-700 ring-1 ring-black/5 dark:ring-white/5">
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
        </div>
      </div>
      <button
        onClick={onRemove}
        className="absolute -top-1 -right-1 w-5 h-5 bg-gray-800 dark:bg-gray-600 hover:bg-gray-700 dark:hover:bg-gray-500 rounded-full flex items-center justify-center"
      >
        <svg
          className="w-3 h-3 text-white"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M6 18L18 6M6 6l12 12"
          />
        </svg>
      </button>
    </div>
  );
}
