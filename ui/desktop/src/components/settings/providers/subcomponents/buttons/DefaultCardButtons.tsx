import { ConfigureSettingsButton, RocketButton } from './CardButtons';
import { ProviderDetails } from '../../../../../api';
import { useTranslation } from 'react-i18next';

// can define other optional callbacks as needed
interface CardButtonsProps {
  provider: ProviderDetails;
  isOnboardingPage: boolean;
  onConfigure: (provider: ProviderDetails) => void;
  onLaunch: (provider: ProviderDetails) => void;
}

export default function DefaultCardButtons({
  provider,
  isOnboardingPage,
  onLaunch,
  onConfigure,
}: CardButtonsProps) {
  const { t } = useTranslation();
  return (
    <>
      {/*Set up an unconfigured provider */}
      {!provider.is_configured && (
        <ConfigureSettingsButton
          tooltip={t('providerCards.configureSettingsTooltip', {
            provider: provider.metadata.display_name,
          })}
          onClick={(e) => {
            e.stopPropagation();
            onConfigure(provider);
          }}
        />
      )}
      {/*show edit tooltip instead when hovering over button for configured providers*/}
      {provider.is_configured && !isOnboardingPage && (
        <ConfigureSettingsButton
          tooltip={t('providerCards.editSettingsTooltip', {
            provider: provider.metadata.display_name,
          })}
          onClick={(e) => {
            e.stopPropagation();
            onConfigure(provider);
          }}
        />
      )}
      {/*show Launch button for configured providers on onboarding page*/}
      {provider.is_configured && isOnboardingPage && (
        <RocketButton
          data-testid={`provider-launch-button-${provider.name.toLowerCase()}`}
          tooltip={t('providerCards.getStartedTooltip')}
          onClick={(e) => {
            e.stopPropagation();
            onLaunch(provider);
          }}
        />
      )}
    </>
  );
}
