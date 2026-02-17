import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import TextInput from 'ink-text-input';
import Spinner from 'ink-spinner';
import { SdkAcpClient } from './acp-client.js';
import type { SessionNotification, TextContent, ToolCall, ToolCallUpdate } from '@agentclientprotocol/sdk';

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
  const clientRef = useRef<SdkAcpClient | null>(null);
  
  useEffect(() => { currentResponseRef.current = currentResponse; }, [currentResponse]);
  useEffect(() => { currentThoughtRef.current = currentThought; }, [currentThought]);

  // Handle session updates from the SDK
  const handleSessionUpdate = useCallback((notification: SessionNotification) => {
    const update = notification.update;
    const updateType = update.sessionUpdate;

    if (updateType === 'agent_message_chunk' && update.content?.type === 'text') {
      setCurrentResponse(prev => prev + ((update.content as TextContent).text || ''));
    }
    if (updateType === 'agent_thought_chunk' && update.content?.type === 'text') {
      setCurrentThought(prev => prev + ((update.content as TextContent).text || ''));
    }
    if (updateType === 'tool_call') {
      const toolCall = update as ToolCall & { sessionUpdate: 'tool_call' };
      if (toolCall.toolCallId) {
        setActiveTools(prev => {
          const next = new Map(prev);
          next.set(toolCall.toolCallId, { title: toolCall.title || 'Tool', status: toolCall.status || 'pending' });
          return next;
        });
      }
    }
    if (updateType === 'tool_call_update') {
      const toolUpdate = update as ToolCallUpdate & { sessionUpdate: 'tool_call_update' };
      if (toolUpdate.toolCallId && toolUpdate.status) {
        setActiveTools(prev => {
          const next = new Map(prev);
          const existing = next.get(toolUpdate.toolCallId);
          if (existing) {
            next.set(toolUpdate.toolCallId, { ...existing, status: toolUpdate.status! });
          }
          return next;
        });
      }
    }
  }, []);

  useEffect(() => {
    const connectAndInit = async () => {
      try {
        // Create the SDK-based client with session update handler
        const client = new SdkAcpClient(
          { serverUrl },
          { onSessionUpdate: handleSessionUpdate }
        );
        clientRef.current = client;

        // Connect and initialize session
        const sid = await client.connect();
        setSessionId(sid);
        setConnected(true);
        setConnecting(false);
        setMessages([{ role: 'system', content: `Connected. Session: ${sid.slice(0, 8)}...` }]);
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Connection failed');
        setConnecting(false);
      }
    };

    connectAndInit();
    return () => { 
      if (clientRef.current) {
        clientRef.current.disconnect();
      }
    };
  }, [serverUrl, handleSessionUpdate]);

  const handleSubmit = useCallback(async (value: string) => {
    if (!value.trim() || isProcessing || !sessionId || !clientRef.current) return;

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
      // Use the SDK client's prompt method
      await clientRef.current.prompt(userMessage);

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
  }, [sessionId, isProcessing]);

  useInput((inputChar, key) => {
    if (key.ctrl && inputChar === 'c') {
      if (clientRef.current) {
        clientRef.current.disconnect();
      }
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
