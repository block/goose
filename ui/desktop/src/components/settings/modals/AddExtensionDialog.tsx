import React from 'react';
import { BaseDialog } from './BaseDialog';
import { Extension } from '../types';
import { Input } from '../../ui/input';

interface AddExtensionDialogProps {
    isOpen: boolean;
    onClose: () => void;
    onAdd: (extension: Extension) => void;
}

export function AddExtensionDialog({ isOpen, onClose, onAdd }: AddExtensionDialogProps) {
    const [name, setName] = React.useState('');
    const [description, setDescription] = React.useState('Standard config');

    // Reset form when dialog closes
    React.useEffect(() => {
        if (!isOpen) {
            setName('');
            setDescription('Standard config');
        }
    }, [isOpen]);

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        const id = name.toLowerCase().replace(/\s+/g, '-');
        onAdd({
            id,
            name,
            description,
            enabled: true
        });
        onClose();
    };

    return (
        <BaseDialog 
            title="Add Extension" 
            isOpen={isOpen} 
            onClose={onClose}
            onSubmit={handleSubmit}
        >
            <form id="keyForm" onSubmit={handleSubmit} className="space-y-4">
                <div>
                    <label className="text-sm font-medium">Name</label>
                    <Input
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                        placeholder="Extension name"
                        required
                    />
                </div>
                <div>
                    <label className="text-sm font-medium">Description</label>
                    <Input
                        value={description}
                        onChange={(e) => setDescription(e.target.value)}
                        placeholder="Extension description"
                    />
                </div>
            </form>
        </BaseDialog>
    );
} 