import React from 'react';
import { AddButton, DeleteButton, ConfigureSettingsButton, RocketButton } from './CardButtons';

interface ProviderActionsProps {
  name: string;
  isConfigured: boolean;
  isOnboardingPage?: boolean;
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
  isOnboardingPage,
  onAdd,
  onDelete,
  onShowSettings,
}: ProviderActionsProps) {
  return (
    <>
      {/*Set up an unconfigured provider */}
      {!isConfigured && (
        <ConfigureSettingsButton
          tooltip={getDefaultTooltipMessages(name, 'add')}
          onClick={(e) => {
            e.stopPropagation();
            onAdd?.();
          }}
        />
      )}
      {/*show edit tooltip instead when hovering over button for configured providers*/}
      {isConfigured && !isOnboardingPage && (
        <ConfigureSettingsButton
          tooltip={getDefaultTooltipMessages(name, 'edit')}
          onClick={(e) => {
            e.stopPropagation();
            onShowSettings?.();
          }}
        />
      )}
      {/*show Launch button for configured providers on onboarding page*/}
      {isConfigured && isOnboardingPage && (
        <RocketButton
          onClick={(e) => {
            e.stopPropagation();
            onShowSettings?.();
          }}
        />
      )}
    </>
  );
}
