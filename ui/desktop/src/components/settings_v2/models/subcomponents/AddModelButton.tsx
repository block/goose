import React, { useState } from 'react';
import { Button } from '../../../ui/button';
import { AddModelModal } from './AddModelModal';
import type { View } from '../../../../App';
import { ArrowLeftRight } from 'lucide-react';

interface AddModelButtonProps {
  setView: (view: View) => void;
}

export const AddModelButton = ({ setView }: AddModelButtonProps) => {
  const [isAddModelModalOpen, setIsAddModelModalOpen] = useState(false);

  return (
    <>
      <Button
        className="flex items-center gap-2 justify-center text-white dark:text-textSubtle bg-bgAppInverse hover:bg-bgStandardInverse [&>svg]:!size-4"
        onClick={() => setIsAddModelModalOpen(true)}
      >
        <ArrowLeftRight />
        Switch models
      </Button>
      {isAddModelModalOpen ? (
        <AddModelModal setView={setView} onClose={() => setIsAddModelModalOpen(false)} />
      ) : null}
    </>
  );
};
