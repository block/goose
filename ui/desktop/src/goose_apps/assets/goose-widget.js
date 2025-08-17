/**
 * Abstract API interface that widgets can use to interact with the system.
 */
window.WidgetAPI = class WidgetAPI {
  constructor() {
    if (new.target === WidgetAPI) {
      throw new TypeError('Cannot construct WidgetAPI instances directly');
    }
  }

  async setProperty(key, value) {
    throw new Error('setProperty must be implemented');
  }

  getProperty(key, defaultValue = null) {
    throw new Error('getProperty must be implemented');
  }

  /** Call an LLM - provide a prompt and get the response as text **/
  async LLMCall(prompt) {
    throw new Error('LLMCall must be implemented');
  }

  /** Get a GoogleClient for API use; calendar, drive, docs & sheets are supported */
  async requestGoogleClient() {
    throw new Error('requestGoogleClient must be implemented');
  }

  /** Update the widget's DOM with new content **/
  update() {
    throw new Error('update must be implemented');
  }

  /**
   * Helper to bind an event listener to an element within the widget
   * selector: CSS selector for the element to bind to, like .clock-settings
   * eventType: The event type to listen for, like click
   * handler: The function to call when the event occurs
   */
  bindEvent(selector, eventType, handler) {
    throw new Error('bindEvent must be implemented');
  }
};

/**
 * Base widget class that all widgets should extend.
 */
window.GooseWidget = class GooseWidget {
  constructor(api) {
    if (!(api instanceof window.WidgetAPI)) {
      throw new TypeError('GooseWidget must be constructed with a WidgetAPI instance');
    }
    this.api = api;
    this.element = null;
  }

  /**
   * Get the display name of the widget. Can be overridden by subclasses.
   * @returns {string} The name to display in the widget header
   */
  getName() {
    return ''; // Default implementation returns empty string
  }

  /**
   * Get the default size for this widget type. Can be overridden by subclasses.
   * @returns {{width: number, height: number}} The default dimensions
   */
  getDefaultSize() {
    return { width: 300, height: 200 };
  }

  /**
   * Get CSS styles needed for this widget. All widget css lives in a shared namespace
   * so to avoid conflicts, all widget css should be prefixed with the widget name.
   */
  css() {
    return '';
  }

  /**
   * Render the widget content. Must be implemented by subclasses.
   * @returns {string} HTML string for the widget content
   */
  render() {
    throw new Error('render() must be implemented by subclass');
  }

  /**
   * Call bindEvent() for any events needed. Don't call explicitly from onMount
   */
  bindEvents() {}

  /**
   * Called when the widget is mounted to the DOM.
   */
  onMount() {}

  /**
   * Called when the widget is about to be removed
   */
  onClose() {}
};
