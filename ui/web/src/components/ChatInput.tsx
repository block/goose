import { useState, useRef, useEffect, KeyboardEvent, ChangeEvent } from 'react'

interface ChatInputProps {
    onSubmit: (message: string) => void
    disabled?: boolean
    placeholder?: string
    autoFocus?: boolean
}

export default function ChatInput({
    onSubmit,
    disabled = false,
    placeholder = "Type a message...",
    autoFocus = false
}: ChatInputProps) {
    const [value, setValue] = useState('')
    const textareaRef = useRef<HTMLTextAreaElement>(null)

    // Auto-resize textarea
    useEffect(() => {
        const textarea = textareaRef.current
        if (textarea) {
            textarea.style.height = 'auto'
            textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`
        }
    }, [value])

    // Auto focus
    useEffect(() => {
        if (autoFocus && textareaRef.current) {
            textareaRef.current.focus()
        }
    }, [autoFocus])

    const handleSubmit = () => {
        const trimmedValue = value.trim()
        if (trimmedValue && !disabled) {
            onSubmit(trimmedValue)
            setValue('')
            // Reset textarea height
            if (textareaRef.current) {
                textareaRef.current.style.height = 'auto'
            }
        }
    }

    const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
        if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
            e.preventDefault()
            handleSubmit()
        }
    }

    const handleChange = (e: ChangeEvent<HTMLTextAreaElement>) => {
        setValue(e.target.value)
    }

    return (
        <div className="chat-input-container">
            <div className="chat-input-wrapper">
                <textarea
                    ref={textareaRef}
                    className="chat-input"
                    value={value}
                    onChange={handleChange}
                    onKeyDown={handleKeyDown}
                    placeholder={placeholder}
                    disabled={disabled}
                    rows={1}
                />
                <button
                    className="chat-send-btn"
                    onClick={handleSubmit}
                    disabled={disabled || !value.trim()}
                    aria-label="Send message"
                >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="20" height="20">
                        <line x1="22" y1="2" x2="11" y2="13" />
                        <polygon points="22 2 15 22 11 13 2 9 22 2" />
                    </svg>
                </button>
            </div>
        </div>
    )
}
