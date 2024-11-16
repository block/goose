import React, { useState } from 'react'
import ReactMarkdown from 'react-markdown'
import { BoxIcon, GPSIcon } from './ui/icons'

interface ToolResultProps {
  result: {
    message?: string
    result?: string
    state?: string
    toolCallId?: string
    toolName?: string
    args?: any
    input_todo?: any
  }
  onSubmitInput?: (input: string) => void
}

interface JsonSchemaProperty {
  type: string
  title?: string
  description?: string
  enum?: string[]
  format?: string
  minimum?: number
  maximum?: number
  minLength?: number
  maxLength?: number
}

const getIconForSchemaType = (schema: JsonSchemaProperty) => {
  if (schema.enum) return BoxIcon
  if (schema.type === 'integer' || schema.type === 'number') return BoxIcon
  if (schema.format === 'date' || schema.format === 'date-time') return BoxIcon
  return BoxIcon
}

export default function ToolResult({ result, onSubmitInput }: ToolResultProps) {
  const [inputValues, setInputValues] = useState<Record<string, string>>({})
  const [submitted, setSubmitted] = useState(false)

  if (!result) return null

  const handleSubmit = () => {
    setSubmitted(true)
    const inputString = Object.entries(inputValues)
      .map(([key, value]) => `${key}: ${value}`)
      .join('\n')
    if (onSubmitInput) {
      onSubmitInput(inputString)
    }
  }

  const renderSchemaInput = (key: string, schema: JsonSchemaProperty) => {
    const value = inputValues[key] || ''
    const Icon = getIconForSchemaType(schema)
    
    if (schema.enum) {
      return (
        <div key={key} className="mb-4">
          <div className="text-white mb-2 flex items-center gap-2">
            <BoxIcon size={16} />
            <span>{schema.title || key}</span>
          </div>
          <div className="flex flex-col space-y-2">
            {schema.enum.map((option) => (
              <div key={option} className="flex items-center space-x-2">
                <div
                  className={`cursor-pointer px-3 py-2 rounded-md ${
                    value === option ? 'bg-blue-500' : 'bg-gray-700'
                  }`}
                  onClick={() => setInputValues({ ...inputValues, [key]: option })}>
                  <span className="text-white">{option}</span>
                </div>
              </div>
            ))}
          </div>
          {schema.description && (
            <div className="text-gray-400 text-sm mt-1">{schema.description}</div>
          )}
        </div>
      )
    }

    return (
      <div key={key} className="w-full">
        <div className="text-white mb-2 flex items-center gap-2">
          <Icon size={16} />
          <span>{schema.title || key}</span>
        </div>
        {schema.type === 'integer' || schema.type === 'number' ? (
          <input
            type="number"
            className="w-full p-2 bg-gray-700 text-white rounded-md"
            min={schema.minimum}
            max={schema.maximum}
            step={schema.type === 'integer' ? 1 : 0.1}
            onChange={(e) => setInputValues({ ...inputValues, [key]: e.target.value })}
          />
        ) : schema.format === 'date' ? (
          <input
            type="date"
            className="w-full p-2 bg-gray-700 text-white rounded-md"
            onChange={(e) => setInputValues({ ...inputValues, [key]: e.target.value })}
          />
        ) : schema.format === 'date-time' ? (
          <input
            type="datetime-local"
            className="w-full p-2 bg-gray-700 text-white rounded-md"
            onChange={(e) => setInputValues({ ...inputValues, [key]: e.target.value })}
          />
        ) : (
          schema.type === 'string' && (
            !schema.maxLength || schema.maxLength > 100 ? (
              <div
                className="w-full p-2 bg-gray-700 text-white rounded-md min-h-[100px]"
                contentEditable
                onBlur={(e) => setInputValues({ ...inputValues, [key]: e.currentTarget.textContent || '' })}
                suppressContentEditableWarning
              />
            ) : (
              <input
                type="text"
                className="w-full p-2 bg-gray-700 text-white rounded-md"
                minLength={schema.minLength}
                maxLength={schema.maxLength}
                onChange={(e) => setInputValues({ ...inputValues, [key]: e.target.value })}
              />
            )
          )
        )}
        {schema.description && (
          <div className="text-gray-400 text-sm mt-1">{schema.description}</div>
        )}
      </div>
    )
  }

  if (result) {
    return (
      <div className="mt-2">
        {result.result && (
          <div className="font-mono text-sm bg-black rounded-b-md rounded-tr-md p-3">
            <div className="flex items-center space-x-2">
              <BoxIcon size={14} />
              <span className="text-blue-400">$</span>
              <span className="text-white">{result.toolName}</span>
            </div>
            <div className="mt-2 text-green-400 whitespace-pre-wrap">
              <ReactMarkdown
                components={{
                  code({ node, inline, className, children, ...props }) {
                    {console.log(children)}
                    return (
                      <code className={`${className} ${inline ? 'bg-black bg-opacity-25 px-1 py-0.5 rounded' : ''}`} {...props}>
                        {typeof children === 'string' ? children : "Unrenderable tool result - check logs"}
                      </code>
                    )
                  },
                  pre({ children }) {
                    {console.log(children)}
                    return <div className="whitespace-pre overflow-x-auto">
                      {typeof children === 'string' ? children : "Unrenderable tool result - check logs"}
                    </div>
                  },
                  p({ children }) {
                    {console.log(children)}
                    return <div className="whitespace-pre-wrap">
                      {typeof children === 'string' ? children : "Unrenderable tool result - check logs"}
                    </div>
                  }
                }}
              >
                {result.result}
              </ReactMarkdown>
            </div>
          </div>
        )}

=

        {result.input_todo && !submitted && (
          <div className="mt-4 p-4 bg-gray-800 rounded-md">
            {Object.entries(result.input_todo.properties).map(([key, schema]) =>
              renderSchemaInput(key, schema as JsonSchemaProperty)
            )}              
            <button
              onClick={handleSubmit}
              className="bg-blue-500 text-white px-4 py-2 rounded-md hover:bg-blue-600 cursor-pointer inline-flex items-center gap-2"
            >
              <GPSIcon size={14} />
              <span>Submit</span>
            </button>
          </div>
        )}
      </div>
    )
  }

  if (result.toolName) {
    return (
      <div className="text-white mt-2">
        <div className="font-semibold flex items-center gap-2">
          <BoxIcon size={14} />
          <span>Tool: {result.toolName}</span>
        </div>
        {result.args && (
          <pre className="text-sm mt-1 text-gray-300">
            {JSON.stringify(result.args, null, 2)}
          </pre>
        )}
      </div>
    )
  }

  return null
}