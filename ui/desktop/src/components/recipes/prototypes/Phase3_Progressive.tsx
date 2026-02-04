/**
 * Phase 3: Progressive Reveal - Best UX Design
 *
 * Flow:
 * 1. Start with full-width chat (clean, focused)
 * 2. After AI builds → Recipe card slides in from right
 * 3. Sections collapsed by default (progressive disclosure)
 * 4. Click to expand/edit sections inline
 * 5. Test opens new session
 * 6. Save when ready
 *
 * Key UX principles:
 * - Starts simple, complexity revealed progressively
 * - "Reward moment" when recipe appears
 * - Inline editing, no mode switching
 * - Clear next steps always visible
 */

import React, { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Send,
  ChevronDown,
  ChevronRight,
  Play,
  Save,
  ArrowLeft,
  Sparkles,
  FileText,
  Zap,
  Settings,
  Plus,
  Trash2,
  Check,
  X,
  ExternalLink,
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

// Initial empty recipe
const emptyRecipe: RecipeData = {
  title: '',
  description: '',
  instructions: '',
  extensions: [],
  parameters: [],
};

// ============================================
// Collapsible Section Component
// ============================================

function CollapsibleSection({
  title,
  icon: Icon,
  badge,
  expanded,
  onToggle,
  children,
  isEmpty,
}: {
  title: string;
  icon: React.ElementType;
  badge?: string | number;
  expanded: boolean;
  onToggle: () => void;
  children: React.ReactNode;
  isEmpty?: boolean;
}) {
  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
      <button
        onClick={onToggle}
        className="w-full px-4 py-3 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
      >
        <div className="flex items-center gap-2">
          <Icon className="w-4 h-4 text-gray-400" />
          <span className="font-medium text-sm">{title}</span>
          {badge !== undefined && badge !== 0 && (
            <span className="px-1.5 py-0.5 text-xs bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-300 rounded">
              {badge}
            </span>
          )}
          {isEmpty && (
            <span className="text-xs text-gray-400">(empty)</span>
          )}
        </div>
        {expanded ? (
          <ChevronDown className="w-4 h-4 text-gray-400" />
        ) : (
          <ChevronRight className="w-4 h-4 text-gray-400" />
        )}
      </button>
      {expanded && (
        <div className="px-4 pb-4 border-t border-gray-100 dark:border-gray-700 pt-3">
          {children}
        </div>
      )}
    </div>
  );
}

// ============================================
// Recipe Card Component
// ============================================

function RecipeCard({
  recipe,
  setRecipe,
  onTest,
  onSave,
  isVisible,
}: {
  recipe: RecipeData;
  setRecipe: React.Dispatch<React.SetStateAction<RecipeData>>;
  onTest: () => void;
  onSave: () => void;
  isVisible: boolean;
}) {
  const [expandedSections, setExpandedSections] = useState({
    instructions: false,
    extensions: false,
    parameters: false,
  });

  const [editingTitle, setEditingTitle] = useState(false);
  const [editingDescription, setEditingDescription] = useState(false);
  const titleInputRef = useRef<HTMLInputElement>(null);
  const descInputRef = useRef<HTMLInputElement>(null);

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

  // Focus input when editing starts
  useEffect(() => {
    if (editingTitle && titleInputRef.current) {
      titleInputRef.current.focus();
      titleInputRef.current.select();
    }
  }, [editingTitle]);

  useEffect(() => {
    if (editingDescription && descInputRef.current) {
      descInputRef.current.focus();
      descInputRef.current.select();
    }
  }, [editingDescription]);

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
    <div
      className={`w-[400px] border-l border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 flex flex-col transition-all duration-500 ease-out ${
        isVisible
          ? 'translate-x-0 opacity-100'
          : 'translate-x-full opacity-0 pointer-events-none'
      }`}
    >
      {/* Header */}
      <div className="p-4 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-2 mb-3">
          <Sparkles className="w-5 h-5 text-yellow-500" />
          <span className="text-sm font-medium text-gray-500">Your Recipe</span>
        </div>

        {/* Editable Title */}
        {editingTitle ? (
          <div className="flex items-center gap-2">
            <input
              ref={titleInputRef}
              type="text"
              value={recipe.title}
              onChange={(e) =>
                setRecipe((prev) => ({ ...prev, title: e.target.value }))
              }
              onBlur={() => setEditingTitle(false)}
              onKeyDown={(e) => e.key === 'Enter' && setEditingTitle(false)}
              className="flex-1 text-xl font-semibold px-2 py-1 border border-blue-500 rounded bg-blue-50 dark:bg-blue-900/20 focus:outline-none"
              placeholder="Recipe title"
            />
            <button
              onClick={() => setEditingTitle(false)}
              className="p-1 text-green-600 hover:bg-green-50 rounded"
            >
              <Check className="w-4 h-4" />
            </button>
          </div>
        ) : (
          <h2
            onClick={() => setEditingTitle(true)}
            className="text-xl font-semibold cursor-pointer hover:text-blue-600 transition-colors"
            title="Click to edit"
          >
            {recipe.title || 'Untitled Recipe'}
          </h2>
        )}

        {/* Editable Description */}
        {editingDescription ? (
          <div className="flex items-center gap-2 mt-2">
            <input
              ref={descInputRef}
              type="text"
              value={recipe.description}
              onChange={(e) =>
                setRecipe((prev) => ({ ...prev, description: e.target.value }))
              }
              onBlur={() => setEditingDescription(false)}
              onKeyDown={(e) => e.key === 'Enter' && setEditingDescription(false)}
              className="flex-1 text-sm px-2 py-1 border border-blue-500 rounded bg-blue-50 dark:bg-blue-900/20 focus:outline-none"
              placeholder="Brief description"
            />
            <button
              onClick={() => setEditingDescription(false)}
              className="p-1 text-green-600 hover:bg-green-50 rounded"
            >
              <Check className="w-4 h-4" />
            </button>
          </div>
        ) : (
          <p
            onClick={() => setEditingDescription(true)}
            className="text-sm text-gray-500 mt-1 cursor-pointer hover:text-blue-600 transition-colors"
            title="Click to edit"
          >
            {recipe.description || 'Click to add description...'}
          </p>
        )}
      </div>

      {/* Sections */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {/* Instructions */}
        <CollapsibleSection
          title="Instructions"
          icon={FileText}
          expanded={expandedSections.instructions}
          onToggle={() => toggleSection('instructions')}
          isEmpty={!recipe.instructions}
        >
          <textarea
            value={recipe.instructions}
            onChange={(e) =>
              setRecipe((prev) => ({ ...prev, instructions: e.target.value }))
            }
            placeholder="Detailed instructions for the AI..."
            rows={6}
            className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 font-mono focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </CollapsibleSection>

        {/* Extensions */}
        <CollapsibleSection
          title="Extensions"
          icon={Zap}
          badge={recipe.extensions.length}
          expanded={expandedSections.extensions}
          onToggle={() => toggleSection('extensions')}
          isEmpty={recipe.extensions.length === 0}
        >
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
        </CollapsibleSection>

        {/* Parameters */}
        <CollapsibleSection
          title="Parameters"
          icon={Settings}
          badge={recipe.parameters.length}
          expanded={expandedSections.parameters}
          onToggle={() => toggleSection('parameters')}
          isEmpty={recipe.parameters.length === 0}
        >
          <p className="text-xs text-gray-500 mb-3">
            Define inputs users provide when running this recipe
          </p>
          <div className="space-y-3">
            {recipe.parameters.map((param, index) => (
              <div
                key={index}
                className="p-3 bg-gray-50 dark:bg-gray-900 rounded-lg space-y-2"
              >
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={param.name}
                    onChange={(e) => updateParameter(index, 'name', e.target.value)}
                    placeholder="name"
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
                  onChange={(e) =>
                    updateParameter(index, 'description', e.target.value)
                  }
                  placeholder="Description"
                  className="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <div className="flex items-center gap-4">
                  <select
                    value={param.type}
                    onChange={(e) => updateParameter(index, 'type', e.target.value)}
                    className="px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700"
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
              className="w-full py-2 text-sm text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg border border-dashed border-gray-300 dark:border-gray-600 flex items-center justify-center gap-1.5 transition-colors"
            >
              <Plus className="w-4 h-4" />
              Add Parameter
            </button>
          </div>
        </CollapsibleSection>
      </div>

      {/* Actions */}
      <div className="p-4 border-t border-gray-200 dark:border-gray-700 space-y-2">
        <button
          onClick={onTest}
          className="w-full py-2.5 bg-green-600 hover:bg-green-700 text-white rounded-lg flex items-center justify-center gap-2 transition-colors"
        >
          <Play className="w-4 h-4" />
          Test Recipe
          <ExternalLink className="w-3 h-3 opacity-60" />
        </button>
        <button
          onClick={onSave}
          className="w-full py-2.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg flex items-center justify-center gap-2 transition-colors"
        >
          <Save className="w-4 h-4" />
          Save Recipe
        </button>
      </div>
    </div>
  );
}

// ============================================
// Test Panel Component
// ============================================

function TestPanel({
  recipe,
  messages,
  setMessages,
  onBack,
}: {
  recipe: RecipeData;
  messages: TestMessage[];
  setMessages: React.Dispatch<React.SetStateAction<TestMessage[]>>;
  onBack: () => void;
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
          content: 'Test session started',
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
        response = `I'll help you with that email.\n\nBased on your request "${userInput}", here's what I suggest:\n\n**Subject:** Re: Your Request\n\n**Body:**\nDear [Recipient],\n\nThank you for reaching out. I wanted to follow up on ${userInput.slice(0, 30)}...\n\nBest regards`;
      } else if (recipe.instructions.toLowerCase().includes('code')) {
        response = `Analyzing: "${userInput}"\n\n**Code Review:**\n• Structure: Good\n• Potential issues: Consider edge cases\n• Suggestions: Add error handling\n\nWould you like me to elaborate?`;
      } else {
        response = `I understand you want help with: "${userInput}"\n\nBased on my instructions, I'll:\n1. Analyze your request\n2. Provide relevant assistance\n\nWhat specific aspect would you like me to focus on?`;
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
    }, 1000);
  };

  const handleReset = () => {
    setMessages([
      {
        id: 'system-reset',
        role: 'system',
        content: 'Test session reset',
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
    <div className="w-[400px] border-l border-green-200 dark:border-green-900 bg-green-50 dark:bg-green-950 flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-green-200 dark:border-green-900 bg-green-100 dark:bg-green-900">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <FlaskConical className="w-5 h-5 text-green-600" />
            <div>
              <h3 className="font-medium text-green-800 dark:text-green-200">Test Mode</h3>
              <p className="text-xs text-green-600 dark:text-green-400">Try your recipe</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleReset}
              className="p-1.5 hover:bg-green-200 dark:hover:bg-green-800 rounded transition-colors"
              title="Reset conversation"
            >
              <RefreshCw className="w-4 h-4 text-green-600" />
            </button>
            <button
              onClick={onBack}
              className="px-3 py-1.5 text-sm bg-white dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md transition-colors"
            >
              ← Back to Recipe
            </button>
          </div>
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {messages.map((msg) => {
          if (msg.role === 'system') {
            return (
              <div key={msg.id} className="flex justify-center">
                <span className="px-2 py-1 text-xs bg-green-200 dark:bg-green-800 rounded-full text-green-700 dark:text-green-300">
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
                    : 'bg-white dark:bg-gray-800 border border-green-200 dark:border-green-800'
                }`}
              >
                <p className="text-sm whitespace-pre-wrap">{msg.content}</p>
              </div>
            </div>
          );
        })}
        {isTyping && (
          <div className="flex justify-start">
            <div className="bg-white dark:bg-gray-800 border border-green-200 dark:border-green-800 rounded-lg px-3 py-2">
              <span className="text-sm text-gray-500">Typing...</span>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="p-4 border-t border-green-200 dark:border-green-900 bg-green-100 dark:bg-green-900">
        <div className="flex gap-2">
          <input
            type="text"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSend()}
            placeholder="Test your recipe..."
            className="flex-1 px-3 py-2 border border-green-300 dark:border-green-700 rounded-lg bg-white dark:bg-gray-800 text-sm focus:outline-none focus:ring-2 focus:ring-green-500"
          />
          <button
            onClick={handleSend}
            disabled={!inputValue.trim()}
            className="px-3 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <Send className="w-4 h-4" />
          </button>
        </div>
        <p className="text-xs text-green-600 dark:text-green-400 mt-2 text-center">
          Not working as expected? Ask the build chat to adjust the recipe.
        </p>
      </div>
    </div>
  );
}

// ============================================
// Main Component
// ============================================

export default function Phase3_Progressive() {
  const navigate = useNavigate();

  // Recipe state
  const [recipe, setRecipe] = useState<RecipeData>(emptyRecipe);
  const [recipeVisible, setRecipeVisible] = useState(false);

  // Test mode state
  const [testMode, setTestMode] = useState(false);
  const [testMessages, setTestMessages] = useState<TestMessage[]>([]);

  // Chat state
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: '1',
      role: 'assistant',
      content:
        "Hi! I'll help you create a recipe. Just describe what you want it to do, and I'll build it for you.\n\nFor example:\n• \"Help me write professional emails\"\n• \"Review my code for bugs and security issues\"\n• \"Summarize documents and extract key points\"",
    },
  ]);
  const [inputValue, setInputValue] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Handle send message
  const handleSend = () => {
    if (!inputValue.trim()) return;

    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: inputValue,
    };

    setMessages((prev) => [...prev, userMessage]);
    const userInput = inputValue;
    setInputValue('');
    setIsTyping(true);

    // Simulate AI building recipe
    setTimeout(() => {
      let response: string;
      let newRecipe: RecipeData;

      if (!recipeVisible) {
        // First message - create recipe
        if (userInput.toLowerCase().includes('email')) {
          newRecipe = {
            title: 'Email Writing Assistant',
            description: 'Helps write professional emails quickly',
            instructions:
              'You are a professional email writing assistant.\n\n1. Ask what the email is about\n2. Ask who the recipient is\n3. Use professional but friendly tone\n4. Keep emails concise (under 200 words)\n5. Offer to adjust tone or length',
            extensions: ['developer'],
            parameters: [],
          };
          response =
            "I've created an **Email Writing Assistant** for you! ✨\n\nCheck out the recipe card that just appeared. You can:\n• Click any section to expand and edit\n• Click the title or description to rename\n• Test it to see how it works\n\nWant me to add anything? Maybe a parameter for tone preference?";
        } else if (
          userInput.toLowerCase().includes('code') ||
          userInput.toLowerCase().includes('review')
        ) {
          newRecipe = {
            title: 'Code Review Assistant',
            description: 'Reviews code and suggests improvements',
            instructions:
              'You are a thorough code reviewer.\n\n1. Analyze code structure and logic\n2. Check for bugs and edge cases\n3. Look for security vulnerabilities\n4. Suggest performance improvements\n5. Recommend best practices',
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
            "I've built a **Code Review Assistant**! ✨\n\nI added:\n• Developer tools extension\n• Optional language parameter\n\nExpand the sections in the recipe card to see details or make changes. Ready to test it?";
        } else if (userInput.toLowerCase().includes('summar')) {
          newRecipe = {
            title: 'Document Summarizer',
            description: 'Summarizes documents and extracts key points',
            instructions:
              'You are an expert at summarizing content.\n\n1. Read the document carefully\n2. Identify main themes and key points\n3. Create a concise summary (3-5 bullet points)\n4. Highlight any action items\n5. Note important dates or deadlines',
            extensions: ['google_drive'],
            parameters: [
              {
                name: 'length',
                description: 'Summary length (short, medium, detailed)',
                type: 'string',
                required: false,
              },
            ],
          };
          response =
            "Created a **Document Summarizer** for you! ✨\n\nIt includes:\n• Google Drive access for reading documents\n• Length parameter for summary size\n\nClick any section to customize. Or test it right away!";
        } else {
          newRecipe = {
            title: 'Custom Assistant',
            description: userInput.slice(0, 60),
            instructions: `You are an assistant that helps with: ${userInput}\n\nBe helpful, clear, and thorough in your responses.`,
            extensions: [],
            parameters: [],
          };
          response =
            "I've started building your recipe! ✨\n\nTell me more about:\n• What specific tasks should it handle?\n• Any tools it needs? (files, web, etc.)\n• Any inputs users should provide?\n\nOr expand the sections to edit directly.";
        }

        setRecipe(newRecipe);
        setRecipeVisible(true);
      } else {
        // Follow-up - refine recipe
        newRecipe = { ...recipe };

        if (
          userInput.toLowerCase().includes('formal') ||
          userInput.toLowerCase().includes('tone')
        ) {
          newRecipe.instructions += '\n\nAlways use a formal, professional tone.';
          response =
            "Done! I've added formal tone guidance to the instructions. ✓\n\nClick the Instructions section to see the update.";
        } else if (
          userInput.toLowerCase().includes('parameter') ||
          userInput.toLowerCase().includes('input')
        ) {
          newRecipe.parameters.push({
            name: 'custom_input',
            description: 'User-provided input',
            type: 'string',
            required: true,
          });
          response =
            "Added a new parameter! ✓\n\nExpand the Parameters section to customize it (name, description, type).";
        } else if (userInput.toLowerCase().includes('extension') || userInput.toLowerCase().includes('tool')) {
          response =
            "You can add extensions by expanding the Extensions section in the recipe card.\n\nAvailable tools:\n• developer - file and code operations\n• computeruse - browser control\n• google_drive - Google docs access\n• memory - persistent memory\n• tutorial - guided help";
        } else {
          newRecipe.instructions += `\n\n${userInput}`;
          response =
            "Got it! I've added that to the instructions. ✓\n\nThe recipe card shows all your changes. Ready to test?";
        }

        setRecipe(newRecipe);
      }

      setMessages((prev) => [
        ...prev,
        { id: (Date.now() + 1).toString(), role: 'assistant', content: response },
      ]);
      setIsTyping(false);
    }, 1200);
  };

  // Handlers
  const handleTest = () => {
    // Switch to test mode - show test panel instead of recipe card
    setTestMode(true);
    // Reset test messages to start fresh
    setTestMessages([]);
    // Add a message to the build chat
    setMessages((prev) => [
      ...prev,
      {
        id: Date.now().toString(),
        role: 'assistant',
        content:
          "Test mode activated! 🧪\n\nTry your recipe in the green panel on the right. If something doesn't work as expected, tell me here and I'll adjust the recipe for you.",
      },
    ]);
  };

  const handleBackFromTest = () => {
    setTestMode(false);
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
              <h1 className="text-lg font-semibold">Create Recipe</h1>
              <p className="text-sm text-gray-500">
                {recipeVisible ? 'Refine your recipe' : 'Describe what you want'}
              </p>
            </div>
          </div>
          {recipeVisible && (
            <div className="flex items-center gap-2">
              <button
                onClick={handleSave}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md flex items-center gap-2 transition-colors"
              >
                <Save className="w-4 h-4" />
                Save Recipe
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex min-h-0">
        {/* Chat Panel */}
        <div
          className={`flex-1 flex flex-col bg-white dark:bg-gray-800 transition-all duration-500 ${
            recipeVisible ? '' : 'max-w-3xl mx-auto'
          }`}
        >
          {/* Messages */}
          <div className="flex-1 overflow-y-auto p-6 space-y-4">
            {messages.map((msg) => (
              <div
                key={msg.id}
                className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
              >
                <div
                  className={`max-w-[80%] rounded-lg px-4 py-3 ${
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
                <div className="bg-gray-100 dark:bg-gray-700 rounded-lg px-4 py-3">
                  <div className="flex items-center gap-2">
                    <Sparkles className="w-4 h-4 text-yellow-500 animate-pulse" />
                    <span className="text-sm text-gray-500">Building your recipe...</span>
                  </div>
                </div>
              </div>
            )}
            <div ref={messagesEndRef} />
          </div>

          {/* Input */}
          <div className="p-4 border-t border-gray-200 dark:border-gray-700">
            <div className="flex flex-col gap-2 max-w-3xl mx-auto">
              <div className="flex gap-2">
                <input
                  type="text"
                  value={inputValue}
                  onChange={(e) => setInputValue(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && handleSend()}
                  placeholder={
                    recipeVisible
                      ? "Ask me to help improve your recipe..."
                      : "Describe what your recipe should do..."
                  }
                  className="flex-1 px-4 py-3 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <button
                  onClick={handleSend}
                  disabled={!inputValue.trim()}
                  className="px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                >
                  <Send className="w-5 h-5" />
                </button>
              </div>
              {/* Edit directly link - only show when recipe card is not visible */}
              {!recipeVisible && (
                <div className="text-center">
                  <button
                    onClick={() => {
                      // Show empty recipe card for power users
                      setRecipe({
                        title: '',
                        description: '',
                        instructions: '',
                        extensions: [],
                        parameters: [],
                      });
                      setRecipeVisible(true);
                      // Update chat to assistant mode
                      setMessages((prev) => [
                        ...prev,
                        {
                          id: Date.now().toString(),
                          role: 'assistant',
                          content:
                            "No problem! I've opened the recipe form for you. ✏️\n\nFill in the fields directly, and I'm here if you need help:\n• \"Review my instructions\"\n• \"Suggest extensions for file operations\"\n• \"Make this more concise\"",
                        },
                      ]);
                    }}
                    className="text-sm text-gray-500 hover:text-blue-600 transition-colors"
                  >
                    Prefer to build it yourself? <span className="underline">Edit directly →</span>
                  </button>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Right Panel: Recipe Card or Test Panel */}
        {testMode ? (
          <TestPanel
            recipe={recipe}
            messages={testMessages}
            setMessages={setTestMessages}
            onBack={handleBackFromTest}
          />
        ) : (
          <RecipeCard
            recipe={recipe}
            setRecipe={setRecipe}
            onTest={handleTest}
            onSave={handleSave}
            isVisible={recipeVisible}
          />
        )}
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

export function Phase3Progressive_Demo() {
  return <Phase3_Progressive />;
}
