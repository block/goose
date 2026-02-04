/**
 * Phase 3 (v2): Chat-First Recipe Builder
 *
 * Two modes:
 * 1. Chat Mode: Chat + Test (recipe shown as card in chat)
 * 2. Edit Mode: Form + Test (for advanced config)
 *
 * Key features:
 * - Chat is primary input method
 * - AI shows recipe as structured card in chat
 * - AI auto-detects extensions and parameters
 * - [Edit] switches to form mode
 * - [Test] available in both modes
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
  Save,
  Play,
  RefreshCw,
  MessageSquare,
  Edit,
  FileText,
  Zap,
  Settings,
  ArrowLeft,
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
    required: boolean;
  }>;
}

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  recipeCard?: RecipeData; // Recipe shown as card
}

interface TestMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
}

// Empty recipe
const emptyRecipe: RecipeData = {
  title: '',
  description: '',
  instructions: '',
  extensions: [],
  parameters: [],
};

// Recipe Card component (shown in chat)
function RecipeCard({
  recipe,
  onEdit,
  onTest,
}: {
  recipe: RecipeData;
  onEdit: () => void;
  onTest: () => void;
}) {
  const [expanded, setExpanded] = useState(true);

  return (
    <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-600 rounded-lg overflow-hidden mt-2">
      {/* Header */}
      <div
        className="px-4 py-3 bg-gradient-to-r from-purple-50 to-blue-50 dark:from-purple-900/20 dark:to-blue-900/20 border-b border-gray-200 dark:border-gray-600 cursor-pointer flex items-center justify-between"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center gap-2">
          <Sparkles className="w-4 h-4 text-purple-500" />
          <span className="font-medium">{recipe.title || 'Untitled Recipe'}</span>
        </div>
        {expanded ? (
          <ChevronDown className="w-4 h-4 text-gray-400" />
        ) : (
          <ChevronRight className="w-4 h-4 text-gray-400" />
        )}
      </div>

      {/* Content */}
      {expanded && (
        <div className="p-4 space-y-3">
          {/* Description */}
          {recipe.description && (
            <p className="text-sm text-gray-600 dark:text-gray-300">{recipe.description}</p>
          )}

          {/* Instructions */}
          <div>
            <div className="flex items-center gap-1 text-xs text-gray-500 mb-1">
              <FileText className="w-3 h-3" />
              Instructions
            </div>
            <div className="text-sm bg-gray-50 dark:bg-gray-700 rounded p-2 max-h-24 overflow-y-auto">
              <pre className="whitespace-pre-wrap font-sans">{recipe.instructions}</pre>
            </div>
          </div>

          {/* Extensions */}
          {recipe.extensions.length > 0 && (
            <div>
              <div className="flex items-center gap-1 text-xs text-gray-500 mb-1">
                <Zap className="w-3 h-3" />
                Capabilities
              </div>
              <div className="flex flex-wrap gap-1">
                {recipe.extensions.map((ext, i) => (
                  <span
                    key={i}
                    className="px-2 py-0.5 text-xs bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded"
                  >
                    {ext}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Parameters */}
          {recipe.parameters.length > 0 && (
            <div>
              <div className="flex items-center gap-1 text-xs text-gray-500 mb-1">
                <Settings className="w-3 h-3" />
                Parameters
              </div>
              <div className="space-y-1">
                {recipe.parameters.map((param, i) => (
                  <div key={i} className="text-sm flex items-center gap-2">
                    <code className="text-blue-600 dark:text-blue-400">{`{{${param.name}}}`}</code>
                    <span className="text-gray-500">- {param.description}</span>
                    {param.required && (
                      <span className="text-xs text-red-500">(required)</span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Actions */}
          <div className="flex gap-2 pt-2 border-t border-gray-200 dark:border-gray-600">
            <button
              onClick={onEdit}
              className="flex-1 px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-50 dark:hover:bg-gray-700 flex items-center justify-center gap-2"
            >
              <Edit className="w-4 h-4" />
              Edit
            </button>
            <button
              onClick={onTest}
              className="flex-1 px-3 py-2 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded-md flex items-center justify-center gap-2"
            >
              <Play className="w-4 h-4" />
              Test
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// Collapsible section for edit mode
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

// Main component
export default function Phase3_ChatFirst() {
  const navigate = useNavigate();
  const [mode, setMode] = useState<'chat' | 'edit'>('chat');
  const [recipe, setRecipe] = useState<RecipeData>(emptyRecipe);

  // Chat state
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: '1',
      role: 'assistant',
      content:
        "Hi! I'll help you create a recipe. What would you like it to do?\n\nJust describe it naturally - I'll figure out the details.",
    },
  ]);
  const [chatInput, setChatInput] = useState('');
  const [isTyping, setIsTyping] = useState(false);

  // Test state
  const [testMessages, setTestMessages] = useState<TestMessage[]>([]);
  const [testInput, setTestInput] = useState('');
  const [testTyping, setTestTyping] = useState(false);
  const [showTest, setShowTest] = useState(false);

  const chatEndRef = useRef<HTMLDivElement>(null);
  const testEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll
  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  useEffect(() => {
    testEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [testMessages]);

  // Simulate AI creating recipe from description
  const createRecipeFromChat = (userMessage: string) => {
    setIsTyping(true);

    setTimeout(() => {
      let newRecipe: RecipeData;
      let responseText: string;

      // Detect intent and create appropriate recipe
      if (userMessage.toLowerCase().includes('email')) {
        newRecipe = {
          title: 'Email Writing Assistant',
          description: 'Helps write professional emails quickly and effectively',
          instructions: `You are a professional email writing assistant.

Rules:
1. Always ask what the email is about first
2. Ask who the recipient is (colleague, manager, client)
3. Use professional but friendly tone
4. Keep emails concise (under 200 words)
5. No emojis unless explicitly requested`,
          extensions: ['clipboard'], // auto-detected
          parameters: [
            { name: 'tone', description: 'Preferred tone (formal/casual)', required: false },
          ],
        };
        responseText = "I created an Email Writing Assistant for you. I added clipboard access so it can copy emails directly. Want to test it or make changes?";
      } else if (userMessage.toLowerCase().includes('code') || userMessage.toLowerCase().includes('review')) {
        newRecipe = {
          title: 'Code Review Assistant',
          description: 'Reviews code and suggests improvements',
          instructions: `You are a code review assistant.

Rules:
1. Ask for the code or file to review
2. Check for bugs, security issues, and best practices
3. Suggest improvements with explanations
4. Be constructive, not critical`,
          extensions: ['file_read', 'developer'], // auto-detected
          parameters: [
            { name: 'language', description: 'Programming language', required: false },
          ],
        };
        responseText = "I created a Code Review Assistant. I added file reading and developer tools since you'll be working with code. Test it out!";
      } else if (userMessage.toLowerCase().includes('csv') || userMessage.toLowerCase().includes('file') || userMessage.toLowerCase().includes('analyze')) {
        newRecipe = {
          title: 'Data Analyzer',
          description: 'Analyzes data files and provides insights',
          instructions: `You are a data analysis assistant.

Rules:
1. Ask for the file path or data to analyze
2. Identify patterns and key metrics
3. Provide clear summaries
4. Suggest visualizations when helpful`,
          extensions: ['file_read'], // auto-detected
          parameters: [
            { name: 'file_path', description: 'Path to the data file', required: true },
          ],
        };
        responseText = "I created a Data Analyzer. Since you mentioned files, I added file access and made file_path a required parameter. Ready to test?";
      } else {
        newRecipe = {
          title: 'Custom Assistant',
          description: userMessage.slice(0, 100),
          instructions: `You are a helpful assistant for: ${userMessage}

Rules:
1. Be helpful and clear
2. Ask clarifying questions when needed
3. Provide actionable responses`,
          extensions: [],
          parameters: [],
        };
        responseText = "I created a basic recipe based on your description. You can test it or click Edit to add more capabilities.";
      }

      setRecipe(newRecipe);

      const newMessage: ChatMessage = {
        id: Date.now().toString(),
        role: 'assistant',
        content: responseText,
        recipeCard: newRecipe,
      };

      setMessages((prev) => [...prev, newMessage]);
      setIsTyping(false);
    }, 1500);
  };

  // Handle chat send
  const handleChatSend = () => {
    if (!chatInput.trim()) return;

    const userMessage: ChatMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: chatInput,
    };

    setMessages((prev) => [...prev, userMessage]);
    const msg = chatInput;
    setChatInput('');

    // If recipe exists, treat as refinement
    if (recipe.title) {
      setIsTyping(true);
      setTimeout(() => {
        // Simple refinement simulation
        const updatedRecipe = { ...recipe };
        if (msg.toLowerCase().includes('add') && msg.toLowerCase().includes('parameter')) {
          updatedRecipe.parameters.push({
            name: 'custom_param',
            description: 'Custom parameter',
            required: false,
          });
        }
        setRecipe(updatedRecipe);

        setMessages((prev) => [
          ...prev,
          {
            id: Date.now().toString(),
            role: 'assistant',
            content: "I've updated the recipe. Here's the latest version:",
            recipeCard: updatedRecipe,
          },
        ]);
        setIsTyping(false);
      }, 1000);
    } else {
      createRecipeFromChat(msg);
    }
  };

  // Handle test
  const handleStartTest = () => {
    setShowTest(true);
    setTestMessages([
      { id: 'sys', role: 'system', content: 'Test conversation started' },
      {
        id: 'welcome',
        role: 'assistant',
        content: recipe.instructions.includes('email')
          ? "Hi! I'm your Email Writing Assistant. What email would you like help with today?"
          : recipe.instructions.includes('code')
          ? "Hi! I'm ready to review your code. Share the code or file path you'd like me to look at."
          : `Hi! I'm your ${recipe.title}. How can I help you today?`,
      },
    ]);
  };

  // Handle test send
  const handleTestSend = () => {
    if (!testInput.trim()) return;

    setTestMessages((prev) => [
      ...prev,
      { id: Date.now().toString(), role: 'user', content: testInput },
    ]);
    const msg = testInput;
    setTestInput('');
    setTestTyping(true);

    setTimeout(() => {
      let response = "I understand. Let me help you with that. Could you provide more details?";
      if (msg.toLowerCase().includes('meeting')) {
        response = "I'll help you write an email about a meeting. Who is the recipient and what's the meeting about?";
      } else if (msg.toLowerCase().includes('thank')) {
        response = "Here's a thank-you email draft:\n\n---\nSubject: Thank You\n\nDear [Name],\n\nThank you for your time today...\n---\n\nShould I adjust anything?";
      }

      setTestMessages((prev) => [
        ...prev,
        { id: Date.now().toString(), role: 'assistant', content: response },
      ]);
      setTestTyping(false);
    }, 1000);
  };

  // Reset test
  const handleResetTest = () => {
    handleStartTest();
  };

  // Update recipe field (edit mode)
  const updateRecipe = (field: keyof RecipeData, value: any) => {
    setRecipe((prev) => ({ ...prev, [field]: value }));
  };

  // Save handlers
  const handleSave = () => {
    if (!recipe.title) {
      alert('Please create a recipe first');
      return;
    }
    alert('Recipe saved!');
    navigate('/recipes');
  };

  const handleSaveAndRun = () => {
    if (!recipe.title) {
      alert('Please create a recipe first');
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
            <Sparkles className="w-6 h-6 text-purple-500" />
            <div>
              <h1 className="text-lg font-semibold">Recipe Builder</h1>
              <p className="text-sm text-gray-500">
                {mode === 'chat' ? 'Describe what you want' : 'Edit recipe details'}
              </p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            {mode === 'edit' && (
              <button
                onClick={() => setMode('chat')}
                className="px-4 py-2 text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md flex items-center gap-2"
              >
                <ArrowLeft className="w-4 h-4" />
                Back to Chat
              </button>
            )}
            <button
              onClick={() => navigate('/recipes')}
              className="px-4 py-2 text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md"
            >
              Cancel
            </button>
            <button
              onClick={handleSave}
              disabled={!recipe.title}
              className="px-4 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md flex items-center gap-2 disabled:opacity-50"
            >
              <Save className="w-4 h-4" />
              Save
            </button>
            <button
              onClick={handleSaveAndRun}
              disabled={!recipe.title}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md flex items-center gap-2 disabled:opacity-50"
            >
              <Play className="w-4 h-4" />
              Save & Run
            </button>
          </div>
        </div>
      </div>

      {/* Main content */}
      <div className="flex-1 flex min-h-0">
        {/* Left Panel: Chat or Edit */}
        <div className={`${showTest ? 'w-1/2' : 'w-full'} flex flex-col border-r border-gray-200 dark:border-gray-700`}>
          {mode === 'chat' ? (
            // Chat Mode
            <>
              <div className="flex-1 overflow-y-auto p-4 space-y-4">
                {messages.map((msg) => (
                  <div key={msg.id}>
                    <div className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                      <div
                        className={`max-w-[80%] rounded-lg px-4 py-2 ${
                          msg.role === 'user'
                            ? 'bg-blue-600 text-white'
                            : 'bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700'
                        }`}
                      >
                        <p className="text-sm whitespace-pre-wrap">{msg.content}</p>
                      </div>
                    </div>
                    {msg.recipeCard && (
                      <div className="mt-2 ml-0">
                        <RecipeCard
                          recipe={msg.recipeCard}
                          onEdit={() => setMode('edit')}
                          onTest={handleStartTest}
                        />
                      </div>
                    )}
                  </div>
                ))}
                {isTyping && (
                  <div className="flex justify-start">
                    <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg px-4 py-2">
                      <span className="text-sm text-gray-500">Creating recipe...</span>
                    </div>
                  </div>
                )}
                <div ref={chatEndRef} />
              </div>

              <div className="p-4 border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={chatInput}
                    onChange={(e) => setChatInput(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleChatSend()}
                    placeholder={recipe.title ? "Ask for changes..." : "Describe your recipe..."}
                    className="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700"
                  />
                  <button
                    onClick={handleChatSend}
                    disabled={!chatInput.trim()}
                    className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg disabled:opacity-50"
                  >
                    <Send className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </>
          ) : (
            // Edit Mode
            <div className="flex-1 overflow-y-auto p-6">
              <div className="max-w-xl space-y-4">
                <CollapsibleSection title="Basic Info">
                  <div className="space-y-4">
                    <div>
                      <label className="block text-sm font-medium mb-1">Title</label>
                      <input
                        type="text"
                        value={recipe.title}
                        onChange={(e) => updateRecipe('title', e.target.value)}
                        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800"
                      />
                    </div>
                    <div>
                      <label className="block text-sm font-medium mb-1">Description</label>
                      <textarea
                        value={recipe.description}
                        onChange={(e) => updateRecipe('description', e.target.value)}
                        rows={2}
                        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800"
                      />
                    </div>
                  </div>
                </CollapsibleSection>

                <CollapsibleSection title="Instructions">
                  <textarea
                    value={recipe.instructions}
                    onChange={(e) => updateRecipe('instructions', e.target.value)}
                    rows={8}
                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 font-mono text-sm"
                  />
                </CollapsibleSection>

                <CollapsibleSection
                  title="Capabilities"
                  badge={recipe.extensions.length || undefined}
                >
                  <div className="space-y-2">
                    {recipe.extensions.map((ext, i) => (
                      <div key={i} className="flex items-center gap-2">
                        <span className="flex-1 px-3 py-2 bg-gray-50 dark:bg-gray-700 rounded">
                          {ext}
                        </span>
                        <button
                          onClick={() =>
                            updateRecipe(
                              'extensions',
                              recipe.extensions.filter((_, idx) => idx !== i)
                            )
                          }
                          className="p-2 text-gray-400 hover:text-red-500"
                        >
                          <X className="w-4 h-4" />
                        </button>
                      </div>
                    ))}
                    <button
                      onClick={() =>
                        updateRecipe('extensions', [...recipe.extensions, 'new_extension'])
                      }
                      className="w-full py-2 border border-dashed border-gray-300 rounded-lg text-sm text-gray-500 hover:border-blue-500 hover:text-blue-500"
                    >
                      <Plus className="w-4 h-4 inline mr-1" />
                      Add Capability
                    </button>
                  </div>
                </CollapsibleSection>

                <CollapsibleSection
                  title="Parameters"
                  badge={recipe.parameters.length || undefined}
                >
                  <div className="space-y-2">
                    {recipe.parameters.map((param, i) => (
                      <div key={i} className="p-3 bg-gray-50 dark:bg-gray-700 rounded-lg space-y-2">
                        <div className="flex items-center gap-2">
                          <input
                            type="text"
                            value={param.name}
                            onChange={(e) => {
                              const newParams = [...recipe.parameters];
                              newParams[i].name = e.target.value;
                              updateRecipe('parameters', newParams);
                            }}
                            placeholder="Name"
                            className="flex-1 px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-800"
                          />
                          <button
                            onClick={() =>
                              updateRecipe(
                                'parameters',
                                recipe.parameters.filter((_, idx) => idx !== i)
                              )
                            }
                            className="p-1 text-gray-400 hover:text-red-500"
                          >
                            <X className="w-4 h-4" />
                          </button>
                        </div>
                        <input
                          type="text"
                          value={param.description}
                          onChange={(e) => {
                            const newParams = [...recipe.parameters];
                            newParams[i].description = e.target.value;
                            updateRecipe('parameters', newParams);
                          }}
                          placeholder="Description"
                          className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-800"
                        />
                      </div>
                    ))}
                    <button
                      onClick={() =>
                        updateRecipe('parameters', [
                          ...recipe.parameters,
                          { name: '', description: '', required: false },
                        ])
                      }
                      className="w-full py-2 border border-dashed border-gray-300 rounded-lg text-sm text-gray-500 hover:border-blue-500 hover:text-blue-500"
                    >
                      <Plus className="w-4 h-4 inline mr-1" />
                      Add Parameter
                    </button>
                  </div>
                </CollapsibleSection>

                {/* Test button in edit mode */}
                {!showTest && recipe.title && (
                  <button
                    onClick={handleStartTest}
                    className="w-full py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg flex items-center justify-center gap-2"
                  >
                    <Play className="w-4 h-4" />
                    Test Recipe
                  </button>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Right Panel: Test (when active) */}
        {showTest && (
          <div className="w-1/2 flex flex-col bg-white dark:bg-gray-800">
            <div className="px-4 py-3 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
              <div>
                <h2 className="font-medium">Test: {recipe.title}</h2>
                <p className="text-xs text-gray-500">Try your recipe</p>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={handleResetTest}
                  className="px-3 py-1.5 text-xs text-gray-500 hover:bg-gray-100 dark:hover:bg-gray-700 rounded flex items-center gap-1"
                >
                  <RefreshCw className="w-3 h-3" />
                  Reset
                </button>
                <button
                  onClick={() => setShowTest(false)}
                  className="p-1.5 text-gray-400 hover:text-gray-600"
                >
                  <X className="w-4 h-4" />
                </button>
              </div>
            </div>

            <div className="flex-1 overflow-y-auto p-4 space-y-4 bg-gray-50 dark:bg-gray-900">
              {testMessages.map((msg) =>
                msg.role === 'system' ? (
                  <div key={msg.id} className="flex justify-center">
                    <span className="px-3 py-1 bg-gray-200 dark:bg-gray-700 rounded-full text-xs text-gray-500">
                      {msg.content}
                    </span>
                  </div>
                ) : (
                  <div key={msg.id} className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                    <div
                      className={`max-w-[85%] rounded-lg px-4 py-2 ${
                        msg.role === 'user'
                          ? 'bg-blue-600 text-white'
                          : 'bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700'
                      }`}
                    >
                      <p className="text-sm whitespace-pre-wrap">{msg.content}</p>
                    </div>
                  </div>
                )
              )}
              {testTyping && (
                <div className="flex justify-start">
                  <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg px-4 py-2">
                    <span className="text-sm text-gray-500">Typing...</span>
                  </div>
                </div>
              )}
              <div ref={testEndRef} />
            </div>

            <div className="p-4 border-t border-gray-200 dark:border-gray-700">
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
          </div>
        )}
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

export function Phase3ChatFirst_Demo() {
  return <Phase3_ChatFirst />;
}
