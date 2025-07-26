import React, { useCallback, useEffect, useState } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../../ui/dialog';
import { ScrollArea } from '../../ui/scroll-area';
import { FixedExtensionEntry, useConfig } from '../../ConfigContext';
import { ChevronRight } from 'lucide-react';
import PermissionModal from './PermissionModal';
import { Button } from '../../ui/button';

function RuleItem({ title, description }: { title: string; description: string }) {
  const [isModalOpen, setIsModalOpen] = useState(false);

  const handleModalClose = () => {
    setIsModalOpen(false);
  };

  return (
    <>
      <Button
        className="flex items-center text-left gap-2 w-full justify-between"
        onClick={() => setIsModalOpen(true)}
        variant="secondary"
        size="lg"
      >
        <div>
          <h3 className="font-semibold text-textStandard">{title}</h3>
          <p className="text-xs text-textSubtle mt-1">{description}</p>
        </div>
        <ChevronRight className="w-4 h-4 text-iconStandard" />
      </Button>
      {isModalOpen && <PermissionModal onClose={handleModalClose} extensionName={title} />}
    </>
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

interface PermissionRulesModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function PermissionRulesModal({ isOpen, onClose }: PermissionRulesModalProps) {
  const { getExtensions } = useConfig();
  const [extensions, setExtensions] = useState<FixedExtensionEntry[]>([]);

  const fetchExtensions = useCallback(async () => {
    const extensionsList = await getExtensions(true); // Force refresh
    // Filter out disabled extensions
    const enabledExtensions = extensionsList.filter((extension) => extension.enabled);
    enabledExtensions.push({
      name: 'platform',
      type: 'builtin',
      enabled: true,
    });
    // Sort extensions by name to maintain consistent order
    const sortedExtensions = [...enabledExtensions].sort((a, b) => {
      // First sort by builtin
      if (a.type === 'builtin' && b.type !== 'builtin') return -1;
      if (a.type !== 'builtin' && b.type === 'builtin') return 1;

      // Then sort by bundled (handle null/undefined cases)
      const aBundled = 'bundled' in a && a.bundled === true;
      const bBundled = 'bundled' in b && b.bundled === true;
      if (aBundled && !bBundled) return -1;
      if (!aBundled && bBundled) return 1;

      // Finally sort alphabetically within each group
      return a.name.localeCompare(b.name);
    });
    setExtensions(sortedExtensions);
  }, [getExtensions]);

  useEffect(() => {
    if (isOpen) {
      fetchExtensions();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen]);

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[800px] max-h-[80vh] p-0">
        <DialogHeader className="px-8 pt-6 pb-4">
          <div className="flex items-center gap-4">
            <div className="rounded-full bg-background-inverse w-16 h-16 flex items-center justify-center">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="24"
                height="24"
                viewBox="0 0 24 24"
                className="stroke-text-inverse fill-background-inverse"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="m15.5 7.5 2.3 2.3a1 1 0 0 0 1.4 0l2.1-2.1a1 1 0 0 0 0-1.4L19 4" />
                <path d="m21 2-9.6 9.6" />
                <circle cx="7.5" cy="15.5" r="5.5" />
              </svg>
            </div>
            <div>
              <DialogTitle className="text-3xl font-medium text-textStandard">
                Permission Rules
              </DialogTitle>
              <p className="text-textSubtle">
                Configure tool permissions for extensions to control how they interact with your
                system.
              </p>
            </div>
          </div>
        </DialogHeader>

        <ScrollArea className="flex-1 px-8 pb-8">
          <div className="space-y-8">
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
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}
