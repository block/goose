import type { MouseEvent } from 'react'
import type { Session } from '@goosed/sdk'

interface SessionItemProps {
    session: Session
    onResume: (sessionId: string) => void
    onDelete: (sessionId: string) => void
}

export default function SessionItem({ session, onResume, onDelete }: SessionItemProps) {
    const formattedDate = new Date(session.updated_at || session.created_at).toLocaleDateString('en-US', {
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
    })

    const handleDeleteClick = (e: MouseEvent) => {
        e.preventDefault()
        e.stopPropagation()
        onDelete(session.id)
    }

    return (
        <div className="session-item animate-slide-in">
            <div className="session-info" onClick={() => onResume(session.id)} style={{ cursor: 'pointer', flex: 1 }}>
                <div className="session-name">{session.name || 'Untitled Session'}</div>
                <div className="session-meta">
                    <span>{formattedDate}</span>
                    {session.message_count !== undefined && (
                        <span>{session.message_count} messages</span>
                    )}
                    {session.total_tokens !== undefined && session.total_tokens !== null && (
                        <span>{session.total_tokens.toLocaleString()} tokens</span>
                    )}
                </div>
            </div>

            <div className="session-actions">
                <button
                    type="button"
                    className="session-action-btn delete"
                    onClick={handleDeleteClick}
                    title="Delete"
                    aria-label="Delete session"
                >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16">
                        <polyline points="3 6 5 6 21 6" />
                        <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                    </svg>
                </button>
            </div>
        </div>
    )
}
