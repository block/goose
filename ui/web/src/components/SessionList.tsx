import type { Session } from '@goosed/sdk'
import SessionItem from './SessionItem'

interface SessionListProps {
    sessions: Session[]
    isLoading?: boolean
    onResume: (sessionId: string) => void
    onDelete: (sessionId: string) => void
}

export default function SessionList({
    sessions,
    isLoading = false,
    onResume,
    onDelete
}: SessionListProps) {
    if (isLoading) {
        return (
            <div className="session-list">
                {[1, 2, 3].map((i) => (
                    <div key={i} className="session-item" style={{ opacity: 0.5 }}>
                        <div className="session-info">
                            <div
                                className="session-name"
                                style={{
                                    width: '60%',
                                    height: '20px',
                                    background: 'var(--color-bg-tertiary)',
                                    borderRadius: 'var(--radius-sm)'
                                }}
                            />
                            <div
                                className="session-meta"
                                style={{
                                    width: '40%',
                                    height: '14px',
                                    marginTop: '8px',
                                    background: 'var(--color-bg-tertiary)',
                                    borderRadius: 'var(--radius-sm)'
                                }}
                            />
                        </div>
                    </div>
                ))}
            </div>
        )
    }

    if (sessions.length === 0) {
        return (
            <div className="empty-state">
                <svg
                    className="empty-state-icon"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.5"
                >
                    <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                    <line x1="9" y1="9" x2="15" y2="15" />
                    <line x1="15" y1="9" x2="9" y2="15" />
                </svg>
                <h3 className="empty-state-title">No sessions yet</h3>
                <p className="empty-state-description">
                    Start a new chat to create your first session.
                </p>
            </div>
        )
    }

    return (
        <div className="session-list">
            {sessions.map((session) => (
                <SessionItem
                    key={session.id}
                    session={session}
                    onResume={onResume}
                    onDelete={onDelete}
                />
            ))}
        </div>
    )
}
