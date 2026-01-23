import { useRef, useEffect, useMemo } from 'react'
import Message, { ChatMessage } from './Message'
import type { ToolResponseMap } from './Message'

interface MessageListProps {
    messages: ChatMessage[]
    isLoading?: boolean
}

export default function MessageList({ messages, isLoading = false }: MessageListProps) {
    const containerRef = useRef<HTMLDivElement>(null)
    const bottomRef = useRef<HTMLDivElement>(null)

    // Auto-scroll to bottom when new messages arrive
    useEffect(() => {
        if (bottomRef.current) {
            bottomRef.current.scrollIntoView({ behavior: 'smooth' })
        }
    }, [messages])

    // Filter messages based on metadata.userVisible
    // Only show messages that are visible to the user
    const visibleMessages = messages.filter(msg => {
        // If no metadata, default to visible
        if (!msg.metadata) return true
        // Check userVisible flag
        return msg.metadata.userVisible !== false
    })

    if (visibleMessages.length === 0 && !isLoading) {
        return (
            <div className="empty-state">
                <svg
                    className="empty-state-icon"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.5"
                >
                    <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
                </svg>
                <h3 className="empty-state-title">No messages yet</h3>
                <p className="empty-state-description">
                    Start a conversation by typing a message below.
                </p>
            </div>
        )
    }

    const toolResponses = useMemo<ToolResponseMap>(() => {
        const map: ToolResponseMap = new Map()
        for (const msg of visibleMessages) {
            for (const content of msg.content) {
                if (content.type === 'toolResponse' && content.id) {
                    const toolResult = content.toolResult
                    map.set(content.id, {
                        result: toolResult?.status === 'success' ? toolResult.value : toolResult,
                        isError: toolResult?.status === 'error'
                    })
                }
            }
        }
        return map
    }, [visibleMessages])

    return (
        <div className="chat-messages" ref={containerRef}>
            {visibleMessages.map((message, index) => (
                <Message
                    key={message.id || index}
                    message={message}
                    toolResponses={toolResponses}
                />
            ))}

            {isLoading && (
                <div className="message assistant animate-fade-in">
                    <div className="message-avatar">G</div>
                    <div className="message-content">
                        <div className="loading-dots">
                            <span></span>
                            <span></span>
                            <span></span>
                        </div>
                    </div>
                </div>
            )}

            <div ref={bottomRef} />
        </div>
    )
}
