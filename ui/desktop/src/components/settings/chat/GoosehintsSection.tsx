import { useState } from 'react';
import { Button } from '../../ui/button';
import { FolderKey } from 'lucide-react';
import { GoosehintsModal } from './GoosehintsModal';
import { useLocalization } from '../../../contexts/LocalizationContext';

export const GoosehintsSection = () => {
  const { t } = useLocalization();
  const [isModalOpen, setIsModalOpen] = useState(false);
  const directory = window.appConfig?.get('GOOSE_WORKING_DIR') as string;

  return (
    <>
      <div className="flex items-center justify-between px-2 py-2">
        <div className="flex-1">
          <h3 className="text-text-primary">{t('gooseHints.title')}</h3>
          <p className="text-xs text-text-secondary mt-[2px]">
            {t('gooseHints.description')}
          </p>
        </div>
        <Button
          onClick={() => setIsModalOpen(true)}
          variant="outline"
          size="sm"
          className="flex items-center gap-2"
        >
          <FolderKey size={16} />
          {t('gooseHints.configure')}
        </Button>
      </div>
      {isModalOpen && (
        <GoosehintsModal directory={directory} setIsGoosehintsModalOpen={setIsModalOpen} />
      )}
    </>
  );
};
