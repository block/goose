/**
 * Phase 3: Build - Recipe Builder with Chat + Form
 *
 * The core recipe creation experience:
 * - Left side: Chat with AI to describe what you want
 * - Right side: Form that shows/edits the recipe
 *
 * Key principles:
 * - Form is always the source of truth
 * - Chat suggestions have [Apply] buttons
 * - User can edit form directly at any time
 * - No mode switching - both always available
 */

import React, { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Send,
  Sparkles,
  ChevronDown,
  ChevronRight,
  Plus,
  X,
  ArrowRight,
} from 'lucide-react';

// Types
interface RecipeForm {
  title: string;
  description: string;
  instructions: string;
  activities: string[];
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
  suggestion?: {
    field: keyof RecipeForm;
    value: string | string[];
  };
}

// Initial empty form
const emptyForm: RecipeForm = {
  title: '',
  description: '',
  instructions: '',
  activities: [],
  parameters: [],
};

// Collapsible section component
function CollapsibleSection({
  title,
  children,
  defaultOpen = true,
  badge,
}: {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
  badge?: string | number;
}) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="w-full px-4 py-3 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-800 rounded-t-lg"
      >
        <div className="flex items-center gap-2">
          {isOpen ? (
            <ChevronDown className="w-4 h-4 text-gray-400" />
          ) : (
            <ChevronRight className="w-4 h-4 text-gray-400" />
          )}
          <span className="font-medium">{title}</span>
          {badge !== undefined && (
            <span className="px-2 py-0.5 text-xs bg-gray-100 dark:bg-gray-700 rounded-full">
              {badge}
            </span>
          )}
        </div>
      </button>
      {isOpen && <div className="px-4 pb-4">{children}</div>}
    </div>
  );
}

// Chat message component
function ChatMessageBubble({
  message,
  onApplySuggestion,
}: {
  message: ChatMessage;
  onApplySuggestion?: (field: keyof RecipeForm, value: string | string[]) => void;
}) {
  const isUser = message.role === 'user';

  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`max-w-[85%] rounded-lg px-4 py-2 ${
          isUser
            ? 'bg-blue-600 text-white'
            : 'bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-gray-100'
        }`}
      >
        <p className="text-sm whitespace-pre-wrap">{message.content}</p>

        {/* Suggestion with Apply button */}
        {message.suggestion && onApplySuggestion && (
          <div className="mt-3 pt-3 border-t border-gray-300 dark:border-gray-600">
            <div className="flex items-center justify-between">
              <span className="text-xs text-gray-500 dark:text-gray-400">
                Suggested {message.suggestion.field}
              </span>
              <button
                onClick={() =>
                  onApplySuggestion(message.suggestion!.field, message.suggestion!.value)
                }
                className="px-2 py-1 text-xs bg-blue-500 hover:bg-blue-600 text-white rounded flex items-center gap-1"
              >
                Apply <ArrowRight className="w-3 h-3" />
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// Main component
export default function Phase3_RecipeBuilder() {
  const navigate = useNavigate();
  const [form, setForm] = useState<RecipeForm>(emptyForm);
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: '1',
      role: 'assistant',
      content:
        "Hi! I'll help you create a recipe. What would you like your recipe to do? Describe it in your own words.",
    },
  ]);
  const [inputValue, setInputValue] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll chat
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Update form field
  const updateForm = (field: keyof RecipeForm, value: string | string[] | RecipeForm['parameters']) => {
    setForm((prev) => ({ ...prev, [field]: value }));
  };

  // Apply AI suggestion to form
  const handleApplySuggestion = (field: keyof RecipeForm, value: string | string[]) => {
    updateForm(field, value);
  };

  // Simulate AI response
  const simulateAIResponse = (userMessage: string) => {
    setIsTyping(true);

    setTimeout(() => {
      let response: ChatMessage;

      // Simple keyword-based responses for demo
      if (userMessage.toLowerCase().includes('email')) {
        response = {
          id: Date.now().toString(),
          role: 'assistant',
          content:
            'Great! An email assistant sounds useful. I suggest this title and description:',
          suggestion: {
            field: 'title',
            value: 'Email Writing Assistant',
          },
        };
      } else if (userMessage.toLowerCase().includes('code') || userMessage.toLowerCase().includes('review')) {
        response = {
          id: Date.now().toString(),
          role: 'assistant',
          content:
            "A code review helper! Here's a suggested title. Click Apply to add it to your recipe:",
          suggestion: {
            field: 'title',
            value: 'Code Review Assistant',
          },
        };
      } else if (form.title && !form.instructions) {
        response = {
          id: Date.now().toString(),
          role: 'assistant',
          content:
            "Now let's add some instructions. These tell the AI how to behave. Here's a suggestion based on what you described:",
          suggestion: {
            field: 'instructions',
            value: `You are a helpful ${form.title.toLowerCase()}. Follow these rules:\n\n1. Be professional and friendly\n2. Ask clarifying questions when needed\n3. Provide clear, actionable responses`,
          },
        };
      } else {
        response = {
          id: Date.now().toString(),
          role: 'assistant',
          content:
            "Got it! You can describe more details, or start filling in the form on the right. I'm here to help if you need suggestions.",
        };
      }

      setMessages((prev) => [...prev, response]);
      setIsTyping(false);
    }, 1000);
  };

  // Send message
  const handleSend = () => {
    if (!inputValue.trim()) return;

    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: inputValue,
    };

    setMessages((prev) => [...prev, userMessage]);
    setInputValue('');
    simulateAIResponse(inputValue);
  };

  // Add parameter
  const addParameter = () => {
    setForm((prev) => ({
      ...prev,
      parameters: [
        ...prev.parameters,
        { name: '', description: '', type: 'string', required: false },
      ],
    }));
  };

  // Remove parameter
  const removeParameter = (index: number) => {
    setForm((prev) => ({
      ...prev,
      parameters: prev.parameters.filter((_, i) => i !== index),
    }));
  };

  // Update parameter
  const updateParameter = (index: number, field: string, value: string | boolean) => {
    setForm((prev) => ({
      ...prev,
      parameters: prev.parameters.map((p, i) =>
        i === index ? { ...p, [field]: value } : p
      ),
    }));
  };

  // Add activity
  const addActivity = () => {
    setForm((prev) => ({
      ...prev,
      activities: [...prev.activities, ''],
    }));
  };

  // Handle navigation to test
  const handleNext = () => {
    if (!form.title) {
      alert('Please add a title first');
      return;
    }
    alert('Navigating to Test phase (Phase 4)...');
    // navigate('/prototype-phase4', { state: { recipe: form } });
  };

  return (
    <div className="h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
      {/* Header */}
      <div className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Sparkles className="w-6 h-6 text-purple-500" />
            <div>
              <h1 className="text-lg font-semibold">Recipe Builder</h1>
              <p className="text-sm text-gray-500">Step 1 of 2: Build your recipe</p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={() => navigate('/recipes')}
              className="px-4 py-2 text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md"
            >
              Cancel
            </button>
            <button
              onClick={handleNext}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md flex items-center gap-2"
            >
              Next: Test
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      {/* Main content - Chat + Form side by side */}
      <div className="flex-1 flex min-h-0">
        {/* Left: Chat Panel (40%) */}
        <div className="w-2/5 border-r border-gray-200 dark:border-gray-700 flex flex-col bg-white dark:bg-gray-800">
          <div className="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
            <h2 className="font-medium">Chat with AI</h2>
            <p className="text-xs text-gray-500">Describe what you want, AI will help build it</p>
          </div>

          {/* Messages */}
          <div className="flex-1 overflow-y-auto p-4 space-y-4">
            {messages.map((msg) => (
              <ChatMessageBubble
                key={msg.id}
                message={msg}
                onApplySuggestion={handleApplySuggestion}
              />
            ))}
            {isTyping && (
              <div className="flex justify-start">
                <div className="bg-gray-100 dark:bg-gray-700 rounded-lg px-4 py-2">
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
                placeholder="Describe your recipe..."
                className="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700"
              />
              <button
                onClick={handleSend}
                disabled={!inputValue.trim()}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg disabled:opacity-50"
              >
                <Send className="w-4 h-4" />
              </button>
            </div>
          </div>
        </div>

        {/* Right: Form Panel (60%) */}
        <div className="w-3/5 overflow-y-auto p-6">
          <div className="max-w-2xl space-y-4">
            {/* Basic Info */}
            <CollapsibleSection title="Basic Info" defaultOpen={true}>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium mb-1">
                    Title <span className="text-red-500">*</span>
                  </label>
                  <input
                    type="text"
                    value={form.title}
                    onChange={(e) => updateForm('title', e.target.value)}
                    placeholder="Give your recipe a name"
                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium mb-1">Description</label>
                  <textarea
                    value={form.description}
                    onChange={(e) => updateForm('description', e.target.value)}
                    placeholder="What does this recipe do?"
                    rows={2}
                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800"
                  />
                </div>
              </div>
            </CollapsibleSection>

            {/* Instructions */}
            <CollapsibleSection title="Instructions" defaultOpen={true}>
              <div>
                <label className="block text-sm font-medium mb-1">
                  Behavior Rules
                  <span className="text-xs text-gray-400 ml-2">How should the AI behave?</span>
                </label>
                <textarea
                  value={form.instructions}
                  onChange={(e) => updateForm('instructions', e.target.value)}
                  placeholder="Write instructions for the AI...&#10;&#10;Example:&#10;- Always ask clarifying questions first&#10;- Keep responses concise&#10;- Use professional tone"
                  rows={6}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 font-mono text-sm"
                />
              </div>
            </CollapsibleSection>

            {/* Parameters */}
            <CollapsibleSection
              title="Parameters"
              defaultOpen={form.parameters.length > 0}
              badge={form.parameters.length || undefined}
            >
              <div className="space-y-3">
                <p className="text-sm text-gray-500">
                  Parameters are inputs users provide when running the recipe (e.g., file path, topic)
                </p>

                {form.parameters.map((param, index) => (
                  <div
                    key={index}
                    className="p-3 bg-gray-50 dark:bg-gray-700 rounded-lg space-y-2"
                  >
                    <div className="flex items-center justify-between">
                      <input
                        type="text"
                        value={param.name}
                        onChange={(e) => updateParameter(index, 'name', e.target.value)}
                        placeholder="Parameter name"
                        className="flex-1 px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-800"
                      />
                      <button
                        onClick={() => removeParameter(index)}
                        className="ml-2 p-1 text-gray-400 hover:text-red-500"
                      >
                        <X className="w-4 h-4" />
                      </button>
                    </div>
                    <input
                      type="text"
                      value={param.description}
                      onChange={(e) => updateParameter(index, 'description', e.target.value)}
                      placeholder="Description"
                      className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-800"
                    />
                    <div className="flex items-center gap-4">
                      <select
                        value={param.type}
                        onChange={(e) => updateParameter(index, 'type', e.target.value)}
                        className="px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-800"
                      >
                        <option value="string">Text</option>
                        <option value="number">Number</option>
                        <option value="boolean">Yes/No</option>
                        <option value="file">File</option>
                      </select>
                      <label className="flex items-center gap-2 text-sm">
                        <input
                          type="checkbox"
                          checked={param.required}
                          onChange={(e) => updateParameter(index, 'required', e.target.checked)}
                        />
                        Required
                      </label>
                    </div>
                  </div>
                ))}

                <button
                  onClick={addParameter}
                  className="w-full py-2 border border-dashed border-gray-300 dark:border-gray-600 rounded-lg text-sm text-gray-500 hover:border-blue-500 hover:text-blue-500 flex items-center justify-center gap-2"
                >
                  <Plus className="w-4 h-4" />
                  Add Parameter
                </button>
              </div>
            </CollapsibleSection>

            {/* Activities (Capabilities) */}
            <CollapsibleSection
              title="Capabilities"
              defaultOpen={false}
              badge={form.activities.length || undefined}
            >
              <div className="space-y-3">
                <p className="text-sm text-gray-500">
                  What tools/capabilities should this recipe have access to?
                </p>

                {form.activities.map((activity, index) => (
                  <div key={index} className="flex items-center gap-2">
                    <input
                      type="text"
                      value={activity}
                      onChange={(e) => {
                        const newActivities = [...form.activities];
                        newActivities[index] = e.target.value;
                        updateForm('activities', newActivities);
                      }}
                      placeholder="e.g., read_file, write_file, browse_web"
                      className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md text-sm bg-white dark:bg-gray-800"
                    />
                    <button
                      onClick={() => {
                        const newActivities = form.activities.filter((_, i) => i !== index);
                        updateForm('activities', newActivities);
                      }}
                      className="p-2 text-gray-400 hover:text-red-500"
                    >
                      <X className="w-4 h-4" />
                    </button>
                  </div>
                ))}

                <button
                  onClick={addActivity}
                  className="w-full py-2 border border-dashed border-gray-300 dark:border-gray-600 rounded-lg text-sm text-gray-500 hover:border-blue-500 hover:text-blue-500 flex items-center justify-center gap-2"
                >
                  <Plus className="w-4 h-4" />
                  Add Capability
                </button>
              </div>
            </CollapsibleSection>
          </div>
        </div>
      </div>

      {/* Back button */}
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

export function Phase3_Demo() {
  return <Phase3_RecipeBuilder />;
}
