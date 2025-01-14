import React from 'react';
import { Modal, ModalContent, ModalHeader, ModalTitle } from '../../ui/modal';
import { Button } from '../../ui/button';
import { Key } from '../types';

interface RevealKeysDialogProps {
    isOpen: boolean;
    onClose: () => void;
    keys: Key[];
}

export function RevealKeysDialog({ isOpen, onClose, keys }: RevealKeysDialogProps) {
    return (
        <Modal open={isOpen} onOpenChange={onClose}>
            <ModalContent>
                <ModalHeader>
                    <ModalTitle>All Keys (Development Only)</ModalTitle>
                </ModalHeader>
                <div className="space-y-4 max-h-[60vh] overflow-y-auto">
                    {keys.map(key => (
                        <div key={key.id} className="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
                            <div className="font-medium text-sm text-gray-700 dark:text-gray-300">
                                {key.name}
                            </div>
                            <div className="mt-1 font-mono text-sm text-gray-600 dark:text-gray-400 break-all">
                                {key.value}
                            </div>
                        </div>
                    ))}
                </div>
                <div className="flex justify-end">
                    <Button variant="outline" onClick={onClose}>Close</Button>
                </div>
            </ModalContent>
        </Modal>
    );
} 