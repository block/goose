import React from 'react';

interface ToggleableItemProps {
    id: string;
    name: string;
    description: string;
    enabled: boolean;
    onToggle: (id: string) => void;
}

export function ToggleableItem({
    id,
    name,
    description,
    enabled,
    onToggle,
}: ToggleableItemProps) {
    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg p-4 mb-2">
            <div className="flex justify-between items-center">
                <h3 className="text-lg font-medium dark:text-white">{name}</h3>
                <button
                    onClick={() => onToggle(id)}
                    className={`relative inline-flex h-6 w-11 items-center rounded-full ${
                        enabled ? "bg-indigo-500" : "bg-gray-200 dark:bg-gray-600"
                    } transition-colors duration-200 ease-in-out focus:outline-none`}
                >
                    <span
                        className={`inline-block h-5 w-5 transform rounded-full bg-white shadow ${
                            enabled ? "translate-x-[22px]" : "translate-x-[2px]"
                        } transition-transform duration-200 ease-in-out`}
                    />
                </button>
            </div>
            <p className="text-gray-500 dark:text-gray-400 text-sm mt-1">{description}</p>
        </div>
    );
} 