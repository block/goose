import React from 'react';
import { Card } from './ui/card';
import { Bird } from './ui/icons';
import { ChevronDown } from './icons';

interface ApiKeyWarningProps {
  className?: string;
}

interface CollapsibleProps {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
}

function Collapsible({ title, children, defaultOpen = false }: CollapsibleProps) {
  const [isOpen, setIsOpen] = React.useState(defaultOpen);

  return (
    <div className="border rounded-lg mb-2">
      <button
        className="w-full px-4 py-2 text-left flex justify-between items-center hover:bg-gray-50"
        onClick={() => setIsOpen(!isOpen)}
      >
        <span className="font-medium">{title}</span>
        <ChevronDown
          className={`w-5 h-5 transition-transform ${isOpen ? 'transform rotate-180' : ''}`}
        />
      </button>
      {isOpen && <div className="px-4 py-2 border-t">{children}</div>}
    </div>
  );
}

const OPENAI_CONFIG = `export GOOSE_PROVIDER__TYPE=openai
export GOOSE_PROVIDER__HOST=https://api.openai.com
export GOOSE_PROVIDER__MODEL=gpt-4o
export GOOSE_PROVIDER__API_KEY=your_api_key_here`;

const ANTHROPIC_CONFIG = `export GOOSE_PROVIDER__TYPE=anthropic
export GOOSE_PROVIDER__HOST=https://api.anthropic.com
export GOOSE_PROVIDER__MODEL=claude-3-5-sonnet-latest
export GOOSE_PROVIDER__API_KEY=your_api_key_here`;

const DATABRICKS_CONFIG = `export GOOSE_PROVIDER__TYPE=databricks
export GOOSE_PROVIDER__HOST=your_databricks_host
export GOOSE_PROVIDER__MODEL=your_databricks_model`;

const OPENROUTER_CONFIG = `export GOOSE_PROVIDER__TYPE=openrouter
export GOOSE_PROVIDER__HOST=https://openrouter.ai
export GOOSE_PROVIDER__MODEL=anthropic/claude-3.5-sonnet
export GOOSE_PROVIDER__API_KEY=your_api_key_here`;

export function ApiKeyWarning({ className }: ApiKeyWarningProps) {
  return (
    <Card
      className={`flex flex-col items-center p-8 space-y-6 bg-card-gradient w-full h-full ${className}`}
    >
      <div className="w-16 h-16">
        <Bird />
      </div>
      <div className="text-center space-y-4 max-w-2xl w-full">
        <h2 className="text-2xl font-semibold text-gray-800">Credentials Required</h2>
        <p className="text-gray-600 mb-4">
          To use Goose, you need to set environment variables for one of the following providers:
        </p>

        <div className="text-left">
          <Collapsible title="OpenAI Configuration" defaultOpen={true}>
            <pre className="bg-gray-50 p-4 rounded-md text-sm">{OPENAI_CONFIG}</pre>
          </Collapsible>

          <Collapsible title="Anthropic (Claude) Configuration">
            <pre className="bg-gray-50 p-4 rounded-md text-sm">{ANTHROPIC_CONFIG}</pre>
          </Collapsible>

          <Collapsible title="Databricks Configuration">
            <pre className="bg-gray-50 p-4 rounded-md text-sm">{DATABRICKS_CONFIG}</pre>
          </Collapsible>

          <Collapsible title="OpenRouter Configuration">
            <pre className="bg-gray-50 p-4 rounded-md text-sm">{OPENROUTER_CONFIG}</pre>
          </Collapsible>
        </div>
        <p className="text-gray-600 mt-4">
          After setting these variables, restart Goose for the changes to take effect.
        </p>
      </div>
    </Card>
  );
}
