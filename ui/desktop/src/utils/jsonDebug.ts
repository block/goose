export function debugJsonString(jsonStr: string) {
  // Find problematic characters around position 11843
  const start = Math.max(0, 11843 - 20);
  const end = Math.min(jsonStr.length, 11843 + 20);
  const context = jsonStr.substring(start, end);

  // Convert the context to show control characters
  const charCodes = Array.from(context).map((char) => ({
    char,
    code: char.charCodeAt(0),
    hex: char.charCodeAt(0).toString(16).padStart(4, '0'),
  }));

  console.log('JSON Debug Info:');
  console.log('Context around position 11843:');
  charCodes.forEach(({ char, code, hex }, index) => {
    const position = start + index;
    console.log(`Position ${position}: '${char}' (${code}, 0x${hex})`);
  });

  // Try to validate JSON with more detailed error
  try {
    JSON.parse(jsonStr);
    console.log('JSON is valid');
  } catch (e) {
    console.error('JSON parse error:', e);
  }
}
