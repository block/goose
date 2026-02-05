/**
 * Converts the characters "&", "<", ">", '"', and "'" in `str` to their corresponding HTML entities.
 * For example, "<" becomes "&lt;".
 *
 * @param {string} str  The string to escape.
 * @returns {string} Returns the escaped string.
 *
 * @example
 * escape('This is a <div> element.'); // returns 'This is a &lt;div&gt; element.'
 * escape('This is a "quote"'); // returns 'This is a &quot;quote&quot;'
 * escape("This is a 'quote'"); // returns 'This is a &#39;quote&#39;'
 * escape('This is a & symbol'); // returns 'This is a &amp; symbol'
 */
declare function escape(str: string): string;

export { escape };
