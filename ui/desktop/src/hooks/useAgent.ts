import { useState, useCallback, useRef } from 'react';
import { useConfig } from '../components/ConfigContext';
import { ChatType } from '../types/chat';
import { Recipe } from '../recipe';
import { initializeSystem } from '../utils/providerUtils';
import { initializeCostDatabase } from '../utils/costDatabase';
import {
  backupConfig,
  initConfig,
  Message as ApiMessage,
  readAllConfig,
  recoverConfig,
  resumeAgent,
  startAgent,
  validateConfig,
} from '../api';
import { COST_TRACKING_ENABLED } from '../updates';
import { convertApiMessageToFrontendMessage } from '../components/context_management';
import { fetchSessionDetails } from '../sessions';

export enum AgentState {
  UNINITIALIZED = 'uninitialized',
  INITIALIZING = 'initializing',
  NO_PROVIDER = 'no_provider',
  INITIALIZED = 'initialized',
  ERROR = 'error',
}

export interface InitializationContext {
  recipeConfig?: Recipe;
  resumeSessionId?: string;
  setAgentWaitingMessage: (msg: string | null) => void;
}

interface UseAgentReturn {
  agentState: AgentState;
  resetChat: () => void;
  loadCurrentChat: (context: InitializationContext) => Promise<ChatType>;
}

export function useAgent(): UseAgentReturn {
  const [agentState, setAgentState] = useState<AgentState>(AgentState.UNINITIALIZED);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const initPromiseRef = useRef<Promise<ChatType> | null>(null);

  const { getExtensions, addExtension, read } = useConfig();

  const resetChat = useCallback(() => {
    setSessionId(null);
    setAgentState(AgentState.UNINITIALIZED);
  }, []);

  const currentChat = useCallback(
    async (initContext: InitializationContext): Promise<ChatType> => {
      if (agentState === AgentState.INITIALIZED && sessionId) {
        const sessionDetails = await fetchSessionDetails(sessionId);

        const chat: ChatType = {
          sessionId: sessionDetails.sessionId,
          title: sessionDetails.metadata.description || 'Chat Session',
          messageHistoryIndex: 0,
          messages: sessionDetails.messages,
        };

        // TODO(Douwe): we should store the recipe config on the server so not needed here:
        if (initContext.recipeConfig) {
          chat.title = initContext.recipeConfig.title || chat.title;
          chat.recipeConfig = initContext.recipeConfig;
        }

        return chat;
      }

      if (initPromiseRef.current) {
        return initPromiseRef.current;
      }

      const initPromise = (async () => {
        setAgentState(AgentState.INITIALIZING);
        const agentWaitingMessage = initContext.setAgentWaitingMessage;
        agentWaitingMessage('Agent is initializing');

        try {
          const config = window.electron.getConfig();
          const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
          const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

          if (!provider || !model) {
            setAgentState(AgentState.NO_PROVIDER);
            throw new Error('No provider or model configured');
          }

          const agentResponse = initContext.resumeSessionId
            ? await resumeAgent({
                body: {
                  session_id: initContext.resumeSessionId,
                },
                throwOnError: true,
              })
            : await startAgent({
                body: {
                  working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
                },
                throwOnError: true,
              });

          const agentSessionInfo = agentResponse.data;
          if (!agentSessionInfo) {
            throw Error('Failed to get session info');
          }
          setSessionId(agentSessionInfo.session_id);

          agentWaitingMessage('Agent is loading config');

          await initConfig();

          try {
            await readAllConfig({ throwOnError: true });
          } catch (error) {
            console.warn('Initial config read failed, attempting recovery:', error);
            await handleConfigRecovery();
          }

          agentWaitingMessage('Extensions are loading');
          console.log(`calling initializeSystem with sessionId: ${agentSessionInfo.session_id}`);
          await initializeSystem(agentSessionInfo.session_id, provider as string, model as string, {
            getExtensions,
            addExtension,
          });
          console.log('init system done!!');

          if (COST_TRACKING_ENABLED) {
            try {
              await initializeCostDatabase();
            } catch (error) {
              console.error('Failed to initialize cost database:', error);
            }
          }

          let initChat: ChatType = {
            sessionId: agentSessionInfo.session_id,
            title: agentSessionInfo.metadata.description,
            messageHistoryIndex: 0,
            messages: agentSessionInfo.messages.map((message: ApiMessage) =>
              convertApiMessageToFrontendMessage(message, true, true)
            ),
          };

          if (initContext.recipeConfig) {
            initChat.title = initContext.recipeConfig.title || initChat.title;
            initChat.recipeConfig = initContext.recipeConfig;
          }
          setAgentState(AgentState.INITIALIZED);

          return initChat;
        } catch (error) {
          if ((error + '').includes('Failed to create provider')) {
            setAgentState(AgentState.NO_PROVIDER);
          } else {
            setAgentState(AgentState.ERROR);
          }
          throw error;
        } finally {
          agentWaitingMessage(null);
          initPromiseRef.current = null;
        }
      })();

      initPromiseRef.current = initPromise;
      return initPromise;
    },
    [getExtensions, addExtension, read, agentState, sessionId]
  );

  return {
    agentState,
    resetChat,
    loadCurrentChat: currentChat,
  };
}

const handleConfigRecovery = async () => {
  const configVersion = localStorage.getItem('configVersion');
  const shouldMigrateExtensions = !configVersion || parseInt(configVersion, 10) < 3;

  if (shouldMigrateExtensions) {
    console.log('Performing extension migration...');
    try {
      await backupConfig({ throwOnError: true });
      await initConfig();
    } catch (migrationError) {
      console.error('Migration failed:', migrationError);
    }
  }

  console.log('Attempting config recovery...');
  try {
    await validateConfig({ throwOnError: true });
    await readAllConfig({ throwOnError: true });
  } catch {
    console.log('Config validation failed, attempting recovery...');
    try {
      await recoverConfig({ throwOnError: true });
      await readAllConfig({ throwOnError: true });
    } catch {
      console.warn('Config recovery failed, reinitializing...');
      await initConfig();
    }
  }
};
