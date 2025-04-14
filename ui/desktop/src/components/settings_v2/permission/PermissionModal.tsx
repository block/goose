import { useEffect, useState } from 'react';
import { Button } from '../../ui/button';
import { ChevronDownIcon, SlidersHorizontal } from 'lucide-react';
import { getTools, PermissionLevel, ToolInfo, upsertPermissions } from '../../../api';
import * as Dialog from '@radix-ui/react-dialog';
import * as DropdownMenu from '@radix-ui/react-dropdown-menu';

function getFirstSentence(text: string): string {
  const match = text.match(/^([^.?!]+[.?!])/);
  return match ? match[0] : '';
}

interface PermissionModalProps {
  extensionName: string;
  onClose: () => void;
}

export default function PermissionModal({ extensionName, onClose }: PermissionModalProps) {
  const permissionOptions = [
    { value: 'always_allow', label: 'Always allow' },
    { value: 'ask_before', label: 'Ask before' },
    { value: 'never_allow', label: 'Never allow' },
  ] as { value: PermissionLevel; label: string }[];

  const [tools, setTools] = useState<ToolInfo[]>([]);
  const [updatedPermissions, setUpdatedPermissions] = useState<Record<string, string>>({});

  useEffect(() => {
    const fetchTools = async () => {
      try {
        const response = await getTools({ query: { extension_name: extensionName } });
        if (response.error) {
          console.error('Failed to get tools');
        } else {
          setTools(response.data || []);
        }
      } catch (err) {
        console.error('Error fetching tools:', err);
      }
    };

    fetchTools();
  }, [extensionName]);

  const handleSettingChange = (toolName: string, newPermission: PermissionLevel) => {
    setUpdatedPermissions((prev) => ({
      ...prev,
      [toolName]: newPermission,
    }));
  };

  const handleSave = async () => {
    try {
      const payload = {
        tool_permissions: Object.entries(updatedPermissions).map(([toolName, permission]) => ({
          tool_name: toolName,
          permission: permission as PermissionLevel,
        })),
      };

      if (payload.tool_permissions.length === 0) {
        onClose();
        return;
      }

      const response = await upsertPermissions({
        body: payload,
      });
      if (response.error) {
        console.error('Failed to save permissions:', response.error);
      } else {
        console.log('Permissions updated successfully');
        onClose();
      }
    } catch (err) {
      console.error('Error saving permissions:', err);
    }
  };

  const footerContent = (
    <>
      <Button
        onClick={handleSave}
        className="w-full h-[60px] rounded-none border-b border-borderSubtle bg-transparent hover:bg-bgSubtle text-textProminent font-medium text-md"
      >
        Save Changes
      </Button>
      <Button
        onClick={onClose}
        variant="ghost"
        className="w-full h-[60px] rounded-none hover:bg-bgSubtle text-textSubtle hover:text-textStandard text-md font-regular"
      >
        Cancel
      </Button>
    </>
  );

  return (
    <Dialog.Root open={true} onOpenChange={(open) => !open && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/30 dark:bg-white/20 transition-colors animate-[fadein_200ms_ease-in_forwards]" />
        <Dialog.Content className="fixed top-[50%] left-[50%] translate-x-[-50%] translate-y-[-50%] w-[500px] max-w-[90vw] bg-bgApp rounded-xl shadow-xl max-h-[90vh] flex flex-col">
          <div className="p-6">
            <Dialog.Title className="DialogTitle flex justify-start items-center mb-6">
              <SlidersHorizontal className="text-iconStandard" size={24} />
              <p className="ml-2 text-2xl font-semibold text-textStandard">{extensionName}</p>
            </Dialog.Title>
            <Dialog.Description className="DialogDescription"></Dialog.Description>

            <div className="flex justify-center items-center h-full">
              {tools.length === 0 ? (
                <div className="flex items-center justify-center">
                  {/* Loading spinner */}
                  <svg
                    className="animate-spin h-8 w-8 text-grey-50 dark:text-white"
                    xmlns="http://www.w3.org/2000/svg"
                    fill="none"
                    viewBox="0 0 24 24"
                  >
                    <circle
                      className="opacity-25"
                      cx="12"
                      cy="12"
                      r="10"
                      stroke="currentColor"
                      strokeWidth="4"
                    ></circle>
                    <path
                      className="opacity-75"
                      fill="currentColor"
                      d="M4 12a8 8 0 018-8v8H4z"
                    ></path>
                  </svg>
                </div>
              ) : (
                <div>
                  {tools.map((tool) => (
                    <div
                      key={tool.name}
                      className="mb-4 flex items-center justify-between grid grid-cols-12"
                    >
                      <div className="flex flex-col col-span-8">
                        <label className="block text-sm font-medium text-textStandard">
                          {tool.name}
                        </label>
                        <p className="text-sm text-gray-500 mb-2">
                          {getFirstSentence(tool.description)}
                        </p>
                      </div>
                      <DropdownMenu.Root>
                        <DropdownMenu.Trigger className="flex col-span-4 items-center justify-center bg-bgSubtle text-textStandard rounded-full px-3 py-2">
                          <span>
                            {permissionOptions.find(
                              (option) =>
                                option.value === (updatedPermissions[tool.name] || tool.permission)
                            )?.label || 'Ask Before'}
                          </span>
                          <ChevronDownIcon className="ml-2 h-4 w-4" />
                        </DropdownMenu.Trigger>
                        <DropdownMenu.Portal>
                          <DropdownMenu.Content className="bg-white dark:bg-bgProminent rounded-lg shadow-md">
                            {permissionOptions.map((option) => (
                              <DropdownMenu.Item
                                key={option.value}
                                className="px-8 py-2 rounded-lg hover:cursor-pointer hover:bg-bgSubtle text-textStandard"
                                onSelect={() =>
                                  handleSettingChange(tool.name, option.value as PermissionLevel)
                                }
                              >
                                {option.label}
                              </DropdownMenu.Item>
                            ))}
                          </DropdownMenu.Content>
                        </DropdownMenu.Portal>
                      </DropdownMenu.Root>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>

          {footerContent && (
            <div className="border-t border-borderSubtle bg-bgApp w-full rounded-b-xl overflow-hidden">
              {footerContent}
            </div>
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
