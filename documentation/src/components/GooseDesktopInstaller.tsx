import React from 'react';

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
    const params = new URLSearchParams();
    params.set('cmd', command);
    
    // Add all args
    args.forEach(arg => {
      params.append('arg', arg);
    });
    
    params.set('id', extensionId);
    params.set('name', extensionName);
    params.set('description', description);
    
    // Add environment variables
    envVars.forEach(envVar => {
      params.set('env', `${envVar.name}=${envVar.label}`);
    });
    
    return `goose://extension?${params.toString()}`;
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
        <li>Press <code>Yes</code> to confirm the installation</li>
        <li>{getStep3Content()}</li>
        <li>Click <code>Save Configuration</code></li>
        <li>Scroll to the top and click <code>Exit</code> from the upper left corner</li>
      </ol>
    </div>
  );
}
