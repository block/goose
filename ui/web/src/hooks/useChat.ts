import { useState, useCallback, useRef, useEffect } from 'react'
import { useGoosed } from '../contexts/GoosedContext'
import { ChatMessage, MessageContent } from '../components/Message'

interface UseChatOptions {
    sessionId: string | null
    onSessionCreated?: (sessionId: string) => void
}

interface UseChatReturn {
    messages: ChatMessage[]
    isLoading: boolean
    error: string | null
    sendMessage: (text: string) => Promise<void>
    clearMessages: () => void
    setInitialMessages: (msgs: ChatMessage[]) => void
}

/**
 * Push or update a message in the messages array.
 * This mirrors the desktop's pushMessage logic:
 * - If the incoming message has the same ID as the last message, update it
 * - For text content: accumulate (append) the text
 * - For other content types: push to the content array
 */
function pushMessage(currentMessages: ChatMessage[], incomingMsg: ChatMessage): ChatMessage[] {
    const lastMsg = currentMessages[currentMessages.length - 1]

    // Check if this is an update to the last message (same ID)
    if (lastMsg?.id && lastMsg.id === incomingMsg.id) {
        const lastContent = lastMsg.content[lastMsg.content.length - 1]
        const newContent = incomingMsg.content[incomingMsg.content.length - 1]

        // If both are text and incoming has only one content item, accumulate text
        if (
            lastContent?.type === 'text' &&
            newContent?.type === 'text' &&
            incomingMsg.content.length === 1
        ) {
            // Accumulate text content
            lastContent.text = (lastContent.text || '') + (newContent.text || '')
        } else {
            // Push all incoming content items to the existing message
            lastMsg.content.push(...incomingMsg.content)
        }

        // Return a new array reference to trigger re-render
        return [...currentMessages]
    } else {
        // This is a new message, append it
        return [...currentMessages, incomingMsg]
    }
}

/**
 * Convert backend message format to ChatMessage format
 */
function convertBackendMessage(msg: Record<string, unknown>): ChatMessage {
    const metadata = msg.metadata as { userVisible?: boolean; agentVisible?: boolean } | undefined
    return {
        id: (msg.id as string) || `msg-${Date.now()}-${Math.random()}`,
        role: (msg.role as 'user' | 'assistant') || 'assistant',
        content: (msg.content as MessageContent[]) || [],
        created: (msg.created as number) || Math.floor(Date.now() / 1000),
        metadata: metadata
    }
}

export function useChat({ sessionId }: UseChatOptions): UseChatReturn {
    const { client } = useGoosed()
    const [messages, setMessages] = useState<ChatMessage[]>([])
    const messagesRef = useRef<ChatMessage[]>([])
    const [isLoading, setIsLoading] = useState(false)
    const [error, setError] = useState<string | null>(null)

    // Track mounted state to prevent state updates after unmount
    const isMountedRef = useRef(true)

    useEffect(() => {
        isMountedRef.current = true
        return () => {
            isMountedRef.current = false
        }
    }, [])

    // Keep messagesRef in sync
    const updateMessages = useCallback((newMessages: ChatMessage[]) => {
        if (!isMountedRef.current) return
        messagesRef.current = newMessages
        setMessages(newMessages)
    }, [])

    // Set initial messages from session history
    const setInitialMessages = useCallback((msgs: ChatMessage[]) => {
        updateMessages(msgs)
    }, [updateMessages])

    const sendMessage = useCallback(async (text: string) => {
        if (!sessionId || !text.trim()) return

        setError(null)
        setIsLoading(true)

        // Add user message immediately
        const userMessage: ChatMessage = {
            id: `user-${Date.now()}`,
            role: 'user',
            content: [{ type: 'text', text }],
            created: Math.floor(Date.now() / 1000)
        }

        let currentMessages = [...messagesRef.current, userMessage]
        updateMessages(currentMessages)

        try {
            // Stream the response
            for await (const event of client.sendMessage(sessionId, text)) {
                // Check if component is still mounted before updating state
                if (!isMountedRef.current) break

                if (event.type === 'Message' && event.message) {
                    // Convert incoming message to ChatMessage format
                    const incomingMessage = convertBackendMessage(event.message as Record<string, unknown>)

                    // Use pushMessage logic to properly handle streaming
                    currentMessages = pushMessage(currentMessages, incomingMessage)
                    updateMessages(currentMessages)
                } else if (event.type === 'Error') {
                    if (isMountedRef.current) {
                        setError(event.error || 'Unknown error occurred')
                    }
                }
            }
        } catch (err) {
            if (isMountedRef.current) {
                setError(err instanceof Error ? err.message : 'Failed to send message')
            }
        } finally {
            if (isMountedRef.current) {
                setIsLoading(false)
            }
        }
    }, [client, sessionId, updateMessages])

    const clearMessages = useCallback(() => {
        updateMessages([])
        setError(null)
    }, [updateMessages])

    return {
        messages,
        isLoading,
        error,
        sendMessage,
        clearMessages,
        setInitialMessages
    }
}

// Export the convert function for use in Chat.tsx
export { convertBackendMessage }

