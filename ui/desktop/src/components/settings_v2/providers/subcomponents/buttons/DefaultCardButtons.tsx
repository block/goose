import React from 'react';
import { ConfigureSettingsButton, RocketButton } from './CardButtons';
import ButtonCallbacks from '../../interfaces/ButtonCallbacks';
import ProviderState from '@/src/components/settings_v2/providers/interfaces/ProviderState';

// can define other optional callbacks as needed
interface CardButtonsProps {
  provider: ProviderState;
  isOnboardingPage?: boolean;
  callbacks: ButtonCallbacks; // things like onConfigure, onDelete
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

/// This defines a group of buttons that will appear on the card
/// Controlled by if a provider is configured and which version of the grid page we're on (onboarding vs settings page)
/// This is the default button group
///
/// Settings page:
///   - show configure button
/// Onboarding page:
///   - show configure button if NOT configured
///   - show rocket launch button if configured
///
/// We inject what will happen if we click on a button via on<Function>
///  - onConfigure: pop open a modal -- modal is configured dynamically
///  - onLaunch: continue to chat window
export default function DefaultCardButtons({
  provider,
  isOnboardingPage,
  callbacks,
}: CardButtonsProps) {
  return (
    <>
      {/*Set up an unconfigured provider */}
      {!provider.isConfigured && (
        <ConfigureSettingsButton
          tooltip={getDefaultTooltipMessages(provider.name, 'add')}
          onClick={(e) => {
            e.stopPropagation();
            callbacks.onConfigure(provider);
          }}
        />
      )}
      {/*show edit tooltip instead when hovering over button for configured providers*/}
      {provider.isConfigured && !isOnboardingPage && (
        <ConfigureSettingsButton
          tooltip={getDefaultTooltipMessages(provider.name, 'edit')}
          onClick={(e) => {
            e.stopPropagation();
            callbacks.onConfigure(provider);
          }}
        />
      )}
      {/*show Launch button for configured providers on onboarding page*/}
      {provider.isConfigured && isOnboardingPage && (
        <RocketButton
          onClick={(e) => {
            e.stopPropagation();
            callbacks.onLaunch(provider);
          }}
        />
      )}
    </>
  );
}
