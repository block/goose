/**
 * Goose Bridge — runtime API for Goose apps.
 *
 * Implements the MCP Apps UI protocol (JSON-RPC over postMessage) directly,
 * without depending on the ext-apps SDK.
 *
 * Provides window.goose:
 *   goose.theme              - "light" or "dark"
 *   goose.onReady            - callback, fired once after init
 *   goose.onThemeChange      - callback(theme), fired on theme changes
 *   goose.{ext}.{tool}(args) - call any MCP tool, returns result text
 *   goose.chat(messages, opts) - sampling/createMessage
 */
(function() {
  'use strict';

  var _id = 0;
  var _pending = {};
  var _theme = 'light';
  var _ready = false;
  var _onReady = null;
  var _onThemeChange = null;

  // JSON-RPC over postMessage
  function request(method, params) {
    return new Promise(function(resolve, reject) {
      var id = ++_id;
      _pending[id] = { resolve: resolve, reject: reject };
      window.parent.postMessage({ jsonrpc: '2.0', id: id, method: method, params: params || {} }, '*');
    });
  }

  function notify(method, params) {
    window.parent.postMessage({ jsonrpc: '2.0', method: method, params: params || {} }, '*');
  }

  window.addEventListener('message', function(e) {
    var data = e.data;
    if (!data || typeof data !== 'object') return;

    // Response to a pending request
    if ('id' in data && _pending[data.id]) {
      var p = _pending[data.id];
      delete _pending[data.id];
      if (data.error) p.reject(new Error(data.error.message || 'Unknown error'));
      else p.resolve(data.result);
      return;
    }

    // Notification from host
    if (data.method === 'ui/notifications/host-context-changed') {
      var params = data.params || {};
      if (params.theme) {
        _theme = params.theme;
        applyTheme(params.theme);
        if (_onThemeChange) _onThemeChange(params.theme);
      }
    }
  });

  // Tool-calling proxy: goose.developer.shell({command: "ls"}) → tools/call
  var toolProxy = new Proxy({}, {
    get: function(_, namespace) {
      if (typeof namespace !== 'string') return undefined;
      return new Proxy({}, {
        get: function(_, toolName) {
          if (typeof toolName !== 'string') return undefined;
          return function(args) {
            return request('tools/call', {
              name: namespace + '__' + toolName,
              arguments: args || {}
            }).then(function(result) {
              if (result && result.isError) {
                var msg = (result.content && result.content.find(function(c) { return c.type === 'text'; }) || {}).text || 'Tool call failed';
                throw new Error(msg);
              }
              var textBlock = result && result.content && result.content.find(function(c) { return c.type === 'text'; });
              return textBlock ? textBlock.text : (result && result.content);
            });
          };
        }
      });
    }
  });

  function chat(messages, opts) {
    opts = opts || {};
    return request('sampling/createMessage', {
      messages: messages,
      systemPrompt: opts.systemPrompt,
      maxTokens: opts.maxTokens || 1000
    });
  }

  window.goose = new Proxy(toolProxy, {
    get: function(target, prop) {
      if (prop === 'theme') return _theme;
      if (prop === 'onReady') return _onReady;
      if (prop === 'onThemeChange') return _onThemeChange;
      if (prop === 'chat') return chat;
      if (typeof prop === 'symbol') return undefined;
      return target[prop];
    },
    set: function(_, prop, value) {
      if (prop === 'onReady') {
        _onReady = value;
        if (_ready && value) value();
        return true;
      }
      if (prop === 'onThemeChange') { _onThemeChange = value; return true; }
      return false;
    }
  });

  // Apply theme as MCP CSS custom properties and data attribute
  var _themes = {
    light: {
      '--color-background-primary': '#ffffff',
      '--color-background-secondary': '#f5f5f5',
      '--color-background-tertiary': '#e8e8e8',
      '--color-background-inverse': '#1e1e2e',
      '--color-background-info': '#e3f2fd',
      '--color-background-danger': '#fce4ec',
      '--color-background-success': '#e8f5e9',
      '--color-background-warning': '#fff3e0',
      '--color-text-primary': '#1e1e2e',
      '--color-text-secondary': '#666666',
      '--color-text-tertiary': '#999999',
      '--color-text-inverse': '#ffffff',
      '--color-text-info': '#1565c0',
      '--color-text-danger': '#c62828',
      '--color-text-success': '#2e7d32',
      '--color-text-warning': '#e65100',
      '--color-border-primary': '#e0e0e0',
      '--color-border-secondary': '#eeeeee',
      '--font-sans': '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
      '--font-mono': '"SF Mono", SFMono-Regular, Consolas, "Liberation Mono", Menlo, monospace',
      '--border-radius-sm': '4px',
      '--border-radius-md': '8px',
      '--border-radius-lg': '12px'
    },
    dark: {
      '--color-background-primary': '#1e1e2e',
      '--color-background-secondary': '#313244',
      '--color-background-tertiary': '#45475a',
      '--color-background-inverse': '#ffffff',
      '--color-background-info': '#1a237e',
      '--color-background-danger': '#4a0e0e',
      '--color-background-success': '#1b3a1b',
      '--color-background-warning': '#3e2700',
      '--color-text-primary': '#cdd6f4',
      '--color-text-secondary': '#a6adc8',
      '--color-text-tertiary': '#7f849c',
      '--color-text-inverse': '#1e1e2e',
      '--color-text-info': '#89b4fa',
      '--color-text-danger': '#f38ba8',
      '--color-text-success': '#a6e3a1',
      '--color-text-warning': '#fab387',
      '--color-border-primary': '#45475a',
      '--color-border-secondary': '#585b70',
      '--font-sans': '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
      '--font-mono': '"SF Mono", SFMono-Regular, Consolas, "Liberation Mono", Menlo, monospace',
      '--border-radius-sm': '4px',
      '--border-radius-md': '8px',
      '--border-radius-lg': '12px'
    }
  };

  function applyTheme(theme) {
    var root = document.documentElement;
    root.setAttribute('data-theme', theme);
    var vars = _themes[theme] || _themes.light;
    for (var key in vars) {
      root.style.setProperty(key, vars[key]);
    }
  }

  // Initialize: handshake with host
  request('ui/initialize', {
    protocolVersion: '2026-01-26',
    appInfo: { name: '{{APP_NAME}}', version: '1.0.0' },
    appCapabilities: {}
  }).then(function(result) {
    notify('ui/notifications/initialized');
    if (result && result.hostContext && result.hostContext.theme) {
      _theme = result.hostContext.theme;
    }
    applyTheme(_theme);
    _ready = true;
    if (_onReady) _onReady();
  }).catch(function(e) {
    console.warn('[goose] init failed:', e.message);
    applyTheme(_theme);
    _ready = true;
    if (_onReady) _onReady();
  });
})();
