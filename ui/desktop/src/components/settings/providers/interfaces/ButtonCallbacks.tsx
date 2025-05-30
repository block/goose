import ProviderState from '../interfaces/ProviderState';

export default interface ButtonCallbacks {
  onConfigure?: (provider: ProviderState) => void;
  onLaunch?: (provider: ProviderState) => void;
}
