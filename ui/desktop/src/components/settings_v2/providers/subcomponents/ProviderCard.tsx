import React, { useEffect } from 'react';
import CardContainer from './CardContainer';
import CardHeader from './CardHeader';
import CardBody from './CardBody';
import DefaultCardButtons from './buttons/DefaultCardButtons';
import { ProviderDetails, ProviderMetadata } from '../../../../api';

type ProviderCardProps = {
  provider: ProviderDetails;
  onConfigure: () => void;
  onLaunch: () => void;
  isOnboarding: boolean;
};

export const ProviderCard = React.memo(function ProviderCard({
  provider,
  onConfigure,
  onLaunch,
  isOnboarding,
}: ProviderCardProps) {
  // Safely access metadata with null checks
  const providerMetadata: ProviderMetadata = provider?.metadata || {};

  // Use useEffect for logging to avoid console spam
  useEffect(() => {
    console.log('Provider:', provider);
    console.log('Provider Metadata:', providerMetadata);
  }, [provider]);

  return (
    <CardContainer
      header={
        <CardHeader
          name={providerMetadata.display_name || provider?.name || 'Unknown Provider'}
          description={providerMetadata.description || ''}
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
});
