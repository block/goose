import { useState, type FormEvent, type MouseEvent } from 'react'

interface NewSessionModalProps {
    isOpen: boolean
    onClose: () => void
    onSubmit: (workingDir: string, initialMessage?: string) => void
    showInitialMessage?: boolean
}

export default function NewSessionModal({
    isOpen,
    onClose,
    onSubmit,
    showInitialMessage = false
}: NewSessionModalProps) {
    // Default to user home directory using ~
    const defaultWorkingDir = '~'

    const [workingDir, setWorkingDir] = useState(defaultWorkingDir)
    const [initialMessage, setInitialMessage] = useState('')

    if (!isOpen) return null

    const handleSubmit = (e: FormEvent) => {
        e.preventDefault()
        if (workingDir.trim()) {
            onSubmit(workingDir.trim(), showInitialMessage ? initialMessage.trim() : undefined)
            // Reset form
            setWorkingDir(defaultWorkingDir)
            setInitialMessage('')
        }
    }

    const handleOverlayClick = (e: MouseEvent) => {
        if (e.target === e.currentTarget) {
            onClose()
        }
    }

    return (
        <div className="modal-overlay" onClick={handleOverlayClick}>
            <div className="modal animate-slide-up">
                <div className="modal-header">
                    <h2 className="modal-title">New Chat Session</h2>
                </div>

                <form onSubmit={handleSubmit}>
                    <div className="modal-body">
                        <div className="form-group">
                            <label className="form-label" htmlFor="workingDir">
                                Working Directory
                            </label>
                            <input
                                id="workingDir"
                                type="text"
                                className="form-input"
                                value={workingDir}
                                onChange={(e) => setWorkingDir(e.target.value)}
                                placeholder="/path/to/directory"
                                autoFocus
                            />
                            <p style={{
                                fontSize: 'var(--font-size-xs)',
                                color: 'var(--color-text-muted)',
                                marginTop: 'var(--spacing-2)'
                            }}>
                                The directory where Goose will operate. This affects file operations and tool access.
                            </p>
                        </div>

                        {showInitialMessage && (
                            <div className="form-group">
                                <label className="form-label" htmlFor="initialMessage">
                                    Initial Message (optional)
                                </label>
                                <textarea
                                    id="initialMessage"
                                    className="form-input"
                                    value={initialMessage}
                                    onChange={(e) => setInitialMessage(e.target.value)}
                                    placeholder="What would you like to start with?"
                                    rows={3}
                                    style={{ resize: 'vertical', minHeight: '80px' }}
                                />
                            </div>
                        )}
                    </div>

                    <div className="modal-footer">
                        <button type="button" className="btn btn-secondary" onClick={onClose}>
                            Cancel
                        </button>
                        <button
                            type="submit"
                            className="btn btn-primary"
                            disabled={!workingDir.trim()}
                        >
                            Start Chat
                        </button>
                    </div>
                </form>
            </div>
        </div>
    )
}
