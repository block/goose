import React from 'react';
import CardContainer from './CardContainer';
import CardHeader from './CardHeader';
import ProviderState from '../interfaces/ProviderState';
import CardBody from './CardBody';
import ButtonCallbacks from '../interfaces/ButtonCallbacks';
import { PROVIDER_REGISTRY } from '../ProviderRegistry';

interface ProviderCardProps {
  provider: ProviderState;
  buttonCallbacks: ButtonCallbacks;
  isOnboarding: boolean;
}

export function ProviderCard({ provider, buttonCallbacks, isOnboarding }: ProviderCardProps) {
  const providerEntry = PROVIDER_REGISTRY.find((p) => p.name === provider.name);

  // Add safety check
  if (!providerEntry) {
    console.error(`Provider ${provider.name} not found in registry`);
    return null;
  }

  const providerDetails = providerEntry.details;
  // Add another safety check
  if (!providerDetails) {
    console.error(`Provider ${provider.name} has no details`);
    return null;
  }
  console.log('provider details', providerDetails);

  try {
    const actions = providerDetails.getActions(provider, buttonCallbacks, isOnboarding);

    return (
      <CardContainer
        header={
          <CardHeader
            name={providerDetails.name}
            description={providerDetails.description}
            isConfigured={provider.isConfigured}
          />
        }
        body={<CardBody actions={actions} />}
      />
    );
  } catch (error) {
    console.error(`Error rendering provider card for ${provider.name}:`, error);
    return null;
  }
}
