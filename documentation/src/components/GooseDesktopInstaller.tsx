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
  // Command-line extension props (optional when using url)
  command?: string;
  args?: string[];
  // SSE extension prop (optional when using command+args)
  url?: string;
  envVars?: EnvVar[];
  apiKeyLink?: string;
  apiKeyLinkText?: string;
  customStep3?: string;
  hasEnvVars?: boolean; // Explicit control over configuration steps
  appendToStep3?: string;
}

export default function GooseDesktopInstaller({
  extensionId,
  extensionName,
  description,
  command,
  args,
  url,
  envVars = [],
  apiKeyLink,
  apiKeyLinkText,
  customStep3,
  hasEnvVars,
  appendToStep3
}: GooseDesktopInstallerProps) {
  
  // Build the goose:// URL
  const buildGooseUrl = () => {
    let urlParts = [];
    
    // Add SSE extension URL or command-line extension command+args first
    if (url) {
      urlParts.push(`url=${encodeURIComponent(url)}`);
    } else if (command && args) {
      urlParts.push(`cmd=${encodeURIComponent(command)}`);
      urlParts.push(...args.map(arg => `arg=${encodeURIComponent(arg)}`));
    }
    
    // Add common parameters
    urlParts.push(
      `id=${encodeURIComponent(extensionId)}`,
      `name=${encodeURIComponent(extensionName)}`,
      `description=${encodeURIComponent(description)}`
    );
    
    // Add environment variables (matching TLDR sections encoding)
    urlParts.push(...envVars.map(envVar => 
      `env=${encodeURIComponent(`${envVar.name}=${envVar.label}`)}`
    ));
    
    return `goose://extension?${urlParts.join('&')}`;
  };

  // Generate step 3 content (only if needed)
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
    
    return null; // No configuration needed
  };

  const content = getStep3Content();
  const step3Content = appendToStep3
    ? (
        <>
          {content}
          {content ? <br /> : null}
          {appendToStep3}
        </>
      )
    : content;
  
  const hasConfigurationContent = step3Content !== null;
  const shouldShowConfigurationSteps = hasEnvVars ?? hasConfigurationContent;

  return (
    <div>
      <ol>
        <li>
          <a href={buildGooseUrl()}>Launch the installer</a>
        </li>
        <li>Click <code>OK</code> to confirm the installation</li>
        {shouldShowConfigurationSteps && (
          <>
            <li>{step3Content}</li>
            <li>Click <code>Add Extension</code></li>
          </>
    )}
        <li>Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar</li>
        <li>Navigate to the chat</li>
      </ol>
    </div>
  );
}
