export default interface ProviderCallbacks {
  onShowModal?: () => void;
  onAdd?: () => void;
  onDelete?: () => void;
  onShowSettings?: () => void;
  onRefresh?: () => void;
}
