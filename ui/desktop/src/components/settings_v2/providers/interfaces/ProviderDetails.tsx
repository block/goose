import React from 'react';
import ParameterSchema from '../interfaces/ParameterSchema';
import ProviderSetupFormProps from '../modal/interfaces/ProviderSetupFormProps';

export default interface ProviderDetails {
  id: string;
  name: string;
  description: string;
  parameters: ParameterSchema[];
  getTags?: (name: string) => string[];
  customForm?: React.ComponentType<ProviderSetupFormProps>;
  customSubmit?: (e: React.SyntheticEvent) => void;
}
