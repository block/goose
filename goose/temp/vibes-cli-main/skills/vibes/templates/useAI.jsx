/**
 * useAI - React hook for AI calls via the Vibes proxy
 *
 * Replaces call-ai with a proxied solution that:
 * - Routes through /api/ai/chat on the same origin
 * - Handles authentication (Clerk JWT for sell, simple auth for vibes)
 * - Returns structured errors including LIMIT_EXCEEDED
 *
 * Usage:
 *   const { callAI, loading, error } = useAI();
 *   const response = await callAI({
 *     model: "anthropic/claude-sonnet",
 *     messages: [{ role: "user", content: "Hello!" }]
 *   });
 */

const AIContext = React.createContext(null);

function AIProvider({ children }) {
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState(null);

  const callAI = React.useCallback(async (options) => {
    setLoading(true);
    setError(null);

    try {
      // Get auth token if Clerk is available (sell apps)
      let authHeader = {};
      if (typeof window !== 'undefined' && window.Clerk?.session) {
        const token = await window.Clerk.session.getToken();
        if (token) {
          authHeader = { 'Authorization': `Bearer ${token}` };
        }
      }

      const response = await fetch('/api/ai/chat', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...authHeader
        },
        body: JSON.stringify({
          model: options.model || 'anthropic/claude-sonnet-4',
          messages: options.messages,
          ...options
        })
      });

      // Handle limit exceeded (402 from OpenRouter)
      if (response.status === 402) {
        const err = { code: 'LIMIT_EXCEEDED', message: 'AI usage limit reached for this month.' };
        setError(err);
        throw err;
      }

      // Handle other errors
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        const err = {
          code: 'API_ERROR',
          message: errorData.error?.message || `API error: ${response.status}`,
          status: response.status
        };
        setError(err);
        throw err;
      }

      const data = await response.json();
      return data;

    } catch (err) {
      // Don't double-set if already set above
      if (!error) {
        setError({ code: 'NETWORK_ERROR', message: err.message || 'Network error' });
      }
      throw err;
    } finally {
      setLoading(false);
    }
  }, []);

  const value = React.useMemo(() => ({ callAI, loading, error, setError }), [callAI, loading, error]);

  return React.createElement(AIContext.Provider, { value }, children);
}

function useAI() {
  const context = React.useContext(AIContext);

  // If used outside AIProvider, return a standalone version
  if (!context) {
    const [loading, setLoading] = React.useState(false);
    const [error, setError] = React.useState(null);

    const callAI = React.useCallback(async (options) => {
      setLoading(true);
      setError(null);

      try {
        let authHeader = {};
        if (typeof window !== 'undefined' && window.Clerk?.session) {
          const token = await window.Clerk.session.getToken();
          if (token) {
            authHeader = { 'Authorization': `Bearer ${token}` };
          }
        }

        const response = await fetch('/api/ai/chat', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            ...authHeader
          },
          body: JSON.stringify({
            model: options.model || 'anthropic/claude-sonnet-4',
            messages: options.messages,
            ...options
          })
        });

        if (response.status === 402) {
          const err = { code: 'LIMIT_EXCEEDED', message: 'AI usage limit reached for this month.' };
          setError(err);
          throw err;
        }

        if (!response.ok) {
          const errorData = await response.json().catch(() => ({}));
          const err = {
            code: 'API_ERROR',
            message: errorData.error?.message || `API error: ${response.status}`,
            status: response.status
          };
          setError(err);
          throw err;
        }

        return await response.json();

      } catch (err) {
        if (!error) {
          setError({ code: 'NETWORK_ERROR', message: err.message || 'Network error' });
        }
        throw err;
      } finally {
        setLoading(false);
      }
    }, []);

    return { callAI, loading, error, clearError: () => setError(null) };
  }

  return { ...context, clearError: () => context.setError(null) };
}

// Make available globally for template injection
window.AIProvider = AIProvider;
window.useAI = useAI;
