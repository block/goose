import React from 'react';
import { AddButton, DeleteButton, GearSettingsButton } from './ActionButtons';

interface ProviderActionsProps {
  name: string;
  isConfigured: boolean;
  onAdd?: () => void;
  onConfigure?: () => void;
  onDelete?: () => void;
  onShowSettings?: () => void;
}

function getDefaultTooltipMessages(name: string, actionType: string) {
  switch (actionType) {
    case 'add':
      return `Configure ${name} settings`;
    case 'edit':
      return `Edit ${name} settings`;
    case 'delete':
      return `Delete ${name} settings`;
    default:
      return null;
  }
}

export default function DefaultProviderActions({
  name,
  isConfigured,
  onAdd,
  onDelete,
  onShowSettings,
}: ProviderActionsProps) {
  return (
    <>
      {/*Set up an unconfigured provider */}
      {!isConfigured && (
        <AddButton
          tooltip={getDefaultTooltipMessages(name, 'add')}
          onClick={(e) => {
            e.stopPropagation();
            onAdd?.();
          }}
        />
      )}
      {/*Edit settings of configured provider*/}
      {isConfigured && (
        <GearSettingsButton
          tooltip={getDefaultTooltipMessages(name, 'edit')}
          onClick={(e) => {
            e.stopPropagation();
            onShowSettings?.();
          }}
        />
      )}
      {/*Delete configuration*/}
      {isConfigured && (
        <DeleteButton
          tooltip={getDefaultTooltipMessages(name, 'delete')}
          onClick={(e) => {
            e.stopPropagation();
            onDelete?.();
          }}
        />
      )}
    </>
  );
}
