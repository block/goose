export interface MockListedResources {
  resources: MockResourceListItem[];
}

export interface MockReadResources {
  contents: MockReadResourceItem[];
}

export interface MockResourceListItem {
  uri: UIResourceUri;
  name: UIResourceName;
  description: UIResourceDescription;
  mimeType: UIResourceMimeType;
}

export interface MockReadResourceItem {
  uri: UIResourceUri;
  description?: UIResourceDescription;
  mimeType: UIResourceMimeType;
  text?: UIResourceText;

  /**
   * Resource metadata for security and rendering configuration
   *
   * Includes Content Security Policy configuration, dedicated domain settings,
   * and visual preferences.
   */
  _meta?: {
    ui?: UIResourceMeta;
  };
}

export interface UIResourceMeta {
  /**
   * Content Security Policy configuration
   *
   * Servers declare which external origins their UI needs to access.
   * Hosts use this to enforce appropriate CSP headers.
   */
  csp?: {
    connectDomains?: Origin[];
    resourceDomains?: Origin[];
  };
  /**
   * Dedicated origin for widget
   *
   * Optional domain for the widget's sandbox origin. Useful when widgets need
   * dedicated origins for API key allowlists or cross-origin isolation.
   *
   * If omitted, Host uses default sandbox origin.
   *
   * @example
   * "https://weather-widget.example.com"
   */
  domain?: Domain;
  /**
   * Visual boundary preference
   *
   * Boolean indicating the UI prefers a visible border. Useful for widgets
   * that might blend with host background.
   *
   * - `true`: Request visible border (host decides styling)
   * - `false` or omitted: No preference
   */
  prefersBorder?: boolean;
}

/**
 * URI for UI resources
 *
 * MUST use the `ui://` URI scheme to distinguish UI resources from other
 * MCP resource types.
 *
 * @example
 * "ui://weather-dashboard"
 */
type UIResourceUri = `ui://${string}`;

/**
 * Human-readable display name for the UI resource
 *
 * Used for listing and identifying the resource in host interfaces.
 *
 * @example
 * "Weather Dashboard"
 */
type UIResourceName = string;

/**
 * Description of the UI resource's purpose and functionality
 *
 * Provides context about what the UI does and when to use it.
 *
 * @example
 * "Interactive weather visualization with real-time updates"
 */
type UIResourceDescription = string;

/**
 * MIME type of the UI content
 *
 * SHOULD be `text/html;profile=mcp-app` for HTML-based UIs in the initial MVP.
 * Other content types are reserved for future extensions.
 *
 * @example
 * "text/html;profile=mcp-app"
 */
type UIResourceMimeType = `text/html;profile=mcp-app`;

/**
 * Text content of the UI resource
 *
 * @example
 * "<!DOCTYPE html><html>...</html>"
 */
type UIResourceText = string;

/**
 * CSP origin for network requests or static resources
 *
 * Can be a full URL (https://, wss://) or wildcard pattern (https://*.example.com)
 *
 * @example
 * "https://api.weather.com"
 * "wss://realtime.service.com"
 * "https://*.cloudflare.com"
 */
type Origin = string;

/**
 * Dedicated origin domain for widget sandbox
 *
 * Must be a full HTTPS URL for the widget's sandbox origin.
 *
 * @example
 * "https://weather-widget.example.com"
 */
type Domain = `https://${string}`;
