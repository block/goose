import React from 'react';
import Admonition from '@theme/Admonition';

interface ArchivedExtensionWarningProps {
  extensionName?: string;
}

export default function ArchivedExtensionWarning({ extensionName }: ArchivedExtensionWarningProps) {
  const message = extensionName 
    ? `The ${extensionName} is no longer actively maintained. The repository remains available for reference, but may not be compatible with current versions of goose.`
    : 'This extension is no longer actively maintained. The repository remains available for reference, but may not be compatible with current versions of goose.';

  return (
    <Admonition type="warning" title="Archived Extension">
      {message}
    </Admonition>
  );
}
