// used both for initial config and editing config
import ProviderDetails from '../../interfaces/ProviderDetails';

export default interface ProviderConfiguationModalProps {
  provider: ProviderDetails;
  title?: string;
  onSubmit: (configValues: { [key: string]: string }) => void;
  onCancel: () => void;
}
