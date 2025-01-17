import React from 'react';
import { Modal, ModalContent, ModalHeader, ModalTitle } from '../../ui/modal';

export const ConfirmDeletionModal = ({ keyToDelete, onCancel, onConfirm }) => {
    return (
        <Modal open={!!keyToDelete} onOpenChange={onCancel}>
            <ModalContent>
                <ModalHeader>
                    <ModalTitle>Confirm Deletion</ModalTitle>
                </ModalHeader>
                <div className="p-6">
                    <p className="text-gray-700 dark:text-gray-300">
                        Are you sure you want to delete this API key from the keychain?
                    </p>
                    <div className="mt-6 flex justify-end gap-3">
                        <button
                            onClick={onCancel}
                            className="px-4 py-2 text-gray-700 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-800 rounded-lg"
                        >
                            Cancel
                        </button>
                        <button
                            onClick={onConfirm}
                            className="px-4 py-2 bg-red-500 text-white rounded-lg hover:bg-red-600"
                        >
                            Delete
                        </button>
                    </div>
                </div>
            </ModalContent>
        </Modal>
    );
};
