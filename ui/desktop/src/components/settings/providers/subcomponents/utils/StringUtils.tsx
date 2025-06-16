// Functions for string / string-based element creation (e.g. tooltips for each provider, descriptions, etc)
export function OllamaNotConfiguredTooltipMessage() {
  return (
    <p>
      To use, either the{' '}
      <a
        href="https://ollama.com/download"
        target="_blank"
        rel="noopener noreferrer"
        className="text-blue-600 underline hover:text-blue-800"
      >
        Ollama app
      </a>{' '}
      must be installed on your machine and open, or you must enter a value for OLLAMA_HOST.
    </p>
  );
}

export function ConfiguredProviderTooltipMessage(name: string) {
  return `${name} provider is configured`;
}

interface ProviderDescriptionProps {
  description: string;
}

export function ProviderDescription({ description }: ProviderDescriptionProps) {
  return (
    <p className="text-xs text-textSubtle mt-1.5 mb-3 leading-normal overflow-y-auto max-h-[54px]">
      {description}
    </p>
  );
}
