import React, { useEffect, useRef, useState } from 'react';
import { Message, useChat } from './ai-sdk-fork/useChat';
import { getApiUrl, getSecretKey } from './config';
import BottomMenu from './components/BottomMenu';
import FlappyGoose from './components/FlappyGoose';
import GooseMessage from './components/GooseMessage';
import Input from './components/Input';
import LoadingGoose from './components/LoadingGoose';
import MoreMenu from './components/MoreMenu';
import { Card } from './components/ui/card';
import { ScrollArea, ScrollAreaHandle } from './components/ui/scroll-area';
import UserMessage from './components/UserMessage';
import WingToWing, { Working } from './components/WingToWing';
import { askAi } from './utils/askAI';
import { getStoredModel, Provider } from './utils/providerUtils';
import { ChatLayout } from './components/chat_window/ChatLayout';
import { WelcomeScreen } from './components/welcome_screen/WelcomeScreen';
import { getStoredProvider, initializeSystem } from './utils/providerUtils';
import { useModel } from './components/settings/models/ModelContext';
import { useRecentModels } from './components/settings/models/RecentModels';
import { createSelectedModel } from './components/settings/models/utils';
import { getDefaultModel } from './components/settings/models/hardcoded_stuff';
import Splash from './components/Splash';
import Settings from './components/settings/Settings';
import MoreModelsSettings from './components/settings/models/MoreModels';
import ConfigureProviders from './components/settings/providers/ConfigureProviders';
import { ConfigPage } from './components/pages/ConfigPage';

export interface Chat {
  id: number;
  title: string;
  messages: Array<{
    id: string;
    role: 'function' | 'system' | 'user' | 'assistant' | 'data' | 'tool';
    content: string;
  }>;
}

export type View =
  | 'welcome'
  | 'chat'
  | 'settings'
  | 'moreModels'
  | 'configureProviders'
  | 'configPage';

// This component is our main chat content.
// We'll move the majority of chat logic here, minus the 'view' state.
export function ChatContent({
  chats,
  setChats,
  selectedChatId,
  setSelectedChatId,
  initialQuery,
  setProgressMessage,
  setWorking,
  setView,
}: {
  chats: Chat[];
  setChats: React.Dispatch<React.SetStateAction<Chat[]>>;
  selectedChatId: number;
  setSelectedChatId: React.Dispatch<React.SetStateAction<number>>;
  initialQuery: string | null;
  setProgressMessage: React.Dispatch<React.SetStateAction<string>>;
  setWorking: React.Dispatch<React.SetStateAction<Working>>;
  setView: (view: View) => void;
}) {
  const chat = chats.find((c: Chat) => c.id === selectedChatId);
  const [messageMetadata, setMessageMetadata] = useState<Record<string, string[]>>({});
  const [hasMessages, setHasMessages] = useState(false);
  const [lastInteractionTime, setLastInteractionTime] = useState<number>(Date.now());
  const [showGame, setShowGame] = useState(false);
  const [working, setWorkingLocal] = useState<Working>(Working.Idle);
  const scrollRef = useRef<ScrollAreaHandle>(null);

  useEffect(() => {
    setWorking(working);
  }, [working, setWorking]);

  const updateWorking = (newWorking: Working) => {
    setWorkingLocal(newWorking);
  };

  const { messages, append, stop, isLoading, error, setMessages } = useChat({
    api: getApiUrl('/reply'),
    initialMessages: chat?.messages || [],
    onToolCall: ({ toolCall }) => {
      updateWorking(Working.Working);
      setProgressMessage(`Executing tool: ${toolCall.toolName}`);
    },
    onResponse: (response) => {
      if (!response.ok) {
        setProgressMessage('An error occurred while receiving the response.');
        updateWorking(Working.Idle);
      } else {
        setProgressMessage('thinking...');
        updateWorking(Working.Working);
      }
    },
    onFinish: async (message, _) => {
      window.electron.stopPowerSaveBlocker();
      setTimeout(() => {
        setProgressMessage('Task finished. Click here to expand.');
        updateWorking(Working.Idle);
      }, 500);

      const fetchResponses = await askAi(message.content);
      setMessageMetadata((prev) => ({ ...prev, [message.id]: fetchResponses }));

      const timeSinceLastInteraction = Date.now() - lastInteractionTime;
      window.electron.logInfo('last interaction:' + lastInteractionTime);
      if (timeSinceLastInteraction > 60000) {
        // 60000ms = 1 minute
        window.electron.showNotification({
          title: 'Goose finished the task.',
          body: 'Click here to expand.',
        });
      }
    },
  });

  // Update chat messages when they change
  useEffect(() => {
    const updatedChats = chats.map((c) => (c.id === selectedChatId ? { ...c, messages } : c));
    setChats(updatedChats);
  }, [messages, selectedChatId]);

  const initialQueryAppended = useRef(false);
  useEffect(() => {
    if (initialQuery && !initialQueryAppended.current) {
      append({ role: 'user', content: initialQuery });
      initialQueryAppended.current = true;
    }
  }, [initialQuery]);

  useEffect(() => {
    if (messages.length > 0) {
      setHasMessages(true);
    }
  }, [messages]);

  // Handle submit
  const handleSubmit = (e: React.FormEvent) => {
    window.electron.startPowerSaveBlocker();
    const customEvent = e as CustomEvent;
    const content = customEvent.detail?.value || '';
    if (content.trim()) {
      setLastInteractionTime(Date.now());
      append({
        role: 'user',
        content,
      });
      if (scrollRef.current?.scrollToBottom) {
        scrollRef.current.scrollToBottom();
      }
    }
  };

  if (error) {
    console.log('Error:', error);
  }

  const onStopGoose = () => {
    stop();
    setLastInteractionTime(Date.now());
    window.electron.stopPowerSaveBlocker();

    const lastMessage: Message = messages[messages.length - 1];
    if (lastMessage.role === 'user' && lastMessage.toolInvocations === undefined) {
      // Remove the last user message.
      if (messages.length > 1) {
        setMessages(messages.slice(0, -1));
      } else {
        setMessages([]);
      }
    } else if (lastMessage.role === 'assistant' && lastMessage.toolInvocations !== undefined) {
      // Add messaging about interrupted ongoing tool invocations
      const newLastMessage: Message = {
        ...lastMessage,
        toolInvocations: lastMessage.toolInvocations.map((invocation) => {
          if (invocation.state !== 'result') {
            return {
              ...invocation,
              result: [
                {
                  audience: ['user'],
                  text: 'Interrupted.\n',
                  type: 'text',
                },
                {
                  audience: ['assistant'],
                  text: 'Interrupted by the user to make a correction.\n',
                  type: 'text',
                },
              ],
              state: 'result',
            };
          } else {
            return invocation;
          }
        }),
      };

      const updatedMessages = [...messages.slice(0, -1), newLastMessage];
      setMessages(updatedMessages);
    }
  };

  return (
    <div className="flex flex-col w-full h-screen items-center justify-center">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle border-b border-borderSubtle">
        {/* Pass setView to MoreMenu so it can switch to settings or other views */}
        <MoreMenu setView={setView} />
      </div>
      <Card className="flex flex-col flex-1 rounded-none h-[calc(100vh-95px)] w-full bg-bgApp mt-0 border-none relative">
        {messages.length === 0 ? (
          <Splash append={append} />
        ) : (
          <ScrollArea ref={scrollRef} className="flex-1 px-4" autoScroll>
            {messages.map((message) => (
              <div key={message.id} className="mt-[16px]">
                {message.role === 'user' ? (
                  <UserMessage message={message} />
                ) : (
                  <GooseMessage
                    message={message}
                    messages={messages}
                    metadata={messageMetadata[message.id]}
                    append={append}
                  />
                )}
              </div>
            ))}
            {error && (
              <div className="flex flex-col items-center justify-center p-4">
                <div className="text-red-700 dark:text-red-300 bg-red-400/50 p-3 rounded-lg mb-2">
                  {error.message || 'Honk! Goose experienced an error while responding'}
                  {error.status && <span className="ml-2">(Status: {error.status})</span>}
                </div>
                <div
                  className="px-3 py-2 mt-2 text-center whitespace-nowrap cursor-pointer text-textStandard border border-borderSubtle hover:bg-bgSubtle rounded-full inline-block transition-all duration-150"
                  onClick={async () => {
                    const lastUserMessage = messages.reduceRight(
                      (found, m) => found || (m.role === 'user' ? m : null),
                      null
                    );
                    if (lastUserMessage) {
                      append({
                        role: 'user',
                        content: lastUserMessage.content,
                      });
                    }
                  }}
                >
                  Retry Last Message
                </div>
              </div>
            )}
            <div className="block h-16" />
          </ScrollArea>
        )}

        <div className="relative">
          {isLoading && <LoadingGoose />}
          <Input
            handleSubmit={handleSubmit}
            disabled={isLoading}
            isLoading={isLoading}
            onStop={onStopGoose}
          />
          <BottomMenu hasMessages={hasMessages} setView={setView} />
        </div>
      </Card>

      {showGame && <FlappyGoose onClose={() => setShowGame(false)} />}
    </div>
  );
}

export default function ChatWindow() {
  // We'll add a state controlling which "view" is active.
  const [view, setView] = useState<View>('welcome');

  // Shared function to create a chat window
  const openNewChatWindow = () => {
    window.electron.createChatWindow();
  };
  const { switchModel } = useModel();
  const { addRecentModel } = useRecentModels();

  // This will store chat data for the "chat" view.
  const [chats, setChats] = useState<Chat[]>(() => [
    {
      id: 1,
      title: 'Chat 1',
      messages: [],
    },
  ]);
  const [selectedChatId, setSelectedChatId] = useState(1);

  // Additional states
  const [mode, setMode] = useState<'expanded' | 'compact'>('expanded');
  const [working, setWorking] = useState<Working>(Working.Idle);
  const [progressMessage, setProgressMessage] = useState<string>('');
  const [initialQuery, setInitialQuery] = useState<string | null>(null);

  // Keyboard shortcut handler
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === 'n') {
        event.preventDefault();
        openNewChatWindow();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  // Attempt to detect config for a stored provider
  useEffect(() => {
    const config = window.electron.getConfig();
    const storedProvider = getStoredProvider(config);
    if (storedProvider) {
      setView('chat');
    } else {
      setView('welcome');
    }
  }, []);

  // Initialize system if we have a stored provider
  useEffect(() => {
    const setupStoredProvider = async () => {
      const config = window.electron.getConfig();

      if (config.GOOSE_PROVIDER && config.GOOSE_MODEL) {
        window.electron.logInfo(
          'Initializing system with environment: GOOSE_MODEL and GOOSE_PROVIDER as priority.'
        );
        await initializeSystem(config.GOOSE_PROVIDER, config.GOOSE_MODEL);
        return;
      }
      const storedProvider = getStoredProvider(config);
      const storedModel = getStoredModel();
      if (storedProvider) {
        try {
          await initializeSystem(storedProvider, storedModel);

          if (!storedModel) {
            const modelName = getDefaultModel(storedProvider.toLowerCase());
            const model = createSelectedModel(storedProvider.toLowerCase(), modelName);
            switchModel(model);
            addRecentModel(model);
          }
        } catch (error) {
          console.error('Failed to initialize with stored provider:', error);
        }
      }
    };

    setupStoredProvider();
  }, []);

  // Render everything inside ChatLayout now
  // We'll switch views inside the ChatLayout children.

  // If we want to skip showing ChatLayout for the welcome screen, we can do so.
  // But let's do exactly what's requested: put all view options under ChatLayout.

  return (
    <ChatLayout mode={mode}>
      {/* Conditionally render based on `view` */}
      {view === 'welcome' && (
        <WelcomeScreen
          onSubmit={() => {
            setView('chat');
          }}
        />
      )}
      {view === 'settings' && (
        <Settings
          onClose={() => {
            setView('chat');
          }}
          setView={setView}
        />
      )}
      {view === 'moreModels' && (
        <MoreModelsSettings
          onClose={() => {
            setView('settings');
          }}
          setView={setView}
        />
      )}
      {view === 'configPage' && (
        <ConfigPage
          onClose={() => {
            setView('chat');
          }}
          setView={setView}
        />
      )}
      {view === 'configureProviders' && (
        <ConfigureProviders
          onClose={() => {
            setView('settings');
          }}
          setView={setView}
        />
      )}
      {view === 'chat' && (
        <ChatContent
          chats={chats}
          setChats={setChats}
          selectedChatId={selectedChatId}
          setSelectedChatId={setSelectedChatId}
          initialQuery={initialQuery}
          setProgressMessage={setProgressMessage}
          setWorking={setWorking}
          setView={setView}
        />
      )}
    </ChatLayout>
  );
}
