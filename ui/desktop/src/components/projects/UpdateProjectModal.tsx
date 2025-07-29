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
import { toastError, toastSuccess } from '../../toasts';
import { ProjectMetadata, updateProject, UpdateProjectRequest } from '../../projects';

interface UpdateProjectModalProps {
  isOpen: boolean;
  onClose: () => void;
  project: ProjectMetadata;
  onRefresh: () => void;
}

const UpdateProjectModal: React.FC<UpdateProjectModalProps> = ({
  isOpen,
  onClose,
  project,
  onRefresh,
}) => {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [defaultDirectory, setDefaultDirectory] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Initialize form with project data
  useEffect(() => {
    if (isOpen && project) {
      setName(project.name);
      setDescription(project.description || '');
      setDefaultDirectory(project.defaultDirectory);
    }
  }, [isOpen, project]);

  const handleClose = () => {
    onClose();
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!name.trim()) {
      toastError({ title: 'Error', msg: 'Project name is required' });
      return;
    }

    if (!defaultDirectory.trim()) {
      toastError({ title: 'Error', msg: 'Default directory is required' });
      return;
    }

    setIsSubmitting(true);

    try {
      // Create update object, only include changed fields
      const updateData: UpdateProjectRequest = {};

      if (name !== project.name) {
        updateData.name = name;
      }

      if (description !== (project.description || '')) {
        updateData.description = description || null;
      }

      if (defaultDirectory !== project.defaultDirectory) {
        updateData.defaultDirectory = defaultDirectory;
      }

      // Only make the API call if there are changes
      if (Object.keys(updateData).length > 0) {
        await updateProject(project.id, updateData);
        toastSuccess({ title: 'Success', msg: 'Project updated successfully' });
        onRefresh();
      }

      onClose();
    } catch (err) {
      console.error('Failed to update project:', err);
      toastError({ title: 'Error', msg: 'Failed to update project' });
    } finally {
      setIsSubmitting(false);
    }
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
            <DialogTitle>Edit Project</DialogTitle>
            <DialogDescription>Update project information</DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 py-4">
            <div className="grid grid-cols-4 items-center gap-2">
              <Label htmlFor="name" className="text-right">
                Name*
              </Label>
              <Input
                id="name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="col-span-3"
                autoFocus
                required
              />
            </div>

            <div className="grid grid-cols-4 items-start gap-2">
              <Label htmlFor="description" className="text-right pt-2">
                Description
              </Label>
              <Textarea
                id="description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                className="col-span-3 resize-none"
                placeholder="Optional description"
                rows={2}
              />
            </div>

            <div className="grid grid-cols-4 items-center gap-2">
              <Label htmlFor="directory" className="text-right">
                Directory*
              </Label>
              <div className="col-span-3 flex gap-2">
                <Input
                  id="directory"
                  value={defaultDirectory}
                  onChange={(e) => setDefaultDirectory(e.target.value)}
                  className="flex-grow"
                  required
                />
                <Button type="button" variant="outline" onClick={handlePickDirectory}>
                  <FolderSearch className="h-4 w-4" />
                </Button>
              </div>
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={handleClose} type="button">
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? 'Saving...' : 'Save Changes'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
};

export default UpdateProjectModal;
