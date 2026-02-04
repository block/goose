/**
 * Phase 3: Two Panel Builder - Final Design
 *
 * 3 Modes:
 * 1. Chat + Preview (default) - AI builds, see result
 * 2. Chat + Test - AI builds, try it
 * 3. Edit + Test - Manual edit, try it
 *
 * Layout:
 * - Left (60%): Chat or Edit Form
 * - Right (40%): Preview or Test Chat
 *
 * Toggle controls:
 * - Left panel: [Chat] [Edit] toggle
 * - Right panel: [Preview] [Test] toggle (Preview only available in Chat mode)
 */

import React, { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Send,
  Edit3,
  Play,
  MessageSquare,
  Eye,
  ArrowLeft,
  Save,
  Plus,
  Trash2,
  ChevronDown,
  ChevronRight,
  FileText,
  Settings,
  Zap,
  RefreshCw,
  FlaskConical,
} from 'lucide-react';

// Types
interface RecipeData {
  title: string;
  description: string;
  instructions: string;
  extensions: string[];
  parameters: Array<{
    name: string;
    description: string;
    type: string;
    required: boolean;
  }>;
}

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
}

interface TestMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
}

// Initial state
const initialRecipe: RecipeData = {
  title: '',
  description: '',
  instructions: '',
  extensions: [],
  parameters: [],
};

// ============================================
// Left Panel: Chat
// ============================================

function ChatPanel({
  messages,
  inputValue,
  setInputValue,
  onSend,
  isTyping,
}: {
  messages: ChatMessage[];
  inputValue: string;
  setInputValue: (v: string) => void;
  onSend: () => void;
  isTyping: boolean;
}) {
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  return (
    <div className="flex-1 flex flex-col min-h-0">
      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.length === 0 && (
          <div className="text-center text-gray-400 py-8">
            <MessageSquare className="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p className="text-sm">Start describing your recipe</p>
            <p className="text-xs mt-1">AI will help you build it</p>
          </div>
        )}
        {messages.map((msg) => (
          <div
            key={msg.id}
            className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
          >
            <div
              className={`max-w-[85%] rounded-lg px-4 py-2 ${
                msg.role === 'user'
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 dark:bg-gray-700'
              }`}
            >
              <p className="text-sm whitespace-pre-wrap">{msg.content}</p>
            </div>
          </div>
        ))}
        {isTyping && (
          <div className="flex justify-start">
            <div className="bg-gray-100 dark:bg-gray-700 rounded-lg px-4 py-2">
              <span className="text-sm text-gray-500">Thinking...</span>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="p-4 border-t border-gray-200 dark:border-gray-700">
        <div className="flex gap-2">
          <input
            type="text"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && onSend()}
            placeholder="Describe what your recipe should do..."
            className="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <button
            onClick={onSend}
            disabled={!inputValue.trim()}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Send className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}

// ============================================
// Left Panel: Edit Form
// ============================================

function EditPanel({
  recipe,
  setRecipe,
}: {
  recipe: RecipeData;
  setRecipe: React.Dispatch<React.SetStateAction<RecipeData>>;
}) {
  const [expandedSections, setExpandedSections] = useState({
    instructions: true,
    extensions: false,
    parameters: false,
  });

  const toggleSection = (section: keyof typeof expandedSections) => {
    setExpandedSections((prev) => ({ ...prev, [section]: !prev[section] }));
  };

  const availableExtensions = [
    'developer',
    'computeruse',
    'google_drive',
    'memory',
    'tutorial',
  ];

  const addParameter = () => {
    setRecipe((prev) => ({
      ...prev,
      parameters: [
        ...prev.parameters,
        { name: '', description: '', type: 'string', required: false },
      ],
    }));
  };

  const updateParameter = (index: number, field: string, value: string | boolean) => {
    setRecipe((prev) => ({
      ...prev,
      parameters: prev.parameters.map((p, i) =>
        i === index ? { ...p, [field]: value } : p
      ),
    }));
  };

  const removeParameter = (index: number) => {
    setRecipe((prev) => ({
      ...prev,
      parameters: prev.parameters.filter((_, i) => i !== index),
    }));
  };

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-4">
      {/* Title */}
      <div>
        <label className="block text-sm font-medium mb-1">Title *</label>
        <input
          type="text"
          value={recipe.title}
          onChange={(e) => setRecipe((prev) => ({ ...prev, title: e.target.value }))}
          placeholder="e.g., Email Writing Assistant"
          className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* Description */}
      <div>
        <label className="block text-sm font-medium mb-1">Description</label>
        <input
          type="text"
          value={recipe.description}
          onChange={(e) =>
            setRecipe((prev) => ({ ...prev, description: e.target.value }))
          }
          placeholder="Brief description of what this recipe does"
          className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* Instructions (collapsible) */}
      <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
        <button
          onClick={() => toggleSection('instructions')}
          className="w-full px-4 py-3 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
        >
          <span className="font-medium text-sm">Instructions *</span>
          {expandedSections.instructions ? (
            <ChevronDown className="w-4 h-4" />
          ) : (
            <ChevronRight className="w-4 h-4" />
          )}
        </button>
        {expandedSections.instructions && (
          <div className="px-4 pb-4">
            <textarea
              value={recipe.instructions}
              onChange={(e) =>
                setRecipe((prev) => ({ ...prev, instructions: e.target.value }))
              }
              placeholder="Detailed instructions for the AI..."
              rows={8}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
        )}
      </div>

      {/* Extensions (collapsible) */}
      <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
        <button
          onClick={() => toggleSection('extensions')}
          className="w-full px-4 py-3 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
        >
          <div className="flex items-center gap-2">
            <span className="font-medium text-sm">Extensions</span>
            {recipe.extensions.length > 0 && (
              <span className="px-1.5 py-0.5 text-xs bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-300 rounded">
                {recipe.extensions.length}
              </span>
            )}
          </div>
          {expandedSections.extensions ? (
            <ChevronDown className="w-4 h-4" />
          ) : (
            <ChevronRight className="w-4 h-4" />
          )}
        </button>
        {expandedSections.extensions && (
          <div className="px-4 pb-4">
            <p className="text-xs text-gray-500 mb-3">
              Select tools this recipe can use
            </p>
            <div className="flex flex-wrap gap-2">
              {availableExtensions.map((ext) => (
                <button
                  key={ext}
                  onClick={() => {
                    setRecipe((prev) => ({
                      ...prev,
                      extensions: prev.extensions.includes(ext)
                        ? prev.extensions.filter((e) => e !== ext)
                        : [...prev.extensions, ext],
                    }));
                  }}
                  className={`px-3 py-1.5 text-sm rounded-full border transition-colors ${
                    recipe.extensions.includes(ext)
                      ? 'bg-blue-600 text-white border-blue-600'
                      : 'border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700'
                  }`}
                >
                  {ext}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Parameters (collapsible) */}
      <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
        <button
          onClick={() => toggleSection('parameters')}
          className="w-full px-4 py-3 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
        >
          <div className="flex items-center gap-2">
            <span className="font-medium text-sm">Parameters</span>
            {recipe.parameters.length > 0 && (
              <span className="px-1.5 py-0.5 text-xs bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-300 rounded">
                {recipe.parameters.length}
              </span>
            )}
          </div>
          {expandedSections.parameters ? (
            <ChevronDown className="w-4 h-4" />
          ) : (
            <ChevronRight className="w-4 h-4" />
          )}
        </button>
        {expandedSections.parameters && (
          <div className="px-4 pb-4 space-y-3">
            <p className="text-xs text-gray-500">
              Define inputs users must provide when running this recipe
            </p>
            {recipe.parameters.map((param, index) => (
              <div
                key={index}
                className="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg space-y-2"
              >
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={param.name}
                    onChange={(e) => updateParameter(index, 'name', e.target.value)}
                    placeholder="Parameter name"
                    className="flex-1 px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  />
                  <button
                    onClick={() => removeParameter(index)}
                    className="p-1.5 text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded transition-colors"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
                <input
                  type="text"
                  value={param.description}
                  onChange={(e) => updateParameter(index, 'description', e.target.value)}
                  placeholder="Description"
                  className="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <div className="flex items-center gap-4">
                  <select
                    value={param.type}
                    onChange={(e) => updateParameter(index, 'type', e.target.value)}
                    className="px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  >
                    <option value="string">String</option>
                    <option value="number">Number</option>
                    <option value="boolean">Boolean</option>
                  </select>
                  <label className="flex items-center gap-1.5 text-sm cursor-pointer">
                    <input
                      type="checkbox"
                      checked={param.required}
                      onChange={(e) =>
                        updateParameter(index, 'required', e.target.checked)
                      }
                      className="rounded"
                    />
                    Required
                  </label>
                </div>
              </div>
            ))}
            <button
              onClick={addParameter}
              className="w-full py-2 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg border border-dashed border-gray-300 dark:border-gray-600 flex items-center justify-center gap-1.5 transition-colors"
            >
              <Plus className="w-4 h-4" />
              Add Parameter
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

// ============================================
// Right Panel: Preview
// ============================================

function PreviewPanel({ recipe }: { recipe: RecipeData }) {
  const hasContent = recipe.title || recipe.instructions;

  if (!hasContent) {
    return (
      <div className="flex-1 flex items-center justify-center p-4">
        <div className="text-center text-gray-400">
          <Eye className="w-12 h-12 mx-auto mb-3 opacity-50" />
          <p className="text-sm">Recipe preview</p>
          <p className="text-xs mt-1">Start building to see it here</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto p-4 space-y-4">
      {/* Title & Description */}
      <div>
        <h3 className="text-lg font-semibold">{recipe.title || 'Untitled Recipe'}</h3>
        {recipe.description && (
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            {recipe.description}
          </p>
        )}
      </div>

      {/* Instructions */}
      {recipe.instructions && (
        <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-2">
            <FileText className="w-4 h-4 text-gray-400" />
            <span className="text-sm font-medium">Instructions</span>
          </div>
          <pre className="text-xs text-gray-600 dark:text-gray-300 whitespace-pre-wrap font-mono">
            {recipe.instructions}
          </pre>
        </div>
      )}

      {/* Extensions */}
      {recipe.extensions.length > 0 && (
        <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-2">
            <Zap className="w-4 h-4 text-gray-400" />
            <span className="text-sm font-medium">
              Extensions ({recipe.extensions.length})
            </span>
          </div>
          <div className="flex flex-wrap gap-2">
            {recipe.extensions.map((ext, index) => (
              <span
                key={index}
                className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-700 rounded"
              >
                {ext}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Parameters */}
      {recipe.parameters.length > 0 && (
        <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-2">
            <Settings className="w-4 h-4 text-gray-400" />
            <span className="text-sm font-medium">
              Parameters ({recipe.parameters.length})
            </span>
          </div>
          <div className="space-y-2">
            {recipe.parameters.map((param, index) => (
              <div key={index} className="text-sm">
                <span className="font-mono text-blue-600 dark:text-blue-400">
                  {`{{${param.name || 'unnamed'}}}`}
                </span>
                {param.description && (
                  <span className="text-gray-500 ml-2">- {param.description}</span>
                )}
                {param.required && (
                  <span className="ml-2 text-xs text-red-500">required</span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ============================================
// Right Panel: Test Chat
// ============================================

function TestPanel({
  recipe,
  messages,
  setMessages,
}: {
  recipe: RecipeData;
  messages: TestMessage[];
  setMessages: React.Dispatch<React.SetStateAction<TestMessage[]>>;
}) {
  const [inputValue, setInputValue] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Initialize with greeting if empty
  useEffect(() => {
    if (messages.length === 0) {
      setMessages([
        {
          id: 'system-1',
          role: 'system',
          content: 'Test conversation started',
        },
        {
          id: '1',
          role: 'assistant',
          content: recipe.title
            ? `Hi! I'm your ${recipe.title}. ${recipe.description || ''}\n\nHow can I help you?`
            : "Hi! I'm ready to help. What would you like to do?",
        },
      ]);
    }
  }, []);

  const handleSend = () => {
    if (!inputValue.trim()) return;

    const userMessage: TestMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: inputValue,
    };

    setMessages((prev) => [...prev, userMessage]);
    const userInput = inputValue;
    setInputValue('');
    setIsTyping(true);

    // Simulate response based on recipe
    setTimeout(() => {
      let response: string;

      if (recipe.instructions.toLowerCase().includes('email')) {
        response = `I'll help you with that email.\n\nBased on your request "${userInput}", here's what I suggest:\n\n1. First, let me understand the context\n2. Then I'll draft the email\n3. You can review and adjust\n\nWhat's the main purpose of this email?`;
      } else if (recipe.instructions.toLowerCase().includes('code')) {
        response = `I'll analyze that for you.\n\nLooking at "${userInput}"...\n\nHere are my observations:\n• Code structure looks good\n• Consider adding error handling\n• Documentation could be improved\n\nWould you like me to elaborate on any of these points?`;
      } else {
        response = `I understand you want help with: "${userInput}"\n\nBased on my instructions, I'll:\n1. Analyze your request\n2. Provide relevant assistance\n3. Ask clarifying questions if needed\n\nWhat specific aspect would you like me to focus on?`;
      }

      setMessages((prev) => [
        ...prev,
        {
          id: (Date.now() + 1).toString(),
          role: 'assistant',
          content: response,
        },
      ]);
      setIsTyping(false);
    }, 1200);
  };

  const handleReset = () => {
    setMessages([
      {
        id: 'system-reset',
        role: 'system',
        content: 'Test conversation reset',
      },
      {
        id: 'greeting',
        role: 'assistant',
        content: recipe.title
          ? `Hi! I'm your ${recipe.title}. ${recipe.description || ''}\n\nHow can I help you?`
          : "Hi! I'm ready to help. What would you like to do?",
      },
    ]);
  };

  return (
    <div className="flex-1 flex flex-col min-h-0">
      {/* Reset button */}
      <div className="px-4 py-2 border-b border-gray-200 dark:border-gray-700 flex justify-end">
        <button
          onClick={handleReset}
          className="px-2 py-1 text-xs text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-700 rounded flex items-center gap-1 transition-colors"
        >
          <RefreshCw className="w-3 h-3" />
          Reset
        </button>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {messages.map((msg) => {
          if (msg.role === 'system') {
            return (
              <div key={msg.id} className="flex justify-center">
                <span className="px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700 rounded-full text-gray-500">
                  {msg.content}
                </span>
              </div>
            );
          }
          return (
            <div
              key={msg.id}
              className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
            >
              <div
                className={`max-w-[90%] rounded-lg px-3 py-2 ${
                  msg.role === 'user'
                    ? 'bg-green-600 text-white'
                    : 'bg-gray-100 dark:bg-gray-700'
                }`}
              >
                <p className="text-sm whitespace-pre-wrap">{msg.content}</p>
              </div>
            </div>
          );
        })}
        {isTyping && (
          <div className="flex justify-start">
            <div className="bg-gray-100 dark:bg-gray-700 rounded-lg px-3 py-2">
              <span className="text-sm text-gray-500">Typing...</span>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="p-4 border-t border-gray-200 dark:border-gray-700">
        <div className="flex gap-2">
          <input
            type="text"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSend()}
            placeholder="Test your recipe..."
            className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-sm focus:outline-none focus:ring-2 focus:ring-green-500"
          />
          <button
            onClick={handleSend}
            disabled={!inputValue.trim()}
            className="px-3 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <Send className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}

// ============================================
// Main Component
// ============================================

export default function Phase3_TwoPanel() {
  const navigate = useNavigate();

  // Panel modes
  const [leftMode, setLeftMode] = useState<'chat' | 'edit'>('chat');
  const [rightMode, setRightMode] = useState<'preview' | 'test'>('preview');

  // Recipe state (shared across all modes)
  const [recipe, setRecipe] = useState<RecipeData>(initialRecipe);

  // Chat state
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([
    {
      id: '1',
      role: 'assistant',
      content:
        "Hi! I'll help you create a recipe. What would you like it to do?\n\nFor example:\n• \"Help me write professional emails\"\n• \"Review my code for bugs\"\n• \"Summarize documents\"",
    },
  ]);
  const [chatInput, setChatInput] = useState('');
  const [isTyping, setIsTyping] = useState(false);

  // Test state
  const [testMessages, setTestMessages] = useState<TestMessage[]>([]);

  // When switching to Edit mode, force right panel to Test (no Preview in Edit mode)
  useEffect(() => {
    if (leftMode === 'edit' && rightMode === 'preview') {
      setRightMode('test');
    }
  }, [leftMode]);

  // Handle chat send
  const handleChatSend = () => {
    if (!chatInput.trim()) return;

    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: chatInput,
    };

    setChatMessages((prev) => [...prev, userMessage]);
    const userInput = chatInput;
    setChatInput('');
    setIsTyping(true);

    // Simulate AI building recipe
    setTimeout(() => {
      let response: string;
      let updatedRecipe = { ...recipe };

      if (!recipe.title) {
        // First message - create recipe from intent
        if (userInput.toLowerCase().includes('email')) {
          updatedRecipe = {
            title: 'Email Writing Assistant',
            description: 'Helps write professional emails quickly',
            instructions:
              'You are a professional email writing assistant.\n\n1. Ask what the email is about\n2. Ask who the recipient is\n3. Use professional but friendly tone\n4. Keep emails concise',
            extensions: ['developer'],
            parameters: [],
          };
          response =
            "I've created an **Email Writing Assistant** recipe!\n\nCheck the Preview panel to see what I built. You can:\n• Continue chatting to refine it\n• Switch to Edit mode for manual changes\n• Click Test to try it out";
        } else if (
          userInput.toLowerCase().includes('code') ||
          userInput.toLowerCase().includes('review')
        ) {
          updatedRecipe = {
            title: 'Code Review Assistant',
            description: 'Reviews code and suggests improvements',
            instructions:
              'You are a code review assistant.\n\n1. Analyze code structure\n2. Check for bugs and issues\n3. Suggest improvements\n4. Explain reasoning clearly',
            extensions: ['developer'],
            parameters: [
              {
                name: 'language',
                description: 'Programming language',
                type: 'string',
                required: false,
              },
            ],
          };
          response =
            "I've created a **Code Review Assistant** recipe!\n\nIt includes:\n• Developer tools\n• Optional language parameter\n\nWant me to add anything else?";
        } else {
          updatedRecipe = {
            title: 'Custom Assistant',
            description: userInput.slice(0, 60),
            instructions: `You are an assistant that helps with: ${userInput}\n\nBe helpful, clear, and thorough.`,
            extensions: [],
            parameters: [],
          };
          response =
            "I've started your recipe! Tell me more about:\n• What specific tasks should it handle?\n• Any tools it needs?\n• Any inputs users should provide?";
        }
      } else {
        // Follow-up - refine recipe
        if (
          userInput.toLowerCase().includes('formal') ||
          userInput.toLowerCase().includes('tone')
        ) {
          updatedRecipe.instructions += '\n\nAlways use a formal, professional tone.';
          response =
            "Updated! I've added formal tone guidance.\n\nThe Preview shows your changes. Anything else?";
        } else if (
          userInput.toLowerCase().includes('extension') ||
          userInput.toLowerCase().includes('tool')
        ) {
          response =
            'What tools should this recipe have access to?\n\nAvailable: developer, computeruse, google_drive, memory, tutorial\n\nOr switch to Edit mode to select them directly.';
        } else {
          updatedRecipe.instructions += `\n\n${userInput}`;
          response =
            "Got it! I've added that to the instructions.\n\nCheck Preview to see the update. Ready to test it?";
        }
      }

      setRecipe(updatedRecipe);
      setChatMessages((prev) => [
        ...prev,
        { id: (Date.now() + 1).toString(), role: 'assistant', content: response },
      ]);
      setIsTyping(false);
    }, 1000);
  };

  // Save handler
  const handleSave = () => {
    if (!recipe.title) {
      alert('Please add a title to your recipe');
      return;
    }
    alert('Recipe saved!');
    navigate('/recipes');
  };

  const handleSaveAndRun = () => {
    if (!recipe.title) {
      alert('Please add a title to your recipe');
      return;
    }
    alert('Recipe saved! Starting chat...');
    navigate('/recipes');
  };

  return (
    <div className="h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
      {/* Header */}
      <div className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <button
              onClick={() => navigate('/recipes')}
              className="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
            >
              <ArrowLeft className="w-5 h-5" />
            </button>
            <div>
              <h1 className="text-lg font-semibold">
                {recipe.title || 'New Recipe'}
              </h1>
              <p className="text-sm text-gray-500">
                {leftMode === 'chat' ? 'Building with AI' : 'Manual editing'}
                {rightMode === 'test' && ' • Testing'}
              </p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={handleSave}
              className="px-4 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md flex items-center gap-2 transition-colors"
            >
              <Save className="w-4 h-4" />
              Save
            </button>
            <button
              onClick={handleSaveAndRun}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md flex items-center gap-2 transition-colors"
            >
              <Play className="w-4 h-4" />
              Save & Run
            </button>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex min-h-0">
        {/* Left Panel (60%) */}
        <div className="w-[60%] flex flex-col border-r border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
          {/* Left Panel Header */}
          <div className="px-4 py-3 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
            <div className="flex bg-gray-100 dark:bg-gray-700 rounded-lg p-1">
              <button
                onClick={() => setLeftMode('chat')}
                className={`px-3 py-1.5 text-sm rounded-md flex items-center gap-1.5 transition-colors ${
                  leftMode === 'chat'
                    ? 'bg-white dark:bg-gray-600 shadow-sm font-medium'
                    : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
                }`}
              >
                <MessageSquare className="w-4 h-4" />
                Chat
              </button>
              <button
                onClick={() => setLeftMode('edit')}
                className={`px-3 py-1.5 text-sm rounded-md flex items-center gap-1.5 transition-colors ${
                  leftMode === 'edit'
                    ? 'bg-white dark:bg-gray-600 shadow-sm font-medium'
                    : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
                }`}
              >
                <Edit3 className="w-4 h-4" />
                Edit
              </button>
            </div>
          </div>

          {/* Left Panel Content */}
          {leftMode === 'chat' ? (
            <ChatPanel
              messages={chatMessages}
              inputValue={chatInput}
              setInputValue={setChatInput}
              onSend={handleChatSend}
              isTyping={isTyping}
            />
          ) : (
            <EditPanel recipe={recipe} setRecipe={setRecipe} />
          )}
        </div>

        {/* Right Panel (40%) */}
        <div className="w-[40%] flex flex-col bg-gray-50 dark:bg-gray-900">
          {/* Right Panel Header */}
          <div className="px-4 py-3 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 flex items-center justify-between">
            <div className="flex bg-gray-100 dark:bg-gray-700 rounded-lg p-1">
              <button
                onClick={() => setRightMode('preview')}
                disabled={leftMode === 'edit'}
                className={`px-3 py-1.5 text-sm rounded-md flex items-center gap-1.5 transition-colors ${
                  rightMode === 'preview'
                    ? 'bg-white dark:bg-gray-600 shadow-sm font-medium'
                    : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
                } ${leftMode === 'edit' ? 'opacity-50 cursor-not-allowed' : ''}`}
              >
                <Eye className="w-4 h-4" />
                Preview
              </button>
              <button
                onClick={() => setRightMode('test')}
                className={`px-3 py-1.5 text-sm rounded-md flex items-center gap-1.5 transition-colors ${
                  rightMode === 'test'
                    ? 'bg-white dark:bg-gray-600 shadow-sm font-medium'
                    : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
                }`}
              >
                <FlaskConical className="w-4 h-4" />
                Test
              </button>
            </div>
            {leftMode === 'edit' && (
              <span className="text-xs text-gray-400">Preview unavailable in Edit mode</span>
            )}
          </div>

          {/* Right Panel Content */}
          {rightMode === 'preview' ? (
            <PreviewPanel recipe={recipe} />
          ) : (
            <TestPanel
              recipe={recipe}
              messages={testMessages}
              setMessages={setTestMessages}
            />
          )}
        </div>
      </div>

      {/* Back button */}
      <div className="fixed bottom-6 left-6">
        <button
          onClick={() => navigate('/recipes')}
          className="px-4 py-2 bg-gray-800 text-white rounded-md hover:bg-gray-700 transition-colors"
        >
          ← Back to Recipes
        </button>
      </div>
    </div>
  );
}

export function Phase3TwoPanel_Demo() {
  return <Phase3_TwoPanel />;
}
