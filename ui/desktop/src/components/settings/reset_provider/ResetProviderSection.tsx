import { Button } from '../../ui/button';
import { RefreshCw } from 'lucide-react';
import { useConfig } from '../../ConfigContext';
import { View, ViewOptions } from '../../../utils/navigationUtils';
import { useTranslation } from 'react-i18next';

interface ResetProviderSectionProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}

export default function ResetProviderSection(_props: ResetProviderSectionProps) {
  const { remove } = useConfig();
  const { t } = useTranslation();

  const handleResetProvider = async () => {
    try {
      await remove('GOOSE_PROVIDER', false);
      await remove('GOOSE_MODEL', false);

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
        data-testid="reset-provider-and-model-button"
      >
        <RefreshCw className="h-4 w-4" />
        {t('settings.resetProvider.button')}
      </Button>
      <p className="text-xs text-text-secondary mt-2">
        {t('settings.resetProvider.description')}
      </p>
    </div>
  );
}
