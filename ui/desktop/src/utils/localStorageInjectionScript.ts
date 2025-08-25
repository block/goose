/**
 * Generates the JavaScript code to inject into the renderer process for setting localStorage
 * with retry logic. This is used by the main process to inject configuration.
 *
 * @param key The localStorage key to set
 * @param value The value to store (will be JSON stringified if not already a string)
 * @returns JavaScript code as a string
 */
export default function generateLocalStorageInjectionScript(key: string, value: string): string {
  return `
    (function() {
      let retryCount = 0;
      const maxRetries = 5;
      const baseDelay = 100;
      
      function setConfig() {
        try {
          if (window.localStorage && typeof window.localStorage.setItem === 'function') {
            localStorage.setItem('${key}', '${value}');
            console.log('[Renderer] Successfully set localStorage ${key}');
            return true;
          } else {
            console.warn('[Renderer] localStorage not available or setItem not a function');
          }
        } catch (e) {
          console.warn('[Renderer] localStorage access failed:', e);
        }
        return false;
      }

      function retrySetConfig() {
        if (setConfig()) {
          return; // Success, no need to retry
        }
        
        retryCount++;
        if (retryCount < maxRetries) {
          const delay = baseDelay * Math.pow(2, retryCount - 1); // Exponential backoff
          console.log(\`[Renderer] Retrying localStorage ${key} set (attempt \${retryCount + 1}/\${maxRetries}) in \${delay}ms\`);
          setTimeout(retrySetConfig, delay);
        } else {
          console.error('[Renderer] Failed to set localStorage ${key} after all retries - continuing without localStorage config');           
        }
      }

      // Initial attempt
      retrySetConfig();
    })();
  `;
}
