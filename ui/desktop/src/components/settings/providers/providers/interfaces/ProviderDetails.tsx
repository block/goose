// metadata and action builder
import ProviderState from './ProviderState';
import ConfigurationAction from './ConfigurationAction';
import ParameterSchema from '../parameters/interfaces/ParameterSchema';
import ProviderCallbacks from './ConfigurationCallbacks';

export default interface ProviderDetails {
  id: string;
  name: string;
  description: string;
  parameters: ParameterSchema[];
  getTags?: (name: string) => string[];
  getActions?: (provider: ProviderState, callbacks: ProviderCallbacks) => ConfigurationAction[];
}
