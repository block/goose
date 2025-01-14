import React from 'react';
import { BaseDialog } from './BaseDialog';
import { Key } from '../types';
import { Input } from '../../ui/input';

interface KeyDialogProps {
    isOpen: boolean;
    onClose: () => void;
    onSubmit: (key: Key) => void;
    onDelete?: (key: Key) => void;
    initialKey?: Key;
}

export function KeyDialog({ isOpen, onClose, onSubmit, onDelete, initialKey }: KeyDialogProps) {
    const [name, setName] = React.useState(initialKey?.name || '');
    const [value, setValue] = React.useState(initialKey?.value || '');
    const [isValueVisible, setIsValueVisible] = React.useState(false);
    const isEditing = !!initialKey;

    // Reset form when dialog closes (only if not editing)
    React.useEffect(() => {
        if (!isOpen && !isEditing) {
            setName('');
            setValue('');
            setIsValueVisible(false);
        }
    }, [isOpen, isEditing]);

    // Update form when initialKey changes
    React.useEffect(() => {
        if (initialKey) {
            setName(initialKey.name);
            setValue(initialKey.value);
        }
    }, [initialKey]);

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        const id = initialKey?.id || name.toLowerCase().replace(/\s+/g, '-');
        onSubmit({
            id,
            name,
            value
        });
        onClose();
    };

    return (
        <BaseDialog 
            title={isEditing ? 'Edit Key' : 'Add Key'} 
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
                        placeholder="Key name"
                        required
                    />
                </div>
                <div>
                    <label className="text-sm font-medium">Value</label>
                    <div className="relative">
                        <Input
                            type={isValueVisible ? "text" : "password"}
                            value={value}
                            onChange={(e) => setValue(e.target.value)}
                            placeholder="Key value"
                            required
                        />
                        <button
                            type="button"
                            onClick={() => setIsValueVisible(!isValueVisible)}
                            className="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
                        >
                            {isValueVisible ? '👁️' : '👁️‍🗨️'}
                        </button>
                    </div>
                </div>
                
                {isEditing && onDelete && (
                    <div className="pt-2">
                        <button
                            type="button"
                            onClick={() => {
                                onDelete(initialKey!);
                                onClose();
                            }}
                            className="w-full p-2 text-red-600 hover:text-red-700 dark:text-red-500 dark:hover:text-red-400 border border-red-200 dark:border-red-800 rounded-lg"
                        >
                            Delete Key
                        </button>
                    </div>
                )}
            </form>
        </BaseDialog>
    );
} 