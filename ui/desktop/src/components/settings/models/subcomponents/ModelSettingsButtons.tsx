import { useState } from 'react';
import { Button } from '../../../ui/button';
import { AddModelModal } from './AddModelModal';
import type { View } from '../../../../App';
import { shouldShowPredefinedModels } from '../predefinedModelsUtils';

interface ConfigureModelButtonsProps {
  setView: (view: View) => void;
}

export default function ModelSettingsButtons({ setView }: ConfigureModelButtonsProps) {
  const [isAddModelModalOpen, setIsAddModelModalOpen] = useState(false);
  const hasPredefinedModels = shouldShowPredefinedModels();

  return (
    <div className="flex gap-2 pt-4">
      <Button
        className="flex items-center gap-2 justify-center"
        variant="default"
        size="sm"
        onClick={() => setIsAddModelModalOpen(true)}
      >
        Switch models
      </Button>
      {isAddModelModalOpen ? (
        <AddModelModal setView={setView} onClose={() => setIsAddModelModalOpen(false)} />
      ) : null}
      {!hasPredefinedModels && (
        <Button
          className="flex items-center gap-2 justify-center"
          variant="secondary"
          size="sm"
          onClick={() => {
            setView('ConfigureProviders');
          }}
        >
          Configure providers
        </Button>
      )}
    </div>
  );
}
