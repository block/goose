import React, { useEffect, useRef, useState } from 'react';
import { useChat } from 'ai/react';
import { Route, Routes, Navigate } from 'react-router-dom';
import { getApiUrl } from './config';
import { Card } from './components/ui/card';
import { ScrollArea } from './components/ui/scroll-area';
import Splash from './components/Splash';
import GooseMessage from './components/GooseMessage';
import UserMessage from './components/UserMessage';
import Input from './components/Input';
import Tabs from './components/Tabs';
import MoreMenu from './components/MoreMenu';
import { BoxIcon } from './components/ui/icons';
import ReactMarkdown from 'react-markdown';

export interface Chat {
  id: number;
  title: string;
  messages: Array<{
    id: string;
    role: 'function' | 'system' | 'user' | 'assistant' | 'data' | 'tool';
    content: string;
  }>;
}

const handleResize = (mode: 'expanded' | 'compact') => {
  if (window.electron) {
    if (mode === 'expanded') {
      const width = window.innerWidth;
      const height = window.innerHeight;
      window.electron.resizeWindow(width, height);
    } else if (mode === 'compact') {
      const width = window.innerWidth;
      const height = 100; // Make it very thin
      window.electron.resizeWindow(width, height);
    }
  }
};

const WingView: React.FC<{ onExpand: () => void; status: string }> = ({ onExpand, status }) => {
  return (
    <div
      onClick={onExpand}
      className="flex items-center justify-center w-full bg-gray-800 text-green-400 cursor-pointer rounded-lg p-4"
    >
      <div className="text-sm text-left font-mono bg-black bg-opacity-50 p-3 rounded-lg">
        <span className="block">{status}</span>
      </div>
    </div>
  );
};

function ChatContent({
  chats,
  setChats,
  selectedChatId,
  setSelectedChatId,
  initialQuery,
  setStatus,
}: {
  chats: Chat[];
  setChats: React.Dispatch<React.SetStateAction<Chat[]>>;
  selectedChatId: number;
  setSelectedChatId: React.Dispatch<React.SetStateAction<number>>;
  initialQuery: string | null;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
}) {
  const chat = chats.find((c: Chat) => c.id === selectedChatId);

  //window.electron.logInfo('chats' + JSON.stringify(chats, null, 2));

  const [messageMetadata, setMessageMetadata] = useState<Record<string, string[]>>({});

  const {
    messages,
    input,
    handleInputChange,
    handleSubmit,
    append,
    stop,
    isLoading,
    error
  } = useChat({
    api: getApiUrl('/reply'),
    initialMessages: chat?.messages || [],
    onToolCall: ({ toolCall }) => {
      setStatus(`Executing tool: ${toolCall.toolName}`);
      // Optionally handle tool call result here
    },
    onResponse: (response) => {
      if (!response.ok) {
        setStatus('An error occurred while receiving the response.');
      } else {
        setStatus('Receiving response...');
      }
    },
    onFinish: async (message, options) => {
      setStatus('Goose is ready');

      const promptTemplates = [
        "Take a look at this content, if this looks like it could be asking for a confirmation, return QUESTION. If it looks like it is a list of options or plans to choose from, return OPTIONS. It can't just be a list, but clearly must be asking the user to pick one or more of the plan or option alternatives, otherwise return READY. \n ### Message Content:\n" + message.content,
        "If the content is clearly a list of distinct options or plans of action to choose from, and not just a list of things, but clearly a list of things to choose one from from, taking into account the Message Content alone, try to format it in a json array, like this JSON array of objects of the form optionTitle:string, optionDescription:string (markdown).\n If is not a list of options or plans to choose from, then return empty list.\n ### Message Content:\n" + message.content,
      ];

      const fetchResponses = await askAi(promptTemplates);

      setMessageMetadata(prev => ({ ...prev, [message.id]: fetchResponses }));

      console.log('All responses:', fetchResponses);
    },
  });

  // Update chat messages when they change
  useEffect(() => {
    const updatedChats = chats.map((c) =>
      c.id === selectedChatId ? { ...c, messages } : c
    );
    setChats(updatedChats);
  }, [messages, selectedChatId]);

  const initialQueryAppended = useRef(false);
  useEffect(() => {
    if (initialQuery && !initialQueryAppended.current) {
      append({ role: 'user', content: initialQuery });
      initialQueryAppended.current = true;
    }
  }, [initialQuery]);

  if (error) {
    console.log('Error:', error);
  }

  return (
    <div className="chat-content flex flex-col w-screen h-screen bg-window-gradient items-center justify-center p-[10px]">
      <div className="flex w-screen">
        <div className="flex-1">
          <Tabs
            chats={chats}
            selectedChatId={selectedChatId}
            setSelectedChatId={setSelectedChatId}
            setChats={setChats}
          />
        </div>
        <div className="flex">
          <MoreMenu
            className="absolute top-2 right-2"
            onStopGoose={() => {
              stop();
            }}
            onClearContext={() => {
              append({
                id: Date.now().toString(),
                role: 'system',
                content: 'Context cleared',
              });
            }}
            onRestartGoose={() => {
              append({
                id: Date.now().toString(),
                role: 'system',
                content: 'Goose restarted',
              });
            }}
          />
        </div>
      </div>
      <Card className="flex flex-col flex-1 h-[calc(100vh-95px)] w-full bg-card-gradient mt-0 border-none shadow-xl rounded-2xl relative">
        {messages.length === 0 ? (
          <Splash append={append} />
        ) : (
          <ScrollArea className="flex-1 px-[10px]">
            <div className="block h-10" />
            {messages.map((message) => (
              <div key={message.id}>
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
            {isLoading && (
              <div className="flex items-center justify-center p-4">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900" />
              </div>
            )}
            {error && (
              <div className="flex items-center justify-center p-4">
                <div className="text-red-500 bg-red-100 p-3 rounded-lg">
                  {error.message|| 'An error occurred while processing your request'}
                </div>
              </div>
            )}
            <div className="block h-10" />
          </ScrollArea>
        )}

        <Input
          handleSubmit={handleSubmit}
          handleInputChange={handleInputChange}
          input={input}
          disabled={isLoading}
        />
      </Card>
    </div>
  );
}

export default function ChatWindow() {
  // Get initial query and history from URL parameters
  const searchParams = new URLSearchParams(window.location.search);
  const initialQuery = searchParams.get('initialQuery');
  //window.electron.logInfo('initialQuery: ' + initialQuery);
  const historyParam = searchParams.get('history');
  const initialHistory = historyParam
    ? JSON.parse(decodeURIComponent(historyParam))
    : [];

  const [chats, setChats] = useState<Chat[]>(() => {
    const firstChat = {
      id: 1,
      title: initialQuery || 'Chat 1',
      messages: initialHistory.length > 0 ? initialHistory : [],
    };
    return [firstChat];
  });

  const [selectedChatId, setSelectedChatId] = useState(1);

  const [mode, setMode] = useState<'expanded' | 'compact'>(
    initialQuery ? 'compact' : 'expanded'
  );

  const [status, setStatus] = useState('Goose is ready');

  const toggleMode = () => {
    const newMode = mode === 'expanded' ? 'compact' : 'expanded';
    console.log(`Toggle to ${newMode}`);
    setMode(newMode);
    handleResize(newMode);
  };

  window.electron.logInfo('ChatWindow loaded');

  return (
    <div className="relative w-screen h-screen overflow-hidden bg-transparent flex flex-col">
      {/* Always render ChatContent but control its visibility */}
      <div style={{ display: mode === 'expanded' ? 'block' : 'none' }}>
        <Routes>
          <Route
            path="/chat/:id"
            element={
              <ChatContent
                key={selectedChatId}
                chats={chats}
                setChats={setChats}
                selectedChatId={selectedChatId}
                setSelectedChatId={setSelectedChatId}
                initialQuery={initialQuery}
                setStatus={setStatus} // Pass setStatus to ChatContent
              />
            }
          />
          <Route path="*" element={<Navigate to="/chat/1" replace />} />
        </Routes>
      </div>

      {/* Always render WingView but control its visibility */}
      <div style={{ display: mode === 'expanded' ? 'none' : 'flex' }}>
        <WingView onExpand={toggleMode} status={status} />
      </div>
    </div>
  );
}

/**
 * Utillity to ask the LLM any question to clarify without wider context.
 */
async function askAi(promptTemplates: string[]) {
  console.log('askAi called...');
  const responses = await Promise.all(promptTemplates.map(async (template) => {
    const response = await fetch(getApiUrl('/ask'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ prompt: template })
    });

    if (!response.ok) {
      throw new Error('Failed to get response');
    }

    const data = await response.json();
    console.log('ask Response:', data.response);

    return data.response;
  }));

  return responses;
}
