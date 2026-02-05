const htmlEscapes = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;',
};
function escape(str) {
    return str.replace(/[&<>"']/g, match => htmlEscapes[match]);
}

export { escape };
