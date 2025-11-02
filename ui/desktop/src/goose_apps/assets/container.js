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
  constructor(widgetClassName) {
    this.widgetClassName = widgetClassName;
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

  loadWidget() {
    this.container = document.getElementById('app-container');
    if (!this.container) {
      throw new Error('App container not found');
    }

    const WidgetClass = window[this.widgetClassName];
    if (!WidgetClass) {
      throw new Error(`Widget class ${this.widgetClassName} not found`);
    }

    const api = new GooseAppWidgetAPI();
    this.widget = new WidgetClass(api);
    api.widget = this.widget;

    const css = this.widget.css();
    if (css) {
      const style = document.createElement('style');
      style.textContent = css;
      document.head.appendChild(style);
    }

    this.widget.element = document.createElement('div');
    this.widget.element.className = 'widget-container';
    this.widget.element.innerHTML = this.widget.render();
    this.container.appendChild(this.widget.element);
    this.widget.bindEvents();
    this.widget.onMount();
  }
}
