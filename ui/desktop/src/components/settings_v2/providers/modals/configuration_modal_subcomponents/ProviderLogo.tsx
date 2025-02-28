import React from 'react';
// Direct import at the top of your file
import OpenAILogo from './icons/openai.png';

export default function ProviderLogo({ providerName }) {
  return (
    <div className="flex justify-center mb-2">
      {/* Smaller circle */}
      <div className="w-12 h-12 bg-black rounded-full flex items-center justify-center">
        {/* Larger image */}
        <img
          src={OpenAILogo}
          alt={`${providerName} logo`}
          className="w-10 h-10" // Increased from w-8 h-8
        />
      </div>
    </div>
  );
}
