import React from 'react';

type SettingValue = boolean | string | number;

interface BaseSettingItem {
    id: string;
    title: string;
    description: string;
    type: 'toggle' | 'text' | 'password' | 'number';
    value: SettingValue;
    onChange: (value: SettingValue) => void;
    behavior?: {
        // For toggle type - if true, will deactivate other items in the same group
        exclusive?: boolean;
        // For toggle type - if true, cannot be turned off directly
        requireActive?: boolean;
        // Group ID for related settings (like all models, all MCPs, etc)
        group?: string;
    };
}

export function SettingItem({ 
    title, 
    description, 
    type, 
    value, 
    onChange,
    behavior 
}: BaseSettingItem) {
    const renderInput = () => {
        switch (type) {
            case 'toggle':
                return (
                    <button 
                        onClick={() => {
                            if (behavior?.requireActive && value === true) {
                                return; // Prevent turning off if requireActive is true
                            }
                            onChange(!value);
                        }}
                        className={`
                            relative inline-flex h-6 w-11 items-center rounded-full
                            ${value ? 'bg-indigo-500' : 'bg-gray-200 dark:bg-gray-600'}
                            transition-colors duration-200 ease-in-out focus:outline-none
                        `}
                    >
                        <span
                            className={`
                                inline-block h-5 w-5 transform rounded-full bg-white shadow
                                transition-transform duration-200 ease-in-out
                                ${value ? 'translate-x-[22px]' : 'translate-x-[2px]'}
                            `}
                        />
                    </button>
                );
            case 'text':
            case 'password':
                return (
                    <input
                        type={type}
                        value={value as string}
                        onChange={(e) => onChange(e.target.value)}
                        className="bg-transparent border-b border-gray-300 dark:border-gray-600 focus:outline-none focus:border-indigo-500"
                    />
                );
            case 'number':
                return (
                    <input
                        type="number"
                        value={value as number}
                        onChange={(e) => onChange(Number(e.target.value))}
                        className="bg-transparent border-b border-gray-300 dark:border-gray-600 focus:outline-none focus:border-indigo-500"
                    />
                );
        }
    };

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg p-4 mb-2">
            <div className="flex justify-between items-center">
                <h3 className="text-lg font-medium dark:text-white">{title}</h3>
                {renderInput()}
            </div>
            <p className="text-gray-500 dark:text-gray-400 text-sm mt-1">{description}</p>
        </div>
    );
} 