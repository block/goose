import React from 'react';

interface ProviderSetupHeaderProps {
  headerText: string;
}

/**
 * Renders the header (title) for the modal.
 */
export default function ProviderSetupHeader({ headerText }: ProviderSetupHeaderProps) {
  return (
    <div className="flex">
      <h2 className="text-2xl font-regular text-textStandard">{headerText}</h2>
    </div>
  );
}
