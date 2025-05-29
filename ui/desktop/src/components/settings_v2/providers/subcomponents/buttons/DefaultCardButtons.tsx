import { ConfigureSettingsButton, RocketButton } from './CardButtons';
import { ProviderDetails } from '../../../../../api';

// can define other optional callbacks as needed
interface CardButtonsProps {
  provider: ProviderDetails;
  isOnboardingPage: boolean;
  onConfigure: (provider: ProviderDetails) => void;
  onLaunch: (provider: ProviderDetails) => void;
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
      {!provider.is_configured && (
        <ConfigureSettingsButton
          tooltip={getDefaultTooltipMessages(provider.metadata.display_name, 'add')}
          onClick={(e) => {
            e.stopPropagation();
            onConfigure(provider);
          }}
        />
      )}
      {/*show edit tooltip instead when hovering over button for configured providers*/}
      {provider.is_configured && !isOnboardingPage && (
        <ConfigureSettingsButton
          tooltip={getDefaultTooltipMessages(provider.metadata.display_name, 'edit')}
          onClick={(e) => {
            e.stopPropagation();
            onConfigure(provider);
          }}
        />
      )}
      {/*show Launch button for configured providers on onboarding page*/}
      {provider.is_configured && isOnboardingPage && (
        <RocketButton
          tooltip={'Get started with goose!'}
          onClick={(e) => {
            e.stopPropagation();
            onLaunch(provider);
          }}
        />
      )}
    </>
  );
}
