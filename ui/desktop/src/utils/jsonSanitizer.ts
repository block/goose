export function sanitizeJsonString(jsonStr: string) {
  // Replace control characters (0x00-0x1F) except allowed ones
  // Allowed: \b (0x08), \t (0x09), \n (0x0A), \f (0x0C), \r (0x0D)
  // eslint-disable-next-line no-control-regex
  return jsonStr.replace(/[\u0000-\u0007\u000B\u000E-\u001F]/g, '');
}

export function safeJsonParse(str: string) {
  try {
    // First try direct parse
    return JSON.parse(str);
  } catch (e) {
    // If that fails, try sanitizing first
    console.log('Initial JSON parse failed, attempting with sanitization...');
    const sanitized = sanitizeJsonString(str);
    try {
      return JSON.parse(sanitized);
    } catch (e2) {
      console.error('Failed to parse JSON even after sanitization:', e2);
      throw e2;
    }
  }
}
