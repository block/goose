/**
 * Phase 3: Vertical Builder - Simplified Recipe Creation
 *
 * Layout:
 * - Chat Mode: Chat panel (scrollable) + Preview (fixed bottom)
 * - Edit Mode: Replaces chat+preview with form
 * - Test Panel: Slides from right when activated
 *
 * Key features:
 * - Single column focus
 * - Test as slide-out panel (not modal)
 * - Clear mode switching (chat ↔ edit)
 */

import React, { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Send,
  Edit3,
  Play,
  X,
  ArrowLeft,
  Save,
  Plus,
  Trash2,
  ChevronDown,
  ChevronRight,
  FileText,
  Settings,
  Zap,
  MessageSquare,
  RefreshCw,
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
// Chat Mode Components
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
        {messages.map((msg) => (
          <div
            key={msg.id}
            className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
          >
            <div
              className={`max-w-[80%] rounded-lg px-4 py-2 ${
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
            onKeyDown={(e) => e.key === 'Enter' && onSend()}
            placeholder="Describe what your recipe should do..."
            className="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700"
          />
          <button
            onClick={onSend}
            disabled={!inputValue.trim()}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg disabled:opacity-50"
          >
            <Send className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}

function RecipePreview({
  recipe,
  onEdit,
  onTest,
}: {
  recipe: RecipeData;
  onEdit: () => void;
  onTest: () => void;
}) {
  const hasContent = recipe.title || recipe.instructions;

  if (!hasContent) {
    return (
      <div className="border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 p-4">
        <div className="text-center text-gray-400 py-4">
          <FileText className="w-8 h-8 mx-auto mb-2 opacity-50" />
          <p className="text-sm">Your recipe will appear here</p>
          <p className="text-xs mt-1">Start chatting to build it</p>
        </div>
      </div>
    );
  }

  return (
    <div className="border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4">
      {/* Header with actions */}
      <div className="flex items-center justify-between mb-3">
        <h3 className="font-medium text-sm text-gray-500">Recipe Preview</h3>
        <div className="flex gap-2">
          <button
            onClick={onEdit}
            className="px-3 py-1.5 text-sm text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded flex items-center gap-1.5"
          >
            <Edit3 className="w-3.5 h-3.5" />
            Edit
          </button>
          <button
            onClick={onTest}
            className="px-3 py-1.5 text-sm bg-green-600 hover:bg-green-700 text-white rounded flex items-center gap-1.5"
          >
            <Play className="w-3.5 h-3.5" />
            Test
          </button>
        </div>
      </div>

      {/* Recipe summary */}
      <div className="bg-gray-50 dark:bg-gray-900 rounded-lg p-3 space-y-2">
        <div>
          <span className="font-medium">{recipe.title || 'Untitled Recipe'}</span>
          {recipe.description && (
            <p className="text-xs text-gray-500 mt-0.5">{recipe.description}</p>
          )}
        </div>

        {recipe.instructions && (
          <div className="text-xs text-gray-600 dark:text-gray-400 line-clamp-2">
            {recipe.instructions}
          </div>
        )}

        <div className="flex gap-4 text-xs text-gray-500">
          {recipe.extensions.length > 0 && (
            <span className="flex items-center gap-1">
              <Zap className="w-3 h-3" />
              {recipe.extensions.length} extensions
            </span>
          )}
          {recipe.parameters.length > 0 && (
            <span className="flex items-center gap-1">
              <Settings className="w-3 h-3" />
              {recipe.parameters.length} parameters
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

// ============================================
// Edit Mode Components
// ============================================

function EditForm({
  recipe,
  setRecipe,
  onBack,
  onTest,
}: {
  recipe: RecipeData;
  setRecipe: React.Dispatch<React.SetStateAction<RecipeData>>;
  onBack: () => void;
  onTest: () => void;
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
    <div className="flex-1 flex flex-col min-h-0">
      {/* Header */}
      <div className="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <button
            onClick={onBack}
            className="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
          <h2 className="font-medium">Edit Recipe</h2>
        </div>
        <button
          onClick={onTest}
          className="px-3 py-1.5 text-sm bg-green-600 hover:bg-green-700 text-white rounded flex items-center gap-1.5"
        >
          <Play className="w-3.5 h-3.5" />
          Test
        </button>
      </div>

      {/* Form */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {/* Title */}
        <div>
          <label className="block text-sm font-medium mb-1">Title *</label>
          <input
            type="text"
            value={recipe.title}
            onChange={(e) => setRecipe((prev) => ({ ...prev, title: e.target.value }))}
            placeholder="e.g., Email Writing Assistant"
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700"
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
            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700"
          />
        </div>

        {/* Instructions (collapsible) */}
        <div className="border border-gray-200 dark:border-gray-700 rounded-lg">
          <button
            onClick={() => toggleSection('instructions')}
            className="w-full px-4 py-3 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-800"
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
                rows={6}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 font-mono text-sm"
              />
            </div>
          )}
        </div>

        {/* Extensions (collapsible) */}
        <div className="border border-gray-200 dark:border-gray-700 rounded-lg">
          <button
            onClick={() => toggleSection('extensions')}
            className="w-full px-4 py-3 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-800"
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
                    className={`px-3 py-1.5 text-sm rounded-full border ${
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
        <div className="border border-gray-200 dark:border-gray-700 rounded-lg">
          <button
            onClick={() => toggleSection('parameters')}
            className="w-full px-4 py-3 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-800"
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
                      className="flex-1 px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700"
                    />
                    <button
                      onClick={() => removeParameter(index)}
                      className="p-1 text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                  <input
                    type="text"
                    value={param.description}
                    onChange={(e) => updateParameter(index, 'description', e.target.value)}
                    placeholder="Description"
                    className="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700"
                  />
                  <div className="flex items-center gap-4">
                    <select
                      value={param.type}
                      onChange={(e) => updateParameter(index, 'type', e.target.value)}
                      className="px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700"
                    >
                      <option value="string">String</option>
                      <option value="number">Number</option>
                      <option value="boolean">Boolean</option>
                    </select>
                    <label className="flex items-center gap-1.5 text-sm">
                      <input
                        type="checkbox"
                        checked={param.required}
                        onChange={(e) =>
                          updateParameter(index, 'required', e.target.checked)
                        }
                      />
                      Required
                    </label>
                  </div>
                </div>
              ))}
              <button
                onClick={addParameter}
                className="w-full py-2 text-sm text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg border border-dashed border-gray-300 dark:border-gray-600 flex items-center justify-center gap-1.5"
              >
                <Plus className="w-4 h-4" />
                Add Parameter
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// ============================================
// Test Panel Component
// ============================================

function TestPanel({
  recipe,
  onClose,
}: {
  recipe: RecipeData;
  onClose: () => void;
}) {
  const [messages, setMessages] = useState<TestMessage[]>([
    {
      id: 'system-1',
      role: 'system',
      content: 'Test conversation started',
    },
    {
      id: '1',
      role: 'assistant',
      content: recipe.instructions
        ? `Hi! I'm ready to help. ${recipe.description || ''}\n\nHow can I assist you?`
        : 'Hi! How can I help you today?',
    },
  ]);
  const [inputValue, setInputValue] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSend = () => {
    if (!inputValue.trim()) return;

    const userMessage: TestMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: inputValue,
    };

    setMessages((prev) => [...prev, userMessage]);
    setInputValue('');
    setIsTyping(true);

    // Simulate response
    setTimeout(() => {
      setMessages((prev) => [
        ...prev,
        {
          id: (Date.now() + 1).toString(),
          role: 'assistant',
          content: `I understand you want help with: "${inputValue}"\n\nBased on my instructions, I'll assist you with this task. What specific details would you like me to focus on?`,
        },
      ]);
      setIsTyping(false);
    }, 1200);
  };

  const handleReset = () => {
    setMessages([
      {
        id: 'system-1',
        role: 'system',
        content: 'Test conversation reset',
      },
      {
        id: '1',
        role: 'assistant',
        content: recipe.instructions
          ? `Hi! I'm ready to help. ${recipe.description || ''}\n\nHow can I assist you?`
          : 'Hi! How can I help you today?',
      },
    ]);
    setInputValue('');
  };

  return (
    <div className="w-[400px] border-l border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
        <div>
          <h3 className="font-medium">Test Recipe</h3>
          <p className="text-xs text-gray-500">Try before saving</p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleReset}
            className="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
            title="Reset conversation"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
          <button
            onClick={onClose}
            className="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
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
                className={`max-w-[85%] rounded-lg px-3 py-2 ${
                  msg.role === 'user'
                    ? 'bg-blue-600 text-white'
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
            className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-sm"
          />
          <button
            onClick={handleSend}
            disabled={!inputValue.trim()}
            className="px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg disabled:opacity-50"
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

export default function Phase3_VerticalBuilder() {
  const navigate = useNavigate();
  const [mode, setMode] = useState<'chat' | 'edit'>('chat');
  const [showTest, setShowTest] = useState(false);
  const [recipe, setRecipe] = useState<RecipeData>(initialRecipe);
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([
    {
      id: '1',
      role: 'assistant',
      content:
        "Hi! I'll help you create a recipe. What would you like your recipe to do?\n\nFor example:\n• \"Help me write professional emails\"\n• \"Analyze CSV files and create reports\"\n• \"Code review assistant for Python\"",
    },
  ]);
  const [inputValue, setInputValue] = useState('');
  const [isTyping, setIsTyping] = useState(false);

  // Simulate AI building recipe from chat
  const handleSend = () => {
    if (!inputValue.trim()) return;

    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: inputValue,
    };

    setChatMessages((prev) => [...prev, userMessage]);
    const userInput = inputValue;
    setInputValue('');
    setIsTyping(true);

    // Simulate AI response and recipe building
    setTimeout(() => {
      let response: string;
      let updatedRecipe = { ...recipe };

      if (!recipe.title) {
        // First message - extract intent and create initial recipe
        if (userInput.toLowerCase().includes('email')) {
          updatedRecipe = {
            title: 'Email Writing Assistant',
            description: 'Helps write professional emails quickly',
            instructions:
              'You are a professional email writing assistant. Help users compose clear, professional emails.',
            extensions: ['developer'],
            parameters: [],
          };
          response =
            "Great! I've started building an **Email Writing Assistant** recipe.\n\n" +
            "Here's what I've set up:\n" +
            '• Professional tone guidance\n' +
            '• Clear structure templates\n\n' +
            'Would you like to add any specific requirements? For example:\n' +
            '• Specific tone (formal, friendly, urgent)\n' +
            '• Maximum length limits\n' +
            '• Template for specific email types';
        } else if (userInput.toLowerCase().includes('code') || userInput.toLowerCase().includes('review')) {
          updatedRecipe = {
            title: 'Code Review Assistant',
            description: 'Reviews code and suggests improvements',
            instructions:
              'You are a code review assistant. Analyze code for bugs, improvements, and best practices.',
            extensions: ['developer'],
            parameters: [
              {
                name: 'language',
                description: 'Programming language to review',
                type: 'string',
                required: false,
              },
            ],
          };
          response =
            "Perfect! I've created a **Code Review Assistant** recipe.\n\n" +
            'It includes:\n' +
            '• Developer tools access\n' +
            '• Language parameter (optional)\n\n' +
            'What else would you like it to check for? Security issues? Performance?';
        } else {
          updatedRecipe = {
            title: 'Custom Assistant',
            description: userInput.slice(0, 50),
            instructions: `You are an assistant that helps with: ${userInput}`,
            extensions: [],
            parameters: [],
          };
          response =
            "I've started your **Custom Assistant** recipe.\n\n" +
            "I'll need a bit more detail to make it great:\n" +
            '• What specific tasks should it handle?\n' +
            '• Any tools it needs? (file access, web, etc.)\n' +
            '• Any parameters users should provide?';
        }
      } else {
        // Follow-up messages - refine the recipe
        if (userInput.toLowerCase().includes('tone') || userInput.toLowerCase().includes('formal')) {
          updatedRecipe.instructions += '\n\nAlways maintain a professional and formal tone.';
          response =
            "I've added formal tone guidance to the instructions.\n\n" +
            'The recipe preview below shows the current state. You can:\n' +
            '• Continue describing what you want\n' +
            '• Click **Edit** to manually adjust\n' +
            '• Click **Test** to try it out';
        } else if (userInput.toLowerCase().includes('parameter') || userInput.toLowerCase().includes('input')) {
          response =
            'What parameter would you like to add? Tell me:\n' +
            '• The parameter name\n' +
            '• What it represents\n' +
            '• Whether it should be required\n\n' +
            'Or click **Edit** to add it manually.';
        } else {
          updatedRecipe.instructions += `\n\n${userInput}`;
          response =
            "I've incorporated that into the recipe.\n\n" +
            "Is there anything else you'd like to add or modify?";
        }
      }

      setRecipe(updatedRecipe);
      setChatMessages((prev) => [
        ...prev,
        {
          id: (Date.now() + 1).toString(),
          role: 'assistant',
          content: response,
        },
      ]);
      setIsTyping(false);
    }, 1000);
  };

  const handleSave = () => {
    if (!recipe.title) {
      alert('Please add a title to your recipe');
      return;
    }
    alert('Recipe saved!');
    navigate('/recipes');
  };

  return (
    <div className="h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
      {/* Top Header */}
      <div className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <button
              onClick={() => navigate('/recipes')}
              className="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
            >
              <ArrowLeft className="w-5 h-5" />
            </button>
            <div>
              <h1 className="text-lg font-semibold">
                {recipe.title || 'New Recipe'}
              </h1>
              <p className="text-sm text-gray-500">
                {mode === 'chat' ? 'Build with AI' : 'Edit manually'}
              </p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            {/* Mode toggle */}
            <div className="flex bg-gray-100 dark:bg-gray-700 rounded-lg p-1">
              <button
                onClick={() => setMode('chat')}
                className={`px-3 py-1.5 text-sm rounded-md flex items-center gap-1.5 ${
                  mode === 'chat'
                    ? 'bg-white dark:bg-gray-600 shadow-sm'
                    : 'text-gray-600 dark:text-gray-400'
                }`}
              >
                <MessageSquare className="w-4 h-4" />
                Chat
              </button>
              <button
                onClick={() => setMode('edit')}
                className={`px-3 py-1.5 text-sm rounded-md flex items-center gap-1.5 ${
                  mode === 'edit'
                    ? 'bg-white dark:bg-gray-600 shadow-sm'
                    : 'text-gray-600 dark:text-gray-400'
                }`}
              >
                <Edit3 className="w-4 h-4" />
                Edit
              </button>
            </div>
            <button
              onClick={handleSave}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md flex items-center gap-2"
            >
              <Save className="w-4 h-4" />
              Save Recipe
            </button>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex min-h-0">
        {/* Main panel */}
        <div className="flex-1 flex flex-col bg-white dark:bg-gray-800">
          {mode === 'chat' ? (
            <>
              <ChatPanel
                messages={chatMessages}
                inputValue={inputValue}
                setInputValue={setInputValue}
                onSend={handleSend}
                isTyping={isTyping}
              />
              <RecipePreview
                recipe={recipe}
                onEdit={() => setMode('edit')}
                onTest={() => setShowTest(true)}
              />
            </>
          ) : (
            <EditForm
              recipe={recipe}
              setRecipe={setRecipe}
              onBack={() => setMode('chat')}
              onTest={() => setShowTest(true)}
            />
          )}
        </div>

        {/* Test Panel (slide out) */}
        {showTest && (
          <TestPanel
            recipe={recipe}
            onClose={() => setShowTest(false)}
          />
        )}
      </div>

      {/* Back to prototypes */}
      <div className="fixed bottom-6 left-6">
        <button
          onClick={() => navigate('/recipes')}
          className="px-4 py-2 bg-gray-800 text-white rounded-md hover:bg-gray-700"
        >
          ← Back to Recipes
        </button>
      </div>
    </div>
  );
}

export function Phase3Vertical_Demo() {
  return <Phase3_VerticalBuilder />;
}
