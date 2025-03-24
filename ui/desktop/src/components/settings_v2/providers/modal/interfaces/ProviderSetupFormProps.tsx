import React from 'react';
import ProviderState from '../../interfaces/ProviderState';

export default interface ProviderSetupFormProps {
  configValues: { [key: string]: string };
  setConfigValues: React.Dispatch<React.SetStateAction<{ [key: string]: string }>>;
  onSubmit: (e: React.FormEvent) => void;
  provider: ProviderState;
}
