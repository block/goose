/**
 * config-merge.js - Sell CONFIG object merge transform
 *
 * Merges new config fields into existing sell template CONFIG object
 * while preserving user-customized values.
 */

/**
 * Regex to match the CONFIG object in sell templates
 */
const CONFIG_REGEX = /const\s+CONFIG\s*=\s*\{([\s\S]*?)\n\s*\};/;

/**
 * Known CONFIG fields with their default values
 */
const CONFIG_SCHEMA = {
  CLERK_PUBLISHABLE_KEY: { type: 'string', placeholder: '__CLERK_PUBLISHABLE_KEY__' },
  APP_NAME: { type: 'string', placeholder: '__APP_NAME__' },
  APP_TITLE: { type: 'string', placeholder: '__APP_TITLE__' },
  APP_DOMAIN: { type: 'string', placeholder: '__APP_DOMAIN__' },
  APP_TAGLINE: { type: 'string', placeholder: '__APP_TAGLINE__' },
  APP_DESCRIPTION: { type: 'string', placeholder: '__APP_DESCRIPTION__' },
  FEATURES: { type: 'array', placeholder: '__FEATURES__' },
  ADMIN_USER_IDS: { type: 'array', placeholder: '__ADMIN_USER_IDS__' },
  MONTHLY_PRICE: { type: 'string', placeholder: '__MONTHLY_PRICE__' },
  YEARLY_PRICE: { type: 'string', placeholder: '__YEARLY_PRICE__' },
  STRIPE_MONTHLY_PRICE_ID: { type: 'string', placeholder: '__STRIPE_MONTHLY_PRICE_ID__' },
  STRIPE_YEARLY_PRICE_ID: { type: 'string', placeholder: '__STRIPE_YEARLY_PRICE_ID__' }
};

/**
 * Parse CONFIG object from HTML
 * @param {string} html - The HTML content
 * @returns {object|null} - Parsed config or null
 */
function parseConfig(html) {
  const match = html.match(CONFIG_REGEX);
  if (!match) {
    return null;
  }

  const configContent = match[1];
  const config = {};

  // Parse each key-value pair
  // Match: KEY: value, or KEY: "value", or KEY: [...], etc.
  const keyValueRegex = /(\w+)\s*:\s*((?:"[^"]*"|'[^']*'|\[[\s\S]*?\]|[^,\n]+))\s*,?/g;

  let kvMatch;
  while ((kvMatch = keyValueRegex.exec(configContent)) !== null) {
    const key = kvMatch[1];
    let value = kvMatch[2].trim();

    // Remove trailing comma if present
    if (value.endsWith(',')) {
      value = value.slice(0, -1).trim();
    }

    // Try to parse JSON values
    try {
      config[key] = JSON.parse(value);
    } catch (e) {
      // Keep as string if not valid JSON
      config[key] = value;
    }
  }

  return config;
}

/**
 * Serialize config object back to JavaScript object literal
 * @param {object} config - Config object
 * @returns {string} - JavaScript object literal string
 */
function serializeConfig(config) {
  const lines = [];

  for (const [key, value] of Object.entries(config)) {
    let valueStr;

    if (typeof value === 'string') {
      // Check if it's a placeholder (starts and ends with __)
      if (value.startsWith('__') && value.endsWith('__')) {
        valueStr = `"${value}"`;
      } else if (value.includes("'") && !value.includes('"')) {
        valueStr = `"${value}"`;
      } else {
        valueStr = JSON.stringify(value);
      }
    } else if (Array.isArray(value)) {
      valueStr = JSON.stringify(value);
    } else {
      valueStr = JSON.stringify(value);
    }

    lines.push(`  ${key}: ${valueStr}`);
  }

  return `const CONFIG = {\n${lines.join(',\n')}\n};`;
}

/**
 * Merge new config fields into existing config
 * @param {object} existing - Existing config
 * @param {object} updates - New/updated fields
 * @returns {object} - Merged config
 */
function mergeConfigs(existing, updates) {
  const merged = { ...existing };

  for (const [key, value] of Object.entries(updates)) {
    // Only add new fields, don't overwrite user values
    if (!(key in existing)) {
      merged[key] = value;
    }
  }

  return merged;
}

/**
 * Check which config fields are missing
 * @param {object} config - Current config
 * @returns {string[]} - Array of missing field names
 */
function findMissingFields(config) {
  const missing = [];

  for (const key of Object.keys(CONFIG_SCHEMA)) {
    if (!(key in config)) {
      missing.push(key);
    }
  }

  return missing;
}

/**
 * Apply config merge transform
 * @param {string} html - The HTML content
 * @param {object} newFields - New fields to add
 * @returns {object} - Transform result
 */
function applyConfigMerge(html, newFields) {
  const existingConfig = parseConfig(html);

  if (!existingConfig) {
    return {
      success: false,
      error: 'No CONFIG object found in HTML'
    };
  }

  const mergedConfig = mergeConfigs(existingConfig, newFields);
  const newConfigStr = serializeConfig(mergedConfig);

  // Replace CONFIG in HTML
  const newHtml = html.replace(CONFIG_REGEX, newConfigStr);

  if (newHtml === html) {
    return {
      success: false,
      error: 'Config merge had no effect'
    };
  }

  const addedFields = Object.keys(newFields).filter(k => !(k in existingConfig));

  return {
    success: true,
    html: newHtml,
    diff: {
      addedFields,
      mergedConfig
    }
  };
}

/**
 * Add missing standard fields to config
 * @param {string} html - The HTML content
 * @returns {object} - Transform result
 */
function addMissingConfigFields(html) {
  const existingConfig = parseConfig(html);

  if (!existingConfig) {
    return {
      success: false,
      error: 'No CONFIG object found in HTML'
    };
  }

  const missingFields = findMissingFields(existingConfig);

  if (missingFields.length === 0) {
    return {
      success: false,
      error: 'No missing config fields'
    };
  }

  // Create new fields with placeholder values
  const newFields = {};
  for (const field of missingFields) {
    const schema = CONFIG_SCHEMA[field];
    if (schema.type === 'array') {
      newFields[field] = [];
    } else {
      newFields[field] = schema.placeholder;
    }
  }

  return applyConfigMerge(html, newFields);
}

export {
  parseConfig,
  serializeConfig,
  mergeConfigs,
  findMissingFields,
  applyConfigMerge,
  addMissingConfigFields,
  CONFIG_REGEX,
  CONFIG_SCHEMA
};
