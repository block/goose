/**
 * Phase 3+4 Combined: Build & Test in One Screen
 *
 * Layout:
 * - Left: Form (always visible, always editable)
 * - Right: Tabbed panel [AI Help] [Test]
 *
 * Benefits:
 * - No screen switching
 * - Edit → Test → Edit flow is instant
 * - Form is always the source of truth
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
  Save,
  Play,
  RefreshCw,
  MessageSquare,
  Beaker,
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
  role: 'user' | 'assistant' | 'system';
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
  if (message.role === 'system') {
    return (
      <div className="flex justify-center">
        <div className="px-3 py-1 bg-gray-100 dark:bg-gray-800 rounded-full text-xs text-gray-500">
          {message.content}
        </div>
      </div>
    );
  }

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

// Tab button component
function TabButton({
  active,
  onClick,
  icon: Icon,
  label,
}: {
  active: boolean;
  onClick: () => void;
  icon: React.ElementType;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-2 px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
        active
          ? 'border-blue-600 text-blue-600'
          : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
      }`}
    >
      <Icon className="w-4 h-4" />
      {label}
    </button>
  );
}

// Main component
export default function Phase3_BuildAndTest() {
  const navigate = useNavigate();
  const [form, setForm] = useState<RecipeForm>(emptyForm);
  const [activeTab, setActiveTab] = useState<'ai' | 'test'>('ai');

  // AI Help chat state
  const [aiMessages, setAiMessages] = useState<ChatMessage[]>([
    {
      id: '1',
      role: 'assistant',
      content:
        "Hi! I'll help you create a recipe. What would you like your recipe to do? Describe it in your own words.",
    },
  ]);
  const [aiInput, setAiInput] = useState('');
  const [aiTyping, setAiTyping] = useState(false);

  // Test chat state
  const [testMessages, setTestMessages] = useState<ChatMessage[]>([
    {
      id: 'system-1',
      role: 'system',
      content: 'Test conversation - try your recipe here',
    },
  ]);
  const [testInput, setTestInput] = useState('');
  const [testTyping, setTestTyping] = useState(false);

  const aiMessagesEndRef = useRef<HTMLDivElement>(null);
  const testMessagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll
  useEffect(() => {
    if (activeTab === 'ai') {
      aiMessagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    } else {
      testMessagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [aiMessages, testMessages, activeTab]);

  // Generate test welcome message based on recipe
  useEffect(() => {
    if (form.title && form.instructions) {
      const welcomeMsg = form.instructions.includes('email')
        ? "Hi! I'm your Email Writing Assistant. What email would you like help with today?"
        : form.instructions.includes('code')
        ? "Hi! I'm ready to help with your code. What would you like me to review or explain?"
        : `Hi! I'm your ${form.title || 'assistant'}. How can I help you today?`;

      setTestMessages([
        {
          id: 'system-1',
          role: 'system',
          content: 'Test conversation - try your recipe here',
        },
        {
          id: 'welcome',
          role: 'assistant',
          content: welcomeMsg,
        },
      ]);
    }
  }, [form.title, form.instructions]);

  // Update form field
  const updateForm = (field: keyof RecipeForm, value: string | string[] | RecipeForm['parameters']) => {
    setForm((prev) => ({ ...prev, [field]: value }));
  };

  // Apply AI suggestion
  const handleApplySuggestion = (field: keyof RecipeForm, value: string | string[]) => {
    updateForm(field, value);
  };

  // AI chat response simulation
  const simulateAIResponse = (userMessage: string) => {
    setAiTyping(true);

    setTimeout(() => {
      let response: ChatMessage;

      if (userMessage.toLowerCase().includes('email')) {
        response = {
          id: Date.now().toString(),
          role: 'assistant',
          content: 'Great! An email assistant. Here\'s a suggested title:',
          suggestion: {
            field: 'title',
            value: 'Email Writing Assistant',
          },
        };
      } else if (userMessage.toLowerCase().includes('code')) {
        response = {
          id: Date.now().toString(),
          role: 'assistant',
          content: 'A code helper! Here\'s a suggested title:',
          suggestion: {
            field: 'title',
            value: 'Code Review Assistant',
          },
        };
      } else if (form.title && !form.instructions) {
        response = {
          id: Date.now().toString(),
          role: 'assistant',
          content: 'Now let\'s add instructions. Here\'s a suggestion:',
          suggestion: {
            field: 'instructions',
            value: `You are a helpful ${form.title.toLowerCase()}.\n\nRules:\n1. Be professional and friendly\n2. Ask clarifying questions when needed\n3. Provide clear, actionable responses`,
          },
        };
      } else {
        response = {
          id: Date.now().toString(),
          role: 'assistant',
          content: 'Got it! You can edit the form directly or describe more details. When ready, switch to the Test tab to try it out.',
        };
      }

      setAiMessages((prev) => [...prev, response]);
      setAiTyping(false);
    }, 1000);
  };

  // Test chat response simulation
  const simulateTestResponse = (userMessage: string) => {
    setTestTyping(true);

    setTimeout(() => {
      let response: string;

      if (userMessage.toLowerCase().includes('meeting')) {
        response = `Got it! I'll help you write an email about a meeting.\n\nA few questions:\n• Who is this email to?\n• What's the preferred meeting time?\n• Any specific agenda items?`;
      } else if (userMessage.toLowerCase().includes('thank')) {
        response = `Here's a professional thank-you email:\n\n---\nSubject: Thank You\n\nDear [Recipient],\n\nThank you for your time today. I appreciated the opportunity to discuss [topic].\n\nBest regards,\n[Your name]\n---\n\nWould you like me to adjust anything?`;
      } else {
        response = `I understand. To help you best, could you tell me:\n• What's the main goal?\n• Who is the audience?\n• Any specific requirements?`;
      }

      setTestMessages((prev) => [
        ...prev,
        {
          id: Date.now().toString(),
          role: 'assistant',
          content: response,
        },
      ]);
      setTestTyping(false);
    }, 1200);
  };

  // Send AI message
  const handleAISend = () => {
    if (!aiInput.trim()) return;

    setAiMessages((prev) => [
      ...prev,
      { id: Date.now().toString(), role: 'user', content: aiInput },
    ]);
    const msg = aiInput;
    setAiInput('');
    simulateAIResponse(msg);
  };

  // Send test message
  const handleTestSend = () => {
    if (!testInput.trim()) return;

    setTestMessages((prev) => [
      ...prev,
      { id: Date.now().toString(), role: 'user', content: testInput },
    ]);
    const msg = testInput;
    setTestInput('');
    simulateTestResponse(msg);
  };

  // Reset test chat
  const handleResetTest = () => {
    const welcomeMsg = form.title
      ? `Hi! I'm your ${form.title}. How can I help you today?`
      : "Hi! How can I help you today?";

    setTestMessages([
      { id: 'system-1', role: 'system', content: 'Test conversation reset' },
      { id: 'welcome', role: 'assistant', content: welcomeMsg },
    ]);
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

  // Save handlers
  const handleSave = () => {
    if (!form.title) {
      alert('Please add a title first');
      return;
    }
    alert('Recipe saved! Going to Recipes list...');
    navigate('/recipes');
  };

  const handleSaveAndRun = () => {
    if (!form.title) {
      alert('Please add a title first');
      return;
    }
    alert('Recipe saved! Starting new chat with this recipe...');
    navigate('/recipes');
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
              <p className="text-sm text-gray-500">Build and test your recipe</p>
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
              onClick={handleSave}
              className="px-4 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md flex items-center gap-2"
            >
              <Save className="w-4 h-4" />
              Save
            </button>
            <button
              onClick={handleSaveAndRun}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md flex items-center gap-2"
            >
              <Play className="w-4 h-4" />
              Save & Run
            </button>
          </div>
        </div>
      </div>

      {/* Main content */}
      <div className="flex-1 flex min-h-0">
        {/* Left: Form (50%) */}
        <div className="w-1/2 overflow-y-auto p-6 border-r border-gray-200 dark:border-gray-700">
          <div className="max-w-xl space-y-4">
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
                  placeholder="Write instructions for the AI..."
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
                  Inputs users provide when running the recipe
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
          </div>
        </div>

        {/* Right: Tabbed Chat Panel (50%) */}
        <div className="w-1/2 flex flex-col bg-white dark:bg-gray-800">
          {/* Tabs */}
          <div className="flex border-b border-gray-200 dark:border-gray-700 px-4">
            <TabButton
              active={activeTab === 'ai'}
              onClick={() => setActiveTab('ai')}
              icon={MessageSquare}
              label="AI Help"
            />
            <TabButton
              active={activeTab === 'test'}
              onClick={() => setActiveTab('test')}
              icon={Beaker}
              label="Test"
            />
            {activeTab === 'test' && (
              <div className="ml-auto flex items-center">
                <button
                  onClick={handleResetTest}
                  className="px-3 py-1.5 text-xs text-gray-500 hover:text-gray-700 flex items-center gap-1"
                >
                  <RefreshCw className="w-3 h-3" />
                  Reset
                </button>
              </div>
            )}
          </div>

          {/* AI Help Tab Content */}
          {activeTab === 'ai' && (
            <>
              <div className="flex-1 overflow-y-auto p-4 space-y-4">
                {aiMessages.map((msg) => (
                  <ChatMessageBubble
                    key={msg.id}
                    message={msg}
                    onApplySuggestion={handleApplySuggestion}
                  />
                ))}
                {aiTyping && (
                  <div className="flex justify-start">
                    <div className="bg-gray-100 dark:bg-gray-700 rounded-lg px-4 py-2">
                      <span className="text-sm text-gray-500">Typing...</span>
                    </div>
                  </div>
                )}
                <div ref={aiMessagesEndRef} />
              </div>

              <div className="p-4 border-t border-gray-200 dark:border-gray-700">
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={aiInput}
                    onChange={(e) => setAiInput(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleAISend()}
                    placeholder="Describe your recipe..."
                    className="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700"
                  />
                  <button
                    onClick={handleAISend}
                    disabled={!aiInput.trim()}
                    className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg disabled:opacity-50"
                  >
                    <Send className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </>
          )}

          {/* Test Tab Content */}
          {activeTab === 'test' && (
            <>
              <div className="flex-1 overflow-y-auto p-4 space-y-4 bg-gray-50 dark:bg-gray-900">
                {testMessages.map((msg) => (
                  <ChatMessageBubble key={msg.id} message={msg} />
                ))}
                {testTyping && (
                  <div className="flex justify-start">
                    <div className="bg-gray-100 dark:bg-gray-700 rounded-lg px-4 py-2">
                      <span className="text-sm text-gray-500">Typing...</span>
                    </div>
                  </div>
                )}
                <div ref={testMessagesEndRef} />
              </div>

              <div className="p-4 border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={testInput}
                    onChange={(e) => setTestInput(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleTestSend()}
                    placeholder="Test your recipe..."
                    className="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700"
                  />
                  <button
                    onClick={handleTestSend}
                    disabled={!testInput.trim()}
                    className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg disabled:opacity-50"
                  >
                    <Send className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </>
          )}
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

export function Phase3Combined_Demo() {
  return <Phase3_BuildAndTest />;
}
