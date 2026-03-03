import { useMemo } from 'react';
import CardContainer from './CardContainer';
import CardHeader from './CardHeader';
import CardBody from './CardBody';
import DefaultCardButtons from './buttons/DefaultCardButtons';
import { ProviderDetails, ProviderMetadata } from '../../../../api';
import { useTranslation } from 'react-i18next';

type ProviderCardProps = {
  provider: ProviderDetails;
  onConfigure: () => void;
  onLaunch: () => void;
  isOnboarding: boolean;
};

export const ProviderCard = function ProviderCard({
  provider,
  onConfigure,
  onLaunch,
  isOnboarding,
}: ProviderCardProps) {
  const { t } = useTranslation();
  // Safely access metadata with null checks
  const providerMetadata: ProviderMetadata | null = provider?.metadata || null;

  // Instead of useEffect for logging, use useMemo to memoize the metadata
  const metadata = useMemo(() => providerMetadata, [providerMetadata]);

  if (!metadata) {
    return <div>{t('providerCards.noMetadataError')}</div>;
  }

  const handleCardClick = () => {
    if (!isOnboarding) {
      onConfigure();
    }
  };

  return (
    <CardContainer
      testId={`provider-card-${provider.name.toLowerCase()}`}
      grayedOut={!provider.is_configured && isOnboarding} // onboarding page will have grayed out cards if not configured
      onClick={handleCardClick}
      header={
        <CardHeader
          name={metadata.display_name || provider?.name || t('providerCards.unknownProvider')}
          description={metadata.description || ''}
          isConfigured={provider?.is_configured || false}
        />
      }
      body={
        <CardBody>
          <DefaultCardButtons
            provider={provider}
            onConfigure={onConfigure}
            onLaunch={onLaunch}
            isOnboardingPage={isOnboarding}
          />
        </CardBody>
      }
    />
  );
};
