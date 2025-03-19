import React, { useState } from 'react';
import { Plus } from 'lucide-react';
import { Button } from '../../ui/button';
import { AddModelModal } from './AddModelModal';

export const AddModelButton = () => {
  const [isAddModelModalOpen, setIsAddModelModalOpen] = useState(false);

  return (
    <>
      <Button
        className="flex items-center gap-2 flex-1 justify-center text-white dark:text-textSubtle bg-black dark:bg-white hover:bg-subtle"
        onClick={() => setIsAddModelModalOpen(true)}
      >
        <Plus className="h-4 w-4" />
        Add Model
      </Button>
      {isAddModelModalOpen ? <AddModelModal onClose={() => setIsAddModelModalOpen(false)} /> : null}
    </>
  );
};
