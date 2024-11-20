import React, { useState } from 'react';
import { Card } from './ui/card';
import { Button } from './ui/button';
import { BoxIcon, GPSIcon } from './ui/icons'
import ReactMarkdown from 'react-markdown'


export default function ToolInvocation({ toolInvocation }) {
  return (
    <div key={toolInvocation.toolCallId} className="text-tool space-y-4 transition-all duration-300">
      <ToolCall call={toolInvocation} />
      {toolInvocation.state === 'result' && <ToolResult result={toolInvocation} />}
    </div>
  )
}




interface ToolCallProps {
  call: {
    state: 'call' | 'result'
    toolCallId: string
    toolName: string
    args: Record<string, any>
  }
}

function ToolCall({ call }: ToolCallProps) {
  return (
    <Card className="bg-tool-card p-4 mt-2">
      <div className="flex items-center space-x-2">
        <BoxIcon size={14} />
        <span>Tool Called: {call.toolName.substring(call.toolName.lastIndexOf("__") + 2)}</span>
      </div>

      {call.args && (
        <pre className="mt-1 text-tool-result-green">
          {JSON.stringify(call.args, null, 2)}
        </pre>
      )}
    </Card>
  )
}




interface ResultItem {
  text?: string
  type: 'text' | 'image'
  url?: string
}

interface ToolResultProps {
  result: {
    message?: string
    result?: ResultItem[] | string
    state?: string
    toolCallId?: string
    toolName?: string
    args?: any
    input_todo?: any
  }
}

function ToolResult({ result }: ToolResultProps) {
  console.log('result', result)
  if (!result || !result.result) return null

  return (    
    <Card className="bg-tool-card mt-2 p-4">
      <div className="rounded-b-md rounded-tr-md p-3">
        <div className="flex items-center space-x-2">
          <BoxIcon size={14} />
          <span>Tool Result: {result.toolName.substring(result.toolName.lastIndexOf("__") + 2)}</span>
        </div>
        {Array.isArray(result.result) ? (
          <div className="mt-2">
            {result.result.map((item: ResultItem, index: number) => (
              <div key={index} className="mb-2">
                {item.type === 'text' && item.text && (
                  <ReactMarkdown
                    className="text-tool-result-green whitespace-pre-wrap"
                    components={{
                      code({ node, className, children, ...props }) {
                        return (
                          <code className={className} {...props}>
                            {typeof children === 'string' ? children : "Unrenderable tool result - check logs"}
                          </code>
                        )
                      },
                      pre({ children }) {
                        return <div className="whitespace-pre overflow-x-auto">
                          {typeof children === 'string' ? children : "Unrenderable tool result - check logs"}
                        </div>
                      },
                      p({ children }) {
                        return <div>
                          {typeof children === 'string' ? children : "Unrenderable tool result - check logs"}
                        </div>
                      }
                    }}
                  >
                    {item.text}
                  </ReactMarkdown>
                )}
                {item.type === 'image' && item.url && (
                  <img 
                    src={item.url} 
                    alt="Tool result" 
                    className="max-w-full h-auto rounded-md"
                  />
                )}
              </div>
            ))}
          </div>
        ) : (
          <ReactMarkdown
            className="mt-2 text-tool-result-green whitespace-pre-wrap"
            components={{
              code({ node, className, children, ...props }) {
                return (
                  <code className={className} {...props}>
                    {typeof children === 'string' ? children : "Unrenderable tool result - check logs"}
                  </code>
                )
              },
              pre({ children }) {
                return <div className="whitespace-pre overflow-x-auto">
                  {typeof children === 'string' ? children : "Unrenderable tool result - check logs"}
                </div>
              },
              p({ children }) {
                return <div>
                  {typeof children === 'string' ? children : "Unrenderable tool result - check logs"}
                </div>
              }
            }}
          >
            {result.result}
          </ReactMarkdown>
        )}
      </div>
    </Card>
  )
}




interface ToolResponseFormProps {
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

function getIconForSchemaType(schema: JsonSchemaProperty) {
  if (schema.enum) return BoxIcon
  if (schema.type === 'integer' || schema.type === 'number') return BoxIcon
  if (schema.format === 'date' || schema.format === 'date-time') return BoxIcon
  return BoxIcon
}

// NOTE - Not yet tested/matched to designs
function ToolResponseForm({ result, onSubmitInput }: ToolResponseFormProps) {
  const [inputValues, setInputValues] = useState<Record<string, string>>({})
  const [submitted, setSubmitted] = useState(false)

  const handleSubmit = () => {
    setSubmitted(true)
    const inputString = Object.entries(inputValues)
      .map(([key, value]) => `${key}: ${value}`)
      .join('\n')
    if (onSubmitInput) {
      onSubmitInput(inputString)
    }
  }

  {result.input_todo && !submitted && (
    <div>
      <div className="mt-4 p-4 bg-gray-800 rounded-md">
        {Object.entries(result.input_todo.properties).map(([key, schema]: [key: string, schema: JsonSchemaProperty]) => {
          const value = inputValues[key] || ''
          const Icon = getIconForSchemaType(schema as JsonSchemaProperty)
            
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
        )})}
      </div>
      <Button onClick={handleSubmit} className="w-full transition-colors tool-form-button">
        <GPSIcon size={14} />
        <span>Submit</span>
      </Button>
    </div>
  )}
}

// TODO - Use when we have plans
/*
function AppoveFlightPlanButton(){
  return (
    <Button className="w-full transition-colors tool-form-button">
      Take flight with this direction
    </Button>
  )
} */
