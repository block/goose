import React, { useCallback, useEffect, useState } from 'react';
import { ScrollArea } from '../../ui/scroll-area';
import BackButton from '../../ui/BackButton';
import { FixedExtensionEntry, useConfig } from '../../ConfigContext';
import { ChevronRight } from 'lucide-react';
import PermissionModal from './PermissionModal';

function RuleItem({ title, description }: { title: string; description: string }) {
  const [isModalOpen, setIsModalOpen] = useState(false);

  const handleModalClose = () => {
    setIsModalOpen(false);
  };

  return (
    <div className="flex items-center justify-between">
      <div>
        <h3 className="font-semibold text-textStandard">{title}</h3>
        <p className="text-xs text-textSubtle mt-1">{description}</p>
      </div>
      <div>
        <button onClick={() => setIsModalOpen(true)}>
          <ChevronRight className="w-4 h-4 text-iconStandard" />
        </button>
      </div>
      {/* Modal for updating tool permission */}
      {isModalOpen && <PermissionModal onClose={handleModalClose} extensionName={title} />}
    </div>
  );
}

function RulesSection({ title, rules }: { title: string; rules: React.ReactNode }) {
  return (
    <div className="space-y-4">
      <h2 className="text-xl font-medium text-textStandard">{title}</h2>
      {rules}
    </div>
  );
}

export default function PermissionSettingsView({ onClose }: { onClose: () => void }) {
  const { getExtensions } = useConfig();
  const [extensions, setExtensions] = useState<FixedExtensionEntry[]>([]);

  const fetchExtensions = useCallback(async () => {
    const extensionsList = await getExtensions(true); // Force refresh
    // Filter out disabled extensions
    const enabledExtensions = extensionsList.filter((extension) => extension.enabled);
    // Sort extensions by name to maintain consistent order
    const sortedExtensions = [...enabledExtensions].sort((a, b) => {
      // First sort by builtin
      if (a.type === 'builtin' && b.type !== 'builtin') return -1;
      if (a.type !== 'builtin' && b.type === 'builtin') return 1;

      // Then sort by bundled (handle null/undefined cases)
      const aBundled = a.bundled === true;
      const bBundled = b.bundled === true;
      if (aBundled && !bBundled) return -1;
      if (!aBundled && bBundled) return 1;

      // Finally sort alphabetically within each group
      return a.name.localeCompare(b.name);
    });
    setExtensions(sortedExtensions);
  }, [getExtensions]);

  useEffect(() => {
    fetchExtensions();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="h-screen w-full animate-[fadein_200ms_ease-in_forwards]">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      <ScrollArea className="h-full w-full">
        <div className="flex flex-col pb-24">
          <div className="px-8 pt-6 pb-4">
            <BackButton onClick={() => onClose()} className="mb-4" />
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="64"
              height="64"
              viewBox="0 0 64 64"
              fill="none"
            >
              <circle cx="32" cy="32" r="32" fill="#101010" />
              <rect x="22" y="17" width="8" height="8" rx="4" fill="white" />
              <rect x="22" y="28" width="20" height="20" rx="10" fill="white" />
            </svg>
            <h1 className="text-3xl font-medium text-textStandard mt-4">Permission Rules</h1>
            <p className="text-textSubtle">
              Hidden instructions that will be passed to the provider to help direct and add context
              to your responses.
            </p>
          </div>

          {/* Content Area */}
          <div className="flex-1 pt-[20px]">
            <div className="space-y-8 px-8">
              {/* Extension Rules Section */}
              <RulesSection
                title="Extension rules"
                rules={
                  <>
                    {extensions.map((extension) => (
                      <RuleItem
                        key={extension.name}
                        title={extension.name}
                        description={'description' in extension ? extension.description || '' : ''}
                      />
                    ))}
                  </>
                }
              />
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
