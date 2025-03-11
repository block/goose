import React, { memo, useMemo } from 'react';
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

export const ProviderCard = memo(function ProviderCard({
  provider,
  onConfigure,
  onLaunch,
  isOnboarding,
}: ProviderCardProps) {
  // Safely access metadata with null checks
  const providerMetadata: ProviderMetadata | null = provider?.metadata || null;

  // Instead of useEffect for logging, use useMemo to memoize the metadata
  const metadata = useMemo(() => providerMetadata, [provider]);

  // Remove the logging completely

  if (!metadata) {
    return <div>ProviderCard error: No metadata provided</div>;
  }

  return (
    <CardContainer
      header={
        <CardHeader
          name={metadata.display_name || provider?.name || 'Unknown Provider'}
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
});
