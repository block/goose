import React from 'react';
import { Card } from './ui/card';
import { Bird } from './ui/icons';

interface ApiKeyWarningProps {
  className?: string;
}

export function ApiKeyWarning({ className }: ApiKeyWarningProps) {
  return (
    <Card className={`flex flex-col items-center justify-center p-8 space-y-6 bg-card-gradient w-full h-full ${className}`}>
      <div className="w-16 h-16">
        <Bird />
      </div>
      <div className="text-center space-y-4">
        <h2 className="text-2xl font-semibold text-gray-800">API Key Required</h2>
        <div className="whitespace-pre-wrap">
          To use Goose, you need to set some combination of the following env variables
          <br />
          <br />
          # OpenAI
          <br />
          <br />
          export GOOSE_PROVIDER__TYPE=openai<br />
          GOOSE_PROVIDER__HOST=https://api.openai.com<br />
          GOOSE_PROVIDER__MODEL=gpt-4o<br />
          GOOSE_PROVIDER__API_KEY=...<br />
          <br />
          <br />
          # Databricks + Claude
          <br />
          <br />
          export GOOSE_PROVIDER__TYPE=databricks<br />
          export GOOSE_PROVIDER__HOST=...<br />
          export GOOSE_PROVIDER__MODEL="claude-3-5-sonnet-2"<br />
          <br />
          <br />
          Please export these and restart the application.
        </div>
      </div>
    </Card>
  );
}