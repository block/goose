import React from 'react';

interface SectionHeaderProps {
    title: string;
    buttonText?: string;
    onAction: () => void;
}

export function SectionHeader({ title, buttonText, onAction }: SectionHeaderProps) {
    return (
        <div className="flex justify-between items-center mb-4">
            <h2 className="text-2xl font-semibold">{title}</h2>
            <button 
                onClick={onAction}
                className="text-indigo-500 hover:text-indigo-600 font-medium"
            >
                {buttonText || `Add ${title}`}
            </button>
        </div>
    );
} 