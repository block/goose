// metadata and action builder
import ProviderState from './ProviderState';
import ConfigurationAction from './ConfigurationAction';
import ParameterSchema from '../interfaces/ParameterSchema';
import ButtonCallbacks from './ButtonCallbacks';

export default interface ProviderDetails {
  id: string;
  name: string;
  description: string;
  parameters: ParameterSchema[];
  getTags?: (name: string) => string[];
  getActions?: (
    provider: ProviderState,
    callbacks: ButtonCallbacks,
    isOnboardingPage: boolean
  ) => ConfigurationAction[];
}
