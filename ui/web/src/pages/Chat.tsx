import { useState, useEffect, useCallback } from 'react'
import { useSearchParams, useLocation, useNavigate } from 'react-router-dom'
import { useGoosed } from '../contexts/GoosedContext'
import { useChat, convertBackendMessage } from '../hooks/useChat'
import MessageList from '../components/MessageList'
import ChatInput from '../components/ChatInput'
import NewSessionModal from '../components/NewSessionModal'
import type { Session } from '@goosed/sdk'

interface LocationState {
    initialMessage?: string
}

export default function Chat() {
    const [searchParams] = useSearchParams()
    const location = useLocation()
    const navigate = useNavigate()
    const { client, isConnected } = useGoosed()

    const sessionId = searchParams.get('sessionId')
    const [session, setSession] = useState<Session | null>(null)
    const [isInitializing, setIsInitializing] = useState(true)
    const [showNewSessionModal, setShowNewSessionModal] = useState(false)
    const [initError, setInitError] = useState<string | null>(null)

    const { messages, isLoading, error, sendMessage, clearMessages, setInitialMessages } = useChat({
        sessionId
    })

    // Get initial message from navigation state
    const locationState = location.state as LocationState | null
    const initialMessage = locationState?.initialMessage

    // Initialize session
    useEffect(() => {
        const initSession = async () => {
            if (!isConnected) return

            if (!sessionId) {
                // No session ID, show modal to create new session
                setShowNewSessionModal(true)
                setIsInitializing(false)
                return
            }

            setIsInitializing(true)
            setInitError(null)

            try {
                // Get session details
                const sessionDetails = await client.getSession(sessionId)
                setSession(sessionDetails)

                // Resume session to load model and extensions
                await client.resumeSession(sessionId)

                // Load existing messages from session conversation
                if (sessionDetails.conversation && Array.isArray(sessionDetails.conversation)) {
                    const historyMessages = sessionDetails.conversation.map(msg =>
                        convertBackendMessage(msg as Record<string, unknown>)
                    )
                    setInitialMessages(historyMessages)
                }
            } catch (err) {
                console.error('Failed to initialize session:', err)
                setInitError(err instanceof Error ? err.message : 'Failed to load session')
            } finally {
                setIsInitializing(false)
            }
        }

        initSession()
    }, [client, isConnected, sessionId, setInitialMessages])

    // Send initial message if provided
    useEffect(() => {
        if (initialMessage && sessionId && !isInitializing && messages.length === 0) {
            sendMessage(initialMessage)
            // Clear the state so it doesn't resend on refresh
            window.history.replaceState({}, document.title)
        }
    }, [initialMessage, sessionId, isInitializing, messages.length, sendMessage])

    const handleCreateSession = async (workingDir: string) => {
        try {
            const newSession = await client.startSession(workingDir)
            await client.resumeSession(newSession.id)

            setSession(newSession)
            setShowNewSessionModal(false)
            clearMessages()

            // Update URL with new session ID
            navigate(`/chat?sessionId=${newSession.id}`, { replace: true })
        } catch (err) {
            console.error('Failed to create session:', err)
            alert('Failed to create session: ' + (err instanceof Error ? err.message : 'Unknown error'))
        }
    }

    const handleSendMessage = useCallback((text: string) => {
        sendMessage(text)
    }, [sendMessage])



    if (showNewSessionModal) {
        return (
            <div className="chat-container">
                <NewSessionModal
                    isOpen={true}
                    onClose={() => {
                        if (sessionId) {
                            setShowNewSessionModal(false)
                        } else {
                            navigate('/')
                        }
                    }}
                    onSubmit={handleCreateSession}
                    showInitialMessage={true}
                />
            </div>
        )
    }

    if (isInitializing) {
        return (
            <div className="chat-container">
                <div style={{
                    flex: 1,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center'
                }}>
                    <div style={{ textAlign: 'center' }}>
                        <div className="loading-spinner" style={{ margin: '0 auto var(--spacing-4)' }} />
                        <p style={{ color: 'var(--color-text-secondary)' }}>Loading session...</p>
                    </div>
                </div>
            </div>
        )
    }

    if (initError) {
        return (
            <div className="chat-container">
                <div style={{
                    flex: 1,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center'
                }}>
                    <div className="empty-state">
                        <svg
                            className="empty-state-icon"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="1.5"
                        >
                            <circle cx="12" cy="12" r="10" />
                            <line x1="12" y1="8" x2="12" y2="12" />
                            <line x1="12" y1="16" x2="12.01" y2="16" />
                        </svg>
                        <h3 className="empty-state-title">Failed to load session</h3>
                        <p className="empty-state-description">{initError}</p>
                        <button
                            className="btn btn-primary"
                            style={{ marginTop: 'var(--spacing-4)' }}
                            onClick={() => navigate('/')}
                        >
                            Back to Home
                        </button>
                    </div>
                </div>
            </div>
        )
    }

    return (
        <div className="chat-container">
            <header className="chat-header">
                <div>
                    <h1 className="chat-title">{session?.name || 'Chat'}</h1>
                    {session?.working_dir && (
                        <p style={{
                            fontSize: 'var(--font-size-xs)',
                            color: 'var(--color-text-muted)',
                            marginTop: 'var(--spacing-1)'
                        }}>
                            📁 {session.working_dir}
                        </p>
                    )}
                </div>
            </header>

            <div className="chat-content">
                <MessageList messages={messages} isLoading={isLoading} />

                {error && (
                    <div style={{
                        padding: 'var(--spacing-3) var(--spacing-6)',
                        background: 'rgba(239, 68, 68, 0.1)',
                        borderTop: '1px solid rgba(239, 68, 68, 0.3)',
                        color: 'var(--color-error)',
                        fontSize: 'var(--font-size-sm)'
                    }}>
                        ⚠️ {error}
                    </div>
                )}

                <div className="chat-input-area">
                    <ChatInput
                        onSubmit={handleSendMessage}
                        disabled={isLoading || !isConnected}
                        placeholder={isLoading ? "Waiting for response..." : "Type a message..."}
                        autoFocus
                    />
                </div>
            </div>
        </div>
    )
}
