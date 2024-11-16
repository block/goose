import React from 'react'
import ToolInvocation from './ToolInvocation'
import ReactMarkdown from 'react-markdown'

export default function GooseMessage({ message, useChatData }) {
  return (
    <div className="flex mb-4 w-auto max-w-full">
      <div className="bg-goose-bubble text-black rounded-2xl p-4">
        {message.toolInvocations ? (
          <div className="space-y-4">
            {message.toolInvocations.map((toolInvocation) => (
              <ToolInvocation
                key={toolInvocation.toolCallId}
                toolInvocation={toolInvocation}
                handleSubmit={useChatData.handleSubmit}
                handleInputChange={useChatData.handleInputChange}
              />
            ))}
          </div>
        ) : (
          <ReactMarkdown>{message.content}</ReactMarkdown>
        )}
      </div>
    </div>
  )
};
