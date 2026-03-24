import React from 'react';
import { Card } from './card';

export function BaseModal({
  isOpen,
  title,
  children,
  actions,
}: {
  isOpen: boolean;
  title?: string;
  children: React.ReactNode;
  actions: React.ReactNode; // Buttons for actions
}) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/20 backdrop-blur-sm z-[9999]">
      <Card className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[440px] bg-white dark:bg-gray-800 rounded-xl shadow-xl overflow-hidden p-[16px] pt-[24px] pb-0">
        <div className="px-8 pb-0 space-y-8">
          {/* Header */}
          {title && (
            <div className="flex">
              <h2 className="text-2xl font-regular dark:text-white text-gray-900">{title}</h2>
            </div>
          )}

          {/* Content */}
          {children && <div className="px-8">{children}</div>}

          {/* Actions */}
          <div className="mt-[8px] ml-[-24px] mr-[-24px] pt-[16px]">{actions}</div>
        </div>
      </Card>
    </div>
  );
}
