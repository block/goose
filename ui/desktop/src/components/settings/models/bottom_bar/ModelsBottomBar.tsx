import { Sliders, Bot } from 'lucide-react';
import React, { useEffect, useState } from 'react';
import { useModelAndProvider } from '../../../ModelAndProviderContext';
import { SwitchModelModal } from '../subcomponents/SwitchModelModal';
import { LeadWorkerSettings } from '../subcomponents/LeadWorkerSettings';
import { View } from '../../../../utils/navigationUtils';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '../../../ui/dropdown-menu';
import { useCurrentModelInfo } from '../../../BaseChat';
import { useConfig } from '../../../ConfigContext';
import { getProviderMetadata } from '../modelInterface';
import { getModelDisplayName } from '../predefinedModelsUtils';
import { Alert } from '../../../alerts';
import BottomMenuAlertPopover from '../../../bottom_menu/BottomMenuAlertPopover';

interface ModelsBottomBarProps {
  sessionId: string | null;
  dropdownRef: React.RefObject<HTMLDivElement>;
  setView: (view: View) => void;
  alerts: Alert[];
}

export default function ModelsBottomBar({
  sessionId,
  dropdownRef,
  setView,
  alerts,
}: ModelsBottomBarProps) {
  const {
    currentModel,
    currentProvider,
    currentVariant,
  } = useModelAndProvider();
  const currentModelInfo = useCurrentModelInfo();
  const { read, getProviders } = useConfig();
  const [displayProvider, setDisplayProvider] = useState<string | null>(null);
  const [displayModelName, setDisplayModelName] = useState<string>('Select Model');
  const [isAddModelModalOpen, setIsAddModelModalOpen] = useState(false);
  const [isLeadWorkerModalOpen, setIsLeadWorkerModalOpen] = useState(false);
  const [isLeadWorkerActive, setIsLeadWorkerActive] = useState(false);
  const [providerDefaultModel, setProviderDefaultModel] = useState<string | null>(null);

  // Check if lead/worker mode is active
  useEffect(() => {
    const checkLeadWorker = async () => {
      try {
        const leadModel = await read('GOOSE_LEAD_MODEL', false);
        setIsLeadWorkerActive(!!leadModel);
      } catch (error) {
        console.error('Error checking lead model:', error);
        setIsLeadWorkerActive(false);
      }
    };
    checkLeadWorker();
  }, [read]);

  // Refresh lead/worker status when modal closes
  const handleLeadWorkerModalClose = () => {
    setIsLeadWorkerModalOpen(false);
    const checkLeadWorker = async () => {
      try {
        const leadModel = await read('GOOSE_LEAD_MODEL', false);
        const currentModel = await read('GOOSE_MODEL', false);
        setIsLeadWorkerActive(!!leadModel);
        setLeadModelName((leadModel as string) || '');
        setCurrentActiveModel((currentModel as string) || '');
      } catch (error) {
        console.error('Error checking lead model after modal close:', error);
        setIsLeadWorkerActive(false);
      }
    };
    checkLeadWorker();
  };

  const [leadModelName, setLeadModelName] = useState<string>('');
  const [currentActiveModel, setCurrentActiveModel] = useState<string>('');

  // Get lead model name and current model for comparison
  useEffect(() => {
    const getModelInfo = async () => {
      try {
        const leadModel = await read('GOOSE_LEAD_MODEL', false);
        const currentModel = await read('GOOSE_MODEL', false);
        setLeadModelName((leadModel as string) || '');
        setCurrentActiveModel((currentModel as string) || '');
      } catch (error) {
        console.error('Error getting model info:', error);
      }
    };
    getModelInfo();
  }, [read]);

  // Determine the mode based on which model is currently active
  const modelMode = isLeadWorkerActive
    ? currentActiveModel === leadModelName
      ? 'lead'
      : 'worker'
    : undefined;

  // Determine which model to display - activeModel takes priority when lead/worker is active
  const displayModel =
    isLeadWorkerActive && currentModelInfo?.model
      ? currentModelInfo.model
      : currentModel || providerDefaultModel || displayModelName;

  useEffect(() => {
    if (!currentProvider) return;
    getProviderMetadata(currentProvider, getProviders)
      .then((metadata) => {
        setDisplayProvider(metadata.display_name || currentProvider);
      })
      .catch(() => {
        setDisplayProvider(currentProvider);
      });
  }, [currentProvider, currentModel, getProviders]);

  // Fetch provider default model when provider changes and no current model
  useEffect(() => {
    if (currentProvider && !currentModel) {
      (async () => {
        try {
          const metadata = await getProviderMetadata(currentProvider, getProviders);
          setProviderDefaultModel(metadata.default_model);
        } catch (error) {
          console.error('Failed to get provider default model:', error);
          setProviderDefaultModel(null);
        }
      })();
    } else if (currentModel) {
      setProviderDefaultModel(null);
    }
  }, [currentProvider, currentModel, getProviders]);

  useEffect(() => {
    if (!currentModel) return;
    setDisplayModelName(getModelDisplayName(currentModel));
  }, [currentModel]);

  return (
    <div className="relative flex items-center" ref={dropdownRef}>
      <BottomMenuAlertPopover alerts={alerts} />
      <DropdownMenu>
        <DropdownMenuTrigger className="flex items-center hover:cursor-pointer max-w-[180px] md:max-w-[200px] lg:max-w-[380px] min-w-0 text-text-default/70 hover:text-text-default transition-colors">
          <div className="flex items-center truncate max-w-[130px] md:max-w-[200px] lg:max-w-[360px] min-w-0">
            <Bot className="mr-1 h-4 w-4 flex-shrink-0" />
            <span className="truncate text-xs">
              {displayModel}
              {currentVariant && (
                <span className="ml-1 text-[10px] opacity-60">({currentVariant})</span>
              )}
              {isLeadWorkerActive && modelMode && (
                <span className="ml-1 text-[10px] opacity-60">({modelMode})</span>
              )}
            </span>
          </div>
        </DropdownMenuTrigger>
        <DropdownMenuContent side="top" align="center" className="w-64 text-sm">
          <h6 className="text-xs text-text-default mt-2 ml-2">Current model</h6>
          <p className="flex items-center justify-between text-sm mx-2 pb-2 border-b mb-2">
            {displayModelName}
            {displayProvider && ` — ${displayProvider}`}
          </p>
          <DropdownMenuItem onClick={() => setIsAddModelModalOpen(true)}>
            <span>Change Model</span>
            <Sliders className="ml-auto h-4 w-4 rotate-90" />
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => setIsLeadWorkerModalOpen(true)}>
            <span>Lead/Worker Settings</span>
            <Sliders className="ml-auto h-4 w-4" />
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      {isAddModelModalOpen ? (
        <SwitchModelModal
          sessionId={sessionId}
          setView={setView}
          onClose={() => setIsAddModelModalOpen(false)}
        />
      ) : null}

      {isLeadWorkerModalOpen ? (
        <LeadWorkerSettings isOpen={isLeadWorkerModalOpen} onClose={handleLeadWorkerModalClose} />
      ) : null}
    </div>
  );
}
