import { Button } from '../../ui/button';
import { RefreshCw } from 'lucide-react';
import { useConfig } from '../../ConfigContext';
import { View, ViewOptions } from '../../../utils/navigationUtils';

interface ResetProviderSectionProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}

export default function ResetProviderSection(_props: ResetProviderSectionProps) {
  const { update } = useConfig();

  const handleResetProvider = async () => {
    try {
      await update({ GOOSE_PROVIDER: null, GOOSE_MODEL: null });

      // Refresh the page to trigger the ProviderGuard check
      window.location.reload();
    } catch (error) {
      console.error('Failed to reset provider and model:', error);
    }
  };

  return (
    <div className="p-2">
      <Button
        onClick={handleResetProvider}
        variant="destructive"
        className="flex items-center justify-center gap-2"
      >
        <RefreshCw className="h-4 w-4" />
        Reset Provider and Model
      </Button>
      <p className="text-xs text-text-secondary mt-2">
        This will clear your selected model and provider settings. If no defaults are available,
        you'll be taken to the welcome screen to set them up again.
      </p>
    </div>
  );
}
