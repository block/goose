import React from 'react';
import { ConfigureSettingsButton, RocketButton } from './CardButtons';
import ProviderState from '@/src/components/settings_v2/providers/interfaces/ProviderState';

// can define other optional callbacks as needed
interface CardButtonsProps {
  provider: ProviderState;
  isOnboardingPage: boolean;
  onConfigure: (provider: ProviderState) => void;
  onLaunch: (provider: ProviderState) => void;
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

export default function DefaultCardButtons({
  provider,
  isOnboardingPage,
  onLaunch,
  onConfigure,
}: CardButtonsProps) {
  return (
    <>
      {/*Set up an unconfigured provider */}
      {!provider.isConfigured && (
        <ConfigureSettingsButton
          tooltip={getDefaultTooltipMessages(provider.name, 'add')}
          onClick={(e) => {
            e.stopPropagation();
            onConfigure(provider);
          }}
        />
      )}
      {/*show edit tooltip instead when hovering over button for configured providers*/}
      {provider.isConfigured && !isOnboardingPage && (
        <ConfigureSettingsButton
          tooltip={getDefaultTooltipMessages(provider.name, 'edit')}
          onClick={(e) => {
            e.stopPropagation();
            onConfigure(provider);
          }}
        />
      )}
      {/*show Launch button for configured providers on onboarding page*/}
      {provider.isConfigured && isOnboardingPage && (
        <RocketButton
          onClick={(e) => {
            e.stopPropagation();
            onLaunch(provider);
          }}
        />
      )}
    </>
  );
}
