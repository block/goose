import React from 'react';
import { SectionHeader } from './SectionHeader';

interface SettingsSectionProps {
    title: string;
    buttonText?: string;
    onAction: () => void;
    description?: string;
    children: React.ReactNode;
}

export function SettingsSection({ 
    title, 
    buttonText, 
    onAction, 
    description, 
    children 
}: SettingsSectionProps) {
    return (
        <section className="mb-10">
            <SectionHeader 
                title={title} 
                buttonText={buttonText} 
                onAction={onAction} 
            />
            {description && (
                <div className="text-gray-500 dark:text-gray-400 mb-4">
                    {description}
                </div>
            )}
            {children}
        </section>
    );
} 