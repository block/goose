import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import TextInput from 'ink-text-input';
import Spinner from 'ink-spinner';
import { AcpClient, AcpMessage } from './client.js';

interface SessionNotificationParams {
  sessionId: string;
  update: {
    sessionUpdate: string;
    content?: { type: string; text?: string };
    id?: string;
    title?: string;
    status?: string;
    fields?: { status?: string; content?: unknown[] };
  };
}

interface Message {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

interface AppProps {
  serverUrl: string;
  transportType?: 'http' | 'websocket';
}

export const App: React.FC<AppProps> = ({ serverUrl, transportType = 'http' }) => {
  const { exit } = useApp();
  const [client] = useState(() => new AcpClient({ baseUrl: serverUrl, transport: transportType }));
  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(true);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [currentResponse, setCurrentResponse] = useState('');
  const [currentThought, setCurrentThought] = useState('');
  const [activeTools, setActiveTools] = useState<Map<string, { title: string; status: string }>>(new Map());
  const [error, setError] = useState<string | null>(null);
  
  const currentResponseRef = useRef('');
  const currentThoughtRef = useRef('');
  
  useEffect(() => { currentResponseRef.current = currentResponse; }, [currentResponse]);
  useEffect(() => { currentThoughtRef.current = currentThought; }, [currentThought]);

  useEffect(() => {
    const connectAndInit = async () => {
      try {
        // connect() now handles MCP initialize internally and returns the session ID
        // The server creates the ACP session during initialize, so we're ready to go
        const sid = await client.connect();
        setSessionId(sid);
        setConnected(true);
        setConnecting(false);
        setMessages([{ role: 'system', content: `Connected. Session: ${sid.slice(0, 8)}...` }]);

        client.onMessage((message: AcpMessage) => {
          if (message.method === 'session/update') {
            const params = message.params as SessionNotificationParams;
            const update = params.update;
            const updateType = update.sessionUpdate;

            if (updateType === 'agent_message_chunk' && update.content?.type === 'text') {
              setCurrentResponse(prev => prev + (update.content?.text || ''));
            }
            if (updateType === 'agent_thought_chunk' && update.content?.type === 'text') {
              setCurrentThought(prev => prev + (update.content?.text || ''));
            }
            if (updateType === 'tool_call' && update.id) {
              setActiveTools(prev => {
                const next = new Map(prev);
                next.set(update.id!, { title: update.title || 'Tool', status: update.status || 'pending' });
                return next;
              });
            }
            if (updateType === 'tool_call_update' && update.id && update.fields?.status) {
              setActiveTools(prev => {
                const next = new Map(prev);
                const existing = next.get(update.id!);
                if (existing) {
                  next.set(update.id!, { ...existing, status: update.fields!.status! });
                }
                return next;
              });
            }
          }
        });

        client.onError((err) => setError(err.message));
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Connection failed');
        setConnecting(false);
      }
    };

    connectAndInit();
    return () => { client.disconnect(); };
  }, [client]);

  const handleSubmit = useCallback(async (value: string) => {
    if (!value.trim() || isProcessing || !sessionId) return;

    const userMessage = value.trim();
    setInput('');
    setMessages(prev => [...prev, { role: 'user', content: userMessage }]);
    setIsProcessing(true);
    setCurrentResponse('');
    setCurrentThought('');
    currentResponseRef.current = '';
    currentThoughtRef.current = '';
    setActiveTools(new Map());

    try {
      await client.sendRequest('session/prompt', {
        sessionId: sessionId,
        prompt: [{ type: 'text', text: userMessage }],
      });

      const finalResponse = currentResponseRef.current;
      const finalThought = currentThoughtRef.current;
      
      if (finalResponse || finalThought) {
        setMessages(prev => [
          ...prev,
          ...(finalThought ? [{ role: 'assistant' as const, content: `💭 ${finalThought}` }] : []),
          ...(finalResponse ? [{ role: 'assistant' as const, content: finalResponse }] : []),
        ]);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to send message');
    } finally {
      setIsProcessing(false);
      setCurrentResponse('');
      setCurrentThought('');
    }
  }, [client, sessionId, isProcessing]);

  useInput((inputChar, key) => {
    if (key.ctrl && inputChar === 'c') {
      client.disconnect();
      exit();
    }
  });

  if (connecting) {
    return (
      <Box flexDirection="column" padding={1}>
        <Text><Spinner type="dots" /> Connecting to {serverUrl}...</Text>
      </Box>
    );
  }

  if (error) {
    return (
      <Box flexDirection="column" padding={1}>
        <Text color="red">Error: {error}</Text>
        <Text dimColor>Press Ctrl+C to exit</Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="column" padding={1}>
      <Box borderStyle="single" borderColor="cyan" paddingX={1} marginBottom={1}>
        <Text bold color="cyan">🪿 Goose ACP TUI</Text>
        <Text dimColor> | Session: {sessionId?.slice(0, 8)}...</Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        {messages.map((msg, i) => (
          <Box key={i} marginBottom={1}>
            <Text>
              {msg.role === 'user' && <Text color="green" bold>You: </Text>}
              {msg.role === 'assistant' && <Text color="blue" bold>Goose: </Text>}
              {msg.role === 'system' && <Text color="yellow" bold>System: </Text>}
              <Text>{msg.content}</Text>
            </Text>
          </Box>
        ))}
      </Box>

      {activeTools.size > 0 && (
        <Box flexDirection="column" marginBottom={1}>
          {Array.from(activeTools.entries()).map(([id, tool]) => (
            <Box key={id}>
              <Text color="magenta">
                {tool.status === 'pending' && <Spinner type="dots" />}
                {tool.status === 'completed' && '✓'}
                {tool.status === 'failed' && '✗'}
                {' '}{tool.title}
              </Text>
            </Box>
          ))}
        </Box>
      )}

      {currentThought && (
        <Box marginBottom={1}>
          <Text color="gray" italic>💭 {currentThought}</Text>
        </Box>
      )}

      {currentResponse && (
        <Box marginBottom={1}>
          <Text color="blue" bold>Goose: </Text>
          <Text>{currentResponse}</Text>
        </Box>
      )}

      {isProcessing && !currentResponse && !currentThought && activeTools.size === 0 && (
        <Box marginBottom={1}>
          <Text color="blue"><Spinner type="dots" /> Thinking...</Text>
        </Box>
      )}

      <Box>
        <Text color="green" bold>{'> '}</Text>
        <TextInput
          value={input}
          onChange={setInput}
          onSubmit={handleSubmit}
          placeholder={isProcessing ? 'Processing...' : 'Type a message...'}
        />
      </Box>

      <Box marginTop={1}>
        <Text dimColor>Press Ctrl+C to exit</Text>
      </Box>
    </Box>
  );
};
