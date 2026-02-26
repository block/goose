import { FolderKey } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/atoms/button';
import { GoosehintsModal } from './GoosehintsModal';

export const GoosehintsSection = () => {
  const [isModalOpen, setIsModalOpen] = useState(false);
  const directory = window.appConfig?.get('GOOSE_WORKING_DIR') as string;

  return (
    <>
      <div className="flex items-center justify-between px-2 py-2">
        <div className="flex-1">
          <h3 className="text-text-default">Project Hints (.goosehints)</h3>
          <p className="text-xs text-text-muted mt-[2px]">
            Configure your project's .goosehints file to provide additional context to Goose
          </p>
        </div>
        <Button
          onClick={() => setIsModalOpen(true)}
          variant="outline"
          size="sm"
          className="flex items-center gap-2"
        >
          <FolderKey size={16} />
          Configure
        </Button>
      </div>
      {isModalOpen && (
        <GoosehintsModal directory={directory} setIsGoosehintsModalOpen={setIsModalOpen} />
      )}
    </>
  );
};
