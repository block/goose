import React from 'react';

export const Switch = ({ checked, onCheckedChange }: { checked: boolean; onCheckedChange: (checked: boolean) => void }) => {
    return (
        <button
            type="button"
            role="switch"
            aria-checked={checked}
            className={`relative inline-flex items-center h-6 rounded-full w-11 transition-colors ${
                checked ? 'bg-blue-500' : 'bg-gray-300'
            }`}
            onClick={() => onCheckedChange(!checked)}
        >
            <span
                className={`inline-block w-4 h-4 transform bg-white rounded-full transition-transform ${
                    checked ? 'translate-x-5' : 'translate-x-1'
                }`}
            />
        </button>
    );
};
