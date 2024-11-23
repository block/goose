import React from 'react'
import ToolInvocation from './ToolInvocation'
import ReactMarkdown from 'react-markdown'


export default function GooseMessage({ message }) {
  return (
    <div className="flex mb-4">
      <div className="bg-goose-bubble w-full text-black rounded-2xl p-4">
        {message.toolInvocations ? (
          <div className="space-y-4">
            {message.toolInvocations.map((toolInvocation) => (
              <ToolInvocation
                key={toolInvocation.toolCallId}
                toolInvocation={toolInvocation}
              />
            ))}
          </div>
        ) : (
          <ReactMarkdown className="prose">{message.content}</ReactMarkdown>
        )}
      </div>
    </div>
  )
};
