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
    <div className="flex gap-4 pt-4 ">
      <AddModelButton setView={setView} />
      <Button
        className="flex items-center gap-2 justify-center text-textStandard bg-bgApp border border-borderSubtle hover:border-borderProminent hover:bg-bgApp [&>svg]:!size-4"
        onClick={() => {
          setView('ConfigureProviders');
        }}
      >
        <Sliders className="rotate-90" />
        Configure providers
      </Button>
    </div>
  );
}
