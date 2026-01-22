import { useState, useEffect, useMemo } from 'react'
import { useNavigate } from 'react-router-dom'
import { useGoosed } from '../contexts/GoosedContext'
import SessionList from '../components/SessionList'
import type { Session } from '@goosed/sdk'

export default function History() {
    const navigate = useNavigate()
    const { client, isConnected } = useGoosed()
    const [sessions, setSessions] = useState<Session[]>([])
    const [isLoading, setIsLoading] = useState(true)
    const [searchTerm, setSearchTerm] = useState('')
    const [error, setError] = useState<string | null>(null)

    // Load all sessions
    useEffect(() => {
        const loadSessions = async () => {
            if (!isConnected) return

            setIsLoading(true)
            setError(null)

            try {
                const allSessions = await client.listSessions()
                // Sort by updated_at descending
                const sorted = allSessions.sort((a, b) =>
                    new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
                )
                setSessions(sorted)
            } catch (err) {
                console.error('Failed to load sessions:', err)
                setError(err instanceof Error ? err.message : 'Failed to load sessions')
            } finally {
                setIsLoading(false)
            }
        }

        loadSessions()
    }, [client, isConnected])

    // Filter sessions by search term
    const filteredSessions = useMemo(() => {
        if (!searchTerm.trim()) return sessions

        const term = searchTerm.toLowerCase()
        return sessions.filter(session =>
            session.name.toLowerCase().includes(term) ||
            session.working_dir.toLowerCase().includes(term)
        )
    }, [sessions, searchTerm])

    const handleResumeSession = (sessionId: string) => {
        navigate(`/chat?sessionId=${sessionId}`)
    }

    const handleDeleteSession = async (sessionId: string) => {
        try {
            await client.deleteSession(sessionId)
            setSessions(prev => prev.filter(s => s.id !== sessionId))
        } catch (err) {
            console.error('Failed to delete session:', err)
            alert('Failed to delete session: ' + (err instanceof Error ? err.message : 'Unknown error'))
        }
    }

    return (
        <div className="page-container">
            <header className="page-header">
                <h1 className="page-title">Chat History</h1>
                <p className="page-subtitle">
                    View and manage your previous chat sessions
                </p>
            </header>

            <div className="search-container">
                <div className="search-input-wrapper">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <circle cx="11" cy="11" r="8" />
                        <line x1="21" y1="21" x2="16.65" y2="16.65" />
                    </svg>
                    <input
                        type="text"
                        className="search-input"
                        placeholder="Search sessions..."
                        value={searchTerm}
                        onChange={(e) => setSearchTerm(e.target.value)}
                    />
                    {searchTerm && (
                        <button
                            onClick={() => setSearchTerm('')}
                            style={{
                                background: 'none',
                                border: 'none',
                                color: 'var(--color-text-muted)',
                                cursor: 'pointer',
                                padding: 'var(--spacing-1)'
                            }}
                            aria-label="Clear search"
                        >
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16">
                                <line x1="18" y1="6" x2="6" y2="18" />
                                <line x1="6" y1="6" x2="18" y2="18" />
                            </svg>
                        </button>
                    )}
                </div>
            </div>

            {error && (
                <div style={{
                    padding: 'var(--spacing-4)',
                    background: 'rgba(239, 68, 68, 0.2)',
                    borderRadius: 'var(--radius-lg)',
                    color: 'var(--color-error)',
                    marginBottom: 'var(--spacing-6)'
                }}>
                    ⚠️ {error}
                </div>
            )}

            {searchTerm && filteredSessions.length === 0 && !isLoading && (
                <div className="empty-state">
                    <svg
                        className="empty-state-icon"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="1.5"
                    >
                        <circle cx="11" cy="11" r="8" />
                        <line x1="21" y1="21" x2="16.65" y2="16.65" />
                    </svg>
                    <h3 className="empty-state-title">No results found</h3>
                    <p className="empty-state-description">
                        No sessions match "{searchTerm}"
                    </p>
                </div>
            )}

            {(!searchTerm || filteredSessions.length > 0) && (
                <>
                    {searchTerm && (
                        <p style={{
                            fontSize: 'var(--font-size-sm)',
                            color: 'var(--color-text-secondary)',
                            marginBottom: 'var(--spacing-4)'
                        }}>
                            {filteredSessions.length} result{filteredSessions.length !== 1 ? 's' : ''} found
                        </p>
                    )}

                    <SessionList
                        sessions={filteredSessions}
                        isLoading={isLoading}
                        onResume={handleResumeSession}
                        onDelete={handleDeleteSession}
                    />
                </>
            )}

            {!isLoading && sessions.length > 0 && (
                <p style={{
                    marginTop: 'var(--spacing-6)',
                    fontSize: 'var(--font-size-sm)',
                    color: 'var(--color-text-muted)',
                    textAlign: 'center'
                }}>
                    {sessions.length} total session{sessions.length !== 1 ? 's' : ''}
                </p>
            )}
        </div>
    )
}
