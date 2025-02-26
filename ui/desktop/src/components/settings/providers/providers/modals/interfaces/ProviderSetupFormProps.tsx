import React from 'react';
import ProviderDetails from '../../interfaces/ProviderDetails';

export default interface ProviderSetupFormProps {
  configValues: { [key: string]: string };
  setConfigValues: React.Dispatch<React.SetStateAction<{ [key: string]: string }>>;
  onSubmit: (e: React.FormEvent) => void;
  provider: ProviderDetails;
}
