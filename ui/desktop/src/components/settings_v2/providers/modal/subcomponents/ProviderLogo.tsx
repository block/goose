import React from 'react';
import OpenAILogo from './icons/openai@3x.png';
import AnthropicLogo from './icons/anthropic@3x.png';
import GoogleLogo from './icons/google@3x.png';
import GroqLogo from './icons/groq@3x.png';
import OllamaLogo from './icons/ollama@3x.png';
import DatabricksLogo from './icons/databricks@3x.png';
import OpenRouterLogo from './icons/openrouter@3x.png';

// Map provider names to their logos
const providerLogos = {
  openai: OpenAILogo,
  anthropic: AnthropicLogo,
  google: GoogleLogo,
  groq: GroqLogo,
  ollama: OllamaLogo,
  databricks: DatabricksLogo,
  openrouter: OpenRouterLogo,
};

export default function ProviderLogo({ providerName }) {
  // Convert provider name to lowercase and fetch the logo
  const logoKey = providerName.toLowerCase();
  const logo = providerLogos[logoKey] || OpenAILogo; // TODO: need default icon

  return (
    <div className="flex justify-center mb-2">
      <div className="w-12 h-12 bg-black rounded-full overflow-hidden flex items-center justify-center">
        <img src={logo} alt={`${providerName} logo`} className="w-16 h-16 object-contain" />
      </div>
    </div>
  );
}
