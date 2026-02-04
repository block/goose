/**
 * Phase 4: Test - Test Your Recipe Before Saving
 *
 * Sequential flow after Phase 3 (Build):
 * - Left: Recipe summary (what you built)
 * - Right: Test chat to try the recipe
 *
 * Key features:
 * - See how the recipe behaves before saving
 * - [Back to Edit] to return to Phase 3
 * - [Save] to save and go to recipes list
 * - [Save & Run] to save and start a real chat
 */

import React, { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Send,
  ArrowLeft,
  Save,
  Play,
  RefreshCw,
  CheckCircle,
  FileText,
  Settings,
  Zap,
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

interface TestMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
}

// Mock recipe data (would come from Phase 3 in real implementation)
const mockRecipe: RecipeForm = {
  title: 'Email Writing Assistant',
  description: 'Helps write professional emails quickly',
  instructions: `You are a professional email writing assistant. Follow these rules:

1. Always ask what the email is about first
2. Ask who the recipient is
3. Use professional but friendly tone
4. Keep emails concise (under 200 words)
5. No emojis unless explicitly requested`,
  activities: ['write_file', 'read_file'],
  parameters: [
    {
      name: 'tone',
      description: 'The tone of the email',
      type: 'string',
      required: false,
    },
  ],
};

// Recipe summary component
function RecipeSummary({ recipe }: { recipe: RecipeForm }) {
  return (
    <div className="space-y-4">
      {/* Title & Description */}
      <div>
        <h3 className="text-lg font-semibold">{recipe.title}</h3>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">{recipe.description}</p>
      </div>

      {/* Instructions preview */}
      <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-4">
        <div className="flex items-center gap-2 mb-2">
          <FileText className="w-4 h-4 text-gray-400" />
          <span className="text-sm font-medium">Instructions</span>
        </div>
        <pre className="text-xs text-gray-600 dark:text-gray-300 whitespace-pre-wrap font-mono">
          {recipe.instructions}
        </pre>
      </div>

      {/* Parameters */}
      {recipe.parameters.length > 0 && (
        <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-2">
            <Settings className="w-4 h-4 text-gray-400" />
            <span className="text-sm font-medium">Parameters ({recipe.parameters.length})</span>
          </div>
          <div className="space-y-2">
            {recipe.parameters.map((param, index) => (
              <div key={index} className="text-sm">
                <span className="font-mono text-blue-600 dark:text-blue-400">
                  {`{{${param.name}}}`}
                </span>
                <span className="text-gray-500 ml-2">- {param.description}</span>
                {param.required && (
                  <span className="ml-2 text-xs text-red-500">required</span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Capabilities */}
      {recipe.activities.length > 0 && (
        <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-2">
            <Zap className="w-4 h-4 text-gray-400" />
            <span className="text-sm font-medium">Capabilities ({recipe.activities.length})</span>
          </div>
          <div className="flex flex-wrap gap-2">
            {recipe.activities.map((activity, index) => (
              <span
                key={index}
                className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-700 rounded"
              >
                {activity}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// Test message component
function TestMessageBubble({ message }: { message: TestMessage }) {
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
            : 'bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-100'
        }`}
      >
        <p className="text-sm whitespace-pre-wrap">{message.content}</p>
      </div>
    </div>
  );
}

// Main component
export default function Phase4_TestRecipe() {
  const navigate = useNavigate();
  const [recipe] = useState<RecipeForm>(mockRecipe);
  const [messages, setMessages] = useState<TestMessage[]>([
    {
      id: 'system-1',
      role: 'system',
      content: 'Test conversation started',
    },
    {
      id: '1',
      role: 'assistant',
      content:
        "Hi! I'm your Email Writing Assistant. What email would you like help with today? Please tell me:\n\n• What's the email about?\n• Who is the recipient?",
    },
  ]);
  const [inputValue, setInputValue] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Simulate AI response based on recipe instructions
  const simulateResponse = (userMessage: string) => {
    setIsTyping(true);

    setTimeout(() => {
      let response: string;

      if (userMessage.toLowerCase().includes('meeting') || userMessage.toLowerCase().includes('schedule')) {
        response = `Got it! I'll help you write an email about scheduling a meeting.\n\nBefore I draft it, a few questions:\n• Who is this email to? (colleague, manager, client)\n• What's the preferred meeting time?\n• Is this urgent or flexible?`;
      } else if (userMessage.toLowerCase().includes('thank')) {
        response = `Here's a professional thank-you email:\n\n---\nSubject: Thank You\n\nDear [Recipient],\n\nThank you for taking the time to meet with me today. I appreciated the opportunity to discuss [topic].\n\nI look forward to our continued collaboration.\n\nBest regards,\n[Your name]\n---\n\nWould you like me to adjust the tone or add any specific details?`;
      } else {
        response = `I understand you need help with: "${userMessage}"\n\nTo write the best email for you, could you tell me:\n• Who is the recipient?\n• What tone would you prefer? (formal, friendly, urgent)\n• Any specific points to include?`;
      }

      setMessages((prev) => [
        ...prev,
        {
          id: Date.now().toString(),
          role: 'assistant',
          content: response,
        },
      ]);
      setIsTyping(false);
    }, 1200);
  };

  // Send message
  const handleSend = () => {
    if (!inputValue.trim()) return;

    const userMessage: TestMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: inputValue,
    };

    setMessages((prev) => [...prev, userMessage]);
    setInputValue('');
    simulateResponse(inputValue);
  };

  // Reset test
  const handleReset = () => {
    setMessages([
      {
        id: 'system-1',
        role: 'system',
        content: 'Test conversation started',
      },
      {
        id: '1',
        role: 'assistant',
        content:
          "Hi! I'm your Email Writing Assistant. What email would you like help with today? Please tell me:\n\n• What's the email about?\n• Who is the recipient?",
      },
    ]);
    setInputValue('');
  };

  // Navigation handlers
  const handleBackToEdit = () => {
    navigate('/prototype-phase3');
  };

  const handleSave = () => {
    alert('Recipe saved! Navigating to Recipes list...');
    navigate('/recipes');
  };

  const handleSaveAndRun = () => {
    alert('Recipe saved! Starting new chat with this recipe...');
    // In real implementation: navigate to new chat with recipe loaded
    navigate('/recipes');
  };

  return (
    <div className="h-screen flex flex-col bg-gray-50 dark:bg-gray-900">
      {/* Header */}
      <div className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <CheckCircle className="w-6 h-6 text-green-500" />
            <div>
              <h1 className="text-lg font-semibold">Test Your Recipe</h1>
              <p className="text-sm text-gray-500">Step 2 of 2: Try it before saving</p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={handleBackToEdit}
              className="px-4 py-2 text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md flex items-center gap-2"
            >
              <ArrowLeft className="w-4 h-4" />
              Back to Edit
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
        {/* Left: Recipe Summary (35%) */}
        <div className="w-[35%] border-r border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-y-auto p-6">
          <div className="mb-4">
            <h2 className="font-medium text-gray-900 dark:text-gray-100">Recipe Summary</h2>
            <p className="text-xs text-gray-500 mt-1">This is what you built</p>
          </div>
          <RecipeSummary recipe={recipe} />
        </div>

        {/* Right: Test Chat (65%) */}
        <div className="w-[65%] flex flex-col">
          {/* Test chat header */}
          <div className="px-4 py-3 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 flex items-center justify-between">
            <div>
              <h2 className="font-medium">Test Chat</h2>
              <p className="text-xs text-gray-500">Try your recipe - this won't be saved</p>
            </div>
            <button
              onClick={handleReset}
              className="px-3 py-1.5 text-sm text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md flex items-center gap-1.5"
            >
              <RefreshCw className="w-3.5 h-3.5" />
              Reset
            </button>
          </div>

          {/* Messages */}
          <div className="flex-1 overflow-y-auto p-4 space-y-4 bg-gray-50 dark:bg-gray-900">
            {messages.map((msg) => (
              <TestMessageBubble key={msg.id} message={msg} />
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
          <div className="p-4 border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
            <div className="flex gap-2">
              <input
                type="text"
                value={inputValue}
                onChange={(e) => setInputValue(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSend()}
                placeholder="Test your recipe..."
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

export function Phase4_Demo() {
  return <Phase4_TestRecipe />;
}
