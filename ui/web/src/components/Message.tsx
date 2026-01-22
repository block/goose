import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import ToolCallDisplay from './ToolCallDisplay'

export interface MessageContent {
    type: string
    text?: string
    id?: string
    name?: string
    input?: Record<string, unknown>
    // For toolRequest - contains the tool call details
    toolCall?: {
        status?: string
        value?: {
            name?: string
            arguments?: Record<string, unknown>
        }
    }
    // For toolResponse - contains the tool result
    toolResult?: {
        status?: string
        value?: unknown
    }
}

export interface MessageMetadata {
    userVisible?: boolean
    agentVisible?: boolean
}

export interface ChatMessage {
    id?: string
    role: 'user' | 'assistant'
    content: MessageContent[]
    created?: number
    metadata?: MessageMetadata
}

interface MessageProps {
    message: ChatMessage
    toolResponses?: ToolResponseMap
}

export type ToolResponseMap = Map<string, { result?: unknown; isError: boolean }>

// Represents a paired tool call with its request and response
interface ToolCallPair {
    id: string
    name: string
    args?: Record<string, unknown>
    result?: unknown
    isPending: boolean
    isError: boolean
}

export default function Message({ message, toolResponses = new Map() }: MessageProps) {
    const isUser = message.role === 'user'

    // Extract text content and tool calls
    const textContent: string[] = []
    const toolRequests: Map<string, { name: string; args?: Record<string, unknown>; status?: string }> = new Map()

    // Collect content from current message
    for (const content of message.content) {
        if (content.type === 'text' && content.text) {
            textContent.push(content.text)
        } else if (content.type === 'toolRequest' && content.id) {
            // toolRequest contains toolCall.value.name and toolCall.value.arguments
            const toolCall = content.toolCall
            toolRequests.set(content.id, {
                name: toolCall?.value?.name || 'unknown',
                args: toolCall?.value?.arguments,
                status: toolCall?.status
            })
        } else if (content.type === 'toolResponse' && content.id) {
            // Also collect from current message
            const toolResult = content.toolResult
            toolResponses.set(content.id, {
                result: toolResult?.status === 'success' ? toolResult.value : toolResult,
                isError: toolResult?.status === 'error'
            })
        }
    }

    // Pair tool requests with their responses
    const toolCalls: ToolCallPair[] = []
    for (const [id, request] of toolRequests) {
        const response = toolResponses.get(id)
        toolCalls.push({
            id,
            name: request.name,
            args: request.args,
            result: response?.result,
            isPending: !response && request.status === 'pending',
            isError: response?.isError || request.status === 'error'
        })
    }

    const fullText = textContent.join('\n')

    // Don't render empty messages (no text and no tool calls)
    if (!fullText && toolCalls.length === 0) {
        return null
    }

    return (
        <div className={`message ${isUser ? 'user' : 'assistant'} animate-slide-in`}>
            <div className="message-avatar">
                {isUser ? 'U' : 'G'}
            </div>
            <div className="message-content">
                {fullText && (
                    <div className="message-text">
                        <ReactMarkdown remarkPlugins={[remarkGfm]}>
                            {fullText}
                        </ReactMarkdown>
                    </div>
                )}

                {toolCalls.map((tool) => (
                    <ToolCallDisplay
                        key={tool.id}
                        name={tool.name}
                        args={tool.args}
                        result={tool.result}
                        isPending={tool.isPending}
                        isError={tool.isError}
                    />
                ))}
            </div>
        </div>
    )
}
