/**
 * Concrete implementation of WidgetAPI for the Goose App environment
 */
class GooseAppWidgetAPI extends window.WidgetAPI {
  constructor(widget) {
    super();
    this.widget = widget;
    this.properties = new Map();
  }

  async setProperty(key, value) {
    this.properties.set(key, value);
    // In a real implementation, this might persist to storage
    return true;
  }

  getProperty(key, defaultValue = null) {
    return this.properties.get(key) ?? defaultValue;
  }

  async LLMCall(prompt) {
    // Mock implementation - in real app this would call the Goose LLM
    console.log('LLM Call:', prompt);
    return `Mock response to: ${prompt}`;
  }

  async requestGoogleClient() {
    // Mock implementation - in real app this would return a Google client
    throw new Error('Google client not implemented in mock environment');
  }

  update() {
    if (this.widget && this.widget.element) {
      const newContent = this.widget.render();
      this.widget.element.innerHTML = newContent;
      // Rebind events after update
      this.widget.bindEvents();
    }
  }

  bindEvent(selector, eventType, handler) {
    if (this.widget && this.widget.element) {
      const elements = this.widget.element.querySelectorAll(selector);
      elements.forEach((element) => {
        element.addEventListener(eventType, handler);
      });
    }
  }
}

/**
 * Main app initialization and widget management
 */
class GooseAppManager {
  constructor() {
    this.widget = null;
    this.container = null;
  }

  showError(message) {
    if (this.container) {
      this.container.innerHTML = `
        <div class="error-container">
          <h2>Error</h2>
          <p>${message}</p>
        </div>
      `;
    }
  }

  getQueryParams() {
    const params = new URLSearchParams(window.location.search);
    return {
      appName: params.get('appName'),
      implementation: params.get('implementation'),
    };
  }

  loadWidget() {
    const { appName, implementation } = this.getQueryParams();

    if (!appName || !implementation) {
      this.showError('Missing app parameters');
      return;
    }

    this.container = document.getElementById('app-container');
    if (!this.container) {
      throw new Error('App container not found');
    }

    const jsImplementation = atob(implementation);

    // Extract class name with regex
    const classMatch = jsImplementation.match(/class\s+(\w+)\s+extends\s+GooseWidget/);
    if (!classMatch) {
      throw new Error('No class extending GooseWidget found in implementation');
    }

    const className = classMatch[1];

    // Execute the script and manually assign to global
    const script = document.createElement('script');
    script.textContent = `${jsImplementation}\nwindow.${className} = ${className};`;
    document.head.appendChild(script);

    const WidgetClass = window[className];
    if (!WidgetClass) {
      throw new Error(`Class ${className} not found after script execution`);
    }

    // Create the widget with its API
    const api = new GooseAppWidgetAPI();
    this.widget = new WidgetClass(api);
    api.widget = this.widget; // Set the widget reference in the API

    // Update document title
    document.title = `Goose App - ${this.widget.getName() || appName}`;

    // Inject widget CSS if provided
    const css = this.widget.css();
    if (css) {
      const style = document.createElement('style');
      style.textContent = css;
      document.head.appendChild(style);
    }

    // Create widget element and render
    this.widget.element = document.createElement('div');
    this.widget.element.className = 'widget-container';
    this.widget.element.innerHTML = this.widget.render();

    this.container.appendChild(this.widget.element);

    // Bind events
    this.widget.bindEvents();

    // Call onMount
    this.widget.onMount();

    console.log(`Widget ${this.widget.getName()} loaded successfully`);
  }
}

let appManager;

function initializeApp() {
  appManager = new GooseAppManager();
  appManager.loadWidget();
}

// Handle page unload
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initializeApp);
} else {
  initializeApp();
}
