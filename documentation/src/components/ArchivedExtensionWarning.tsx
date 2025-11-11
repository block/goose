import React from 'react';
import Admonition from '@theme/Admonition';

interface ArchivedExtensionWarningProps {
  extensionName?: string;
  repoUrl?: string;
}

export default function ArchivedExtensionWarning({ extensionName, repoUrl }: ArchivedExtensionWarningProps) {
  const prefix = extensionName ? `The ${extensionName} is` : 'This extension is';
  
  return (
    <Admonition type="warning" title="Archived Extension">
      {prefix} no longer actively maintained. The{' '}
      {repoUrl ? (
        <a href={repoUrl} target="_blank" rel="noopener noreferrer">
          repository
        </a>
      ) : (
        'repository'
      )}{' '}
      remains available for reference, but may not be compatible with current versions of goose.
    </Admonition>
  );
}
