const htmlUnescapes = {
    '&amp;': '&',
    '&lt;': '<',
    '&gt;': '>',
    '&quot;': '"',
    '&#39;': "'",
};
function unescape(str) {
    return str.replace(/&(?:amp|lt|gt|quot|#(0+)?39);/g, match => htmlUnescapes[match] || "'");
}

export { unescape };
