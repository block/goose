import React, { useState } from 'react';
import { Button } from '../../../ui/button';
import { AddModelModal } from './AddModelModal';
import { Gear } from '../../../icons';
import type { View } from '../../../../App';

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
        <Gear />
        Switch Models
      </Button>
      {isAddModelModalOpen ? (
        <AddModelModal setView={setView} onClose={() => setIsAddModelModalOpen(false)} />
      ) : null}
    </>
  );
};
