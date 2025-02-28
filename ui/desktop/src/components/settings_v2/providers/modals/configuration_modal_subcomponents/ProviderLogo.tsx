import React from 'react';

export default function ProviderLogo(providerName) {
  // TODO: Determine which logo to display
  const logoPath = '../icons/openai.svg'; // /icons/openai.svg
  //const fallbackLogoPath = './icons/default.svg';
  return (
    <div className="flex justify-center mb-2">
      <div className="w-16 h-16 bg-black rounded-full flex items-center justify-center">
        <img src={logoPath} alt={`${providerName} logo`} className="w-8 h-8" />
      </div>
    </div>
  );
}
