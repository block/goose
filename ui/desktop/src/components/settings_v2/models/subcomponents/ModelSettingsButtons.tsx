import { AddModelButton } from './AddModelButton';
import { Button } from '../../../ui/button';
import { Sliders } from 'lucide-react';
import React from 'react';
import type { View } from '../../../../App';

interface ConfigureModelButtonsProps {
  setView: (view: View) => void;
}

export default function ModelSettingsButtons({ setView }: ConfigureModelButtonsProps) {
  return (
    <div className="flex gap-4 pt-4 w-full">
      <AddModelButton setView={setView} />
      <Button
        className="flex items-center gap-2 flex-1 justify-center text-textSubtle bg-white dark:bg-black hover:bg-subtle dark:border dark:border-gray-500 dark:hover:border-gray-400"
        onClick={() => {
          setView('ConfigureProviders');
        }}
      >
        <Sliders className="h-4 w-4 rotate-90" />
        Configure Providers
      </Button>
    </div>
  );
}
