import React from 'react';

interface TooltipProps {
    content: string;
    children: React.ReactNode;
}

export function Tooltip({ content, children }: TooltipProps) {
    return (
        <div className="group relative inline-block">
            {children}
            <div className="opacity-0 group-hover:opacity-100 transition-opacity absolute -top-6 left-1/2 -translate-x-1/2 px-2 py-1 bg-gray-900/75 backdrop-blur-sm text-white text-xs rounded pointer-events-none whitespace-nowrap">
                {content}
            </div>
        </div>
    );
} 