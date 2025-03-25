import React, { useState } from 'react';
import { Button } from '../../../ui/button';
import { AddModelModal } from './AddModelModal';
import { Gear } from '../../../icons';

export const AddModelButton = () => {
  const [isAddModelModalOpen, setIsAddModelModalOpen] = useState(false);

  return (
    <>
      <Button
        className="flex items-center gap-2 flex-1 justify-center text-white dark:text-textSubtle bg-black dark:bg-white hover:bg-subtle"
        onClick={() => setIsAddModelModalOpen(true)}
      >
        <Gear className="h-4 w-4" />
        Switch Models
      </Button>
      {isAddModelModalOpen ? <AddModelModal onClose={() => setIsAddModelModalOpen(false)} /> : null}
    </>
  );
};
