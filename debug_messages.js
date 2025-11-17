
// Debug the actual message content
console.log('🔍 DETAILED MESSAGE ANALYSIS:');
console.log('Messages array:', chat.messages);
chat.messages.forEach((msg, i) => {
  console.log(`Message ${i}:`, {
    id: msg.id,
    role: msg.role,
    content: msg.content,
    contentType: typeof msg.content,
    contentLength: Array.isArray(msg.content) ? msg.content.length : 'not array',
    firstContentItem: Array.isArray(msg.content) ? msg.content[0] : 'not array'
  });
});

// Check if BaseChat is receiving the messages
console.log('🔍 BaseChat props check:');
console.log('filteredMessages length:', document.querySelector('[data-testid="chat-messages"]') ? 'Chat container found' : 'Chat container NOT found');

