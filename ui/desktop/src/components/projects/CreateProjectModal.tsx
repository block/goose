import React, { useState, useEffect } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { Textarea } from '../ui/textarea';
import { FolderSearch } from 'lucide-react';
import { toastError } from '../../toasts';

interface CreateProjectModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (name: string, description: string, defaultDirectory: string) => void;
  defaultDirectory?: string;
}

const CreateProjectModal: React.FC<CreateProjectModalProps> = ({
  isOpen,
  onClose,
  onCreate,
  defaultDirectory: defaultDirectoryProp,
}) => {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [defaultDirectory, setDefaultDirectory] = useState(defaultDirectoryProp || '');
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setDefaultDirectory(defaultDirectoryProp || '');
    }
  }, [defaultDirectoryProp, isOpen]);

  const resetForm = () => {
    setName('');
    setDescription('');
    setDefaultDirectory(defaultDirectoryProp || '');
    setIsSubmitting(false);
  };

  const handleClose = () => {
    resetForm();
    onClose();
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (!name.trim()) {
      toastError({ title: 'Error', msg: 'Project name is required' });
      return;
    }

    setIsSubmitting(true);

    // Pass data to parent component
    onCreate(name, description, defaultDirectory);

    // Form will be reset when the modal is closed by the parent
    // after successful creation
  };

  const handlePickDirectory = async () => {
    try {
      // Use Electron's dialog to pick a directory
      const directory = await window.electron.directoryChooser();

      if (!directory.canceled && directory.filePaths.length > 0) {
        setDefaultDirectory(directory.filePaths[0]);
      }
    } catch (err) {
      console.error('Failed to pick directory:', err);
      toastError({ title: 'Error', msg: 'Failed to pick directory' });
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[425px]">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Create new project</DialogTitle>
            <DialogDescription>
              Create a project to group related sessions together
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 py-4">
            <div className="grid gap-2">
              <Label htmlFor="name">Name*</Label>
              <Input
                id="name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="My Project"
                autoFocus
                required
              />
            </div>

            <div className="grid gap-2">
              <Label htmlFor="description">Description</Label>
              <Textarea
                id="description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                className="resize-none"
                placeholder="Optional description"
                rows={2}
              />
            </div>

            <div className="grid gap-2">
              <Label htmlFor="directory">Directory</Label>
              <div className="flex gap-2">
                <Input
                  id="directory"
                  value={defaultDirectory}
                  onChange={(e) => setDefaultDirectory(e.target.value)}
                  className="flex-grow"
                  placeholder="Default working directory for sessions"
                />
                <Button type="button" variant="outline" onClick={handlePickDirectory}>
                  <FolderSearch className="h-4 w-4" />
                </Button>
              </div>
            </div>
          </div>

          <DialogFooter className="pt-2">
            <Button variant="outline" onClick={handleClose} type="button">
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? 'Creating...' : 'Create'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default CreateProjectModal;
