import React from 'react';
import { BaseDialog } from './BaseDialog';
import { Model } from '../types';
import { Input } from '../../ui/input';

interface AddModelDialogProps {
    isOpen: boolean;
    onClose: () => void;
    onAdd: (model: Model) => void;
}

export function AddModelDialog({ isOpen, onClose, onAdd }: AddModelDialogProps) {
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
            enabled: false
        });
        onClose();
    };

    return (
        <BaseDialog 
            title="Add Model" 
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
                        placeholder="Model name"
                        required
                    />
                </div>
                <div>
                    <label className="text-sm font-medium">Description</label>
                    <Input
                        value={description}
                        onChange={(e) => setDescription(e.target.value)}
                        placeholder="Model description"
                    />
                </div>
            </form>
        </BaseDialog>
    );
} 