import React from 'react';
import { PanelLeft } from 'lucide-react';

interface EnvVar {
  name: string;
  label: string;
}

interface GooseDesktopInstallerProps {
  extensionId: string;
  extensionName: string;
  description: string;
  command: string;
  args: string[];
  envVars?: EnvVar[];
  apiKeyLink?: string;
  apiKeyLinkText?: string;
  customStep3?: string;
}

export default function GooseDesktopInstaller({
  extensionId,
  extensionName,
  description,
  command,
  args,
  envVars = [],
  apiKeyLink,
  apiKeyLinkText,
  customStep3
}: GooseDesktopInstallerProps) {
  
  // Build the goose:// URL
  const buildGooseUrl = () => {
    const urlParts = [
      `cmd=${encodeURIComponent(command)}`,
      ...args.map(arg => `arg=${encodeURIComponent(arg)}`),
      `id=${encodeURIComponent(extensionId)}`,
      `name=${encodeURIComponent(extensionName)}`,
      `description=${encodeURIComponent(description)}`,
      // Add environment variables (matching TLDR sections encoding)
      ...envVars.map(envVar => 
        `env=${encodeURIComponent(`${envVar.name}=${envVar.label}`)}`
      )
    ];
    
    return `goose://extension?${urlParts.join('&')}`;
  };

  // Generate step 3 content
  const getStep3Content = () => {
    if (customStep3) {
      return customStep3;
    }
    
    if (apiKeyLink && apiKeyLinkText) {
      return (
        <>
          Get your <a href={apiKeyLink}>{apiKeyLinkText}</a> and paste it in
        </>
      );
    }
    
    if (envVars.length > 0) {
      const envVarNames = envVars.map(env => env.name).join(', ');
      return `Obtain your ${envVarNames} and paste it in`;
    }
    
    return 'Configure any required settings';
  };

  return (
    <div>
      <ol>
        <li>
          <a href={buildGooseUrl()}>Launch the installer</a>
        </li>
        <li>Click <code>OK</code> to confirm the installation</li>
        <li>{getStep3Content()}</li>
        <li>Click <code>Add Extension</code></li>
        <li>Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar</li>
        <li>Navigate to the chat</li>
      </ol>
    </div>
  );
}
