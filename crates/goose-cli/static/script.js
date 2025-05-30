// WebSocket connection and chat functionality
let socket = null;
let sessionId = getSessionId();
let isConnected = false;

// DOM elements
const messagesContainer = document.getElementById('messages');
const messageInput = document.getElementById('message-input');
const sendButton = document.getElementById('send-button');
const connectionStatus = document.getElementById('connection-status');

// Track if we're currently processing
let isProcessing = false;

// Get session ID - either from URL parameter, injected session name, or generate new one
function getSessionId() {
    // Check if session name was injected by server (for /session/:name routes)
    if (window.GOOSE_SESSION_NAME) {
        return window.GOOSE_SESSION_NAME;
    }
    
    // Check URL parameters
    const urlParams = new URLSearchParams(window.location.search);
    const sessionParam = urlParams.get('session') || urlParams.get('name');
    if (sessionParam) {
        return sessionParam;
    }
    
    // Generate new session ID using CLI format
    return generateSessionId();
}

// Generate a session ID using timestamp format (yyyymmdd_hhmmss) like CLI
function generateSessionId() {
    const now = new Date();
    const year = now.getFullYear();
    const month = String(now.getMonth() + 1).padStart(2, '0');
    const day = String(now.getDate()).padStart(2, '0');
    const hour = String(now.getHours()).padStart(2, '0');
    const minute = String(now.getMinutes()).padStart(2, '0');
    const second = String(now.getSeconds()).padStart(2, '0');
    
    return `${year}${month}${day}_${hour}${minute}${second}`;
}

// Format timestamp
function formatTimestamp(date) {
    return date.toLocaleTimeString('en-US', { 
        hour: '2-digit', 
        minute: '2-digit' 
    });
}

// Create message element
function createMessageElement(content, role, timestamp) {
    const messageDiv = document.createElement('div');
    messageDiv.className = `message ${role}`;
    
    // Create content div
    const contentDiv = document.createElement('div');
    contentDiv.className = 'message-content';
    contentDiv.innerHTML = formatMessageContent(content);
    messageDiv.appendChild(contentDiv);
    
    // Add timestamp
    const timestampDiv = document.createElement('div');
    timestampDiv.className = 'timestamp';
    timestampDiv.textContent = formatTimestamp(new Date(timestamp || Date.now()));
    messageDiv.appendChild(timestampDiv);
    
    return messageDiv;
}

// Format message content (handle markdown-like formatting)
function formatMessageContent(content) {
    // Escape HTML
    let formatted = content
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
    
    // Handle code blocks
    formatted = formatted.replace(/```(\w+)?\n([\s\S]*?)```/g, (match, lang, code) => {
        return `<pre><code class="language-${lang || 'plaintext'}">${code.trim()}</code></pre>`;
    });
    
    // Handle inline code
    formatted = formatted.replace(/`([^`]+)`/g, '<code>$1</code>');
    
    // Handle line breaks
    formatted = formatted.replace(/\n/g, '<br>');
    
    return formatted;
}

// Add message to chat
function addMessage(content, role, timestamp) {
    // Remove welcome message if it exists
    const welcomeMessage = messagesContainer.querySelector('.welcome-message');
    if (welcomeMessage) {
        welcomeMessage.remove();
    }
    
    const messageElement = createMessageElement(content, role, timestamp);
    messagesContainer.appendChild(messageElement);
    
    // Scroll to bottom
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Add thinking indicator
function addThinkingIndicator() {
    removeThinkingIndicator(); // Remove any existing one first
    
    const thinkingDiv = document.createElement('div');
    thinkingDiv.id = 'thinking-indicator';
    thinkingDiv.className = 'message thinking-message';
    thinkingDiv.innerHTML = `
        <div class="thinking-dots">
            <span></span>
            <span></span>
            <span></span>
        </div>
        <span class="thinking-text">Goose is thinking...</span>
    `;
    messagesContainer.appendChild(thinkingDiv);
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Remove thinking indicator
function removeThinkingIndicator() {
    const thinking = document.getElementById('thinking-indicator');
    if (thinking) {
        thinking.remove();
    }
}

// Connect to WebSocket
function connectWebSocket() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws`;
    
    socket = new WebSocket(wsUrl);
    
    socket.onopen = () => {
        console.log('WebSocket connected');
        isConnected = true;
        connectionStatus.textContent = 'Connected';
        connectionStatus.className = 'status connected';
        sendButton.disabled = false;
        
        // Check if this session exists and load history if it does
        loadSessionIfExists();
    };
    
    socket.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            handleServerMessage(data);
        } catch (e) {
            console.error('Failed to parse message:', e);
        }
    };
    
    socket.onclose = () => {
        console.log('WebSocket disconnected');
        isConnected = false;
        connectionStatus.textContent = 'Disconnected';
        connectionStatus.className = 'status disconnected';
        sendButton.disabled = true;
        
        // Attempt to reconnect after 3 seconds
        setTimeout(connectWebSocket, 3000);
    };
    
    socket.onerror = (error) => {
        console.error('WebSocket error:', error);
    };
}

// Handle messages from server
function handleServerMessage(data) {
    switch (data.type) {
        case 'response':
            // For streaming responses, we need to handle partial messages
            handleStreamingResponse(data);
            break;
        case 'tool_request':
            handleToolRequest(data);
            break;
        case 'tool_response':
            handleToolResponse(data);
            break;
        case 'tool_confirmation':
            handleToolConfirmation(data);
            break;
        case 'thinking':
            handleThinking(data);
            break;
        case 'context_exceeded':
            handleContextExceeded(data);
            break;
        case 'cancelled':
            handleCancelled(data);
            break;
        case 'complete':
            handleComplete(data);
            break;
        case 'error':
            removeThinkingIndicator();
            resetSendButton();
            addMessage(`Error: ${data.message}`, 'assistant', Date.now());
            break;
        default:
            console.log('Unknown message type:', data.type);
    }
}

// Track current streaming message
let currentStreamingMessage = null;

// Handle streaming responses
function handleStreamingResponse(data) {
    removeThinkingIndicator();
    
    // If this is the first chunk of a new message, or we don't have a current streaming message
    if (!currentStreamingMessage) {
        // Create a new message element
        const messageElement = createMessageElement(data.content, data.role || 'assistant', data.timestamp);
        messageElement.setAttribute('data-streaming', 'true');
        messagesContainer.appendChild(messageElement);
        
        currentStreamingMessage = {
            element: messageElement,
            content: data.content,
            role: data.role || 'assistant',
            timestamp: data.timestamp
        };
    } else {
        // Append to existing streaming message
        currentStreamingMessage.content += data.content;
        
        // Update the message content using the proper content div
        const contentDiv = currentStreamingMessage.element.querySelector('.message-content');
        if (contentDiv) {
            contentDiv.innerHTML = formatMessageContent(currentStreamingMessage.content);
        }
    }
    
    // Scroll to bottom
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Handle tool requests
function handleToolRequest(data) {
    removeThinkingIndicator(); // Remove thinking when tool starts
    
    // Reset streaming message so tool doesn't interfere with message flow
    currentStreamingMessage = null;
    
    const toolDiv = document.createElement('div');
    toolDiv.className = 'message assistant tool-message';
    
    const headerDiv = document.createElement('div');
    headerDiv.className = 'tool-header';
    headerDiv.innerHTML = `🔧 <strong>${data.tool_name}</strong>`;
    
    const contentDiv = document.createElement('div');
    contentDiv.className = 'tool-content';
    
    // Format the arguments
    if (data.tool_name === 'developer__shell' && data.arguments.command) {
        contentDiv.innerHTML = `<pre><code>${escapeHtml(data.arguments.command)}</code></pre>`;
    } else if (data.tool_name === 'developer__text_editor') {
        const action = data.arguments.command || 'unknown';
        const path = data.arguments.path || 'unknown';
        contentDiv.innerHTML = `<div class="tool-param"><strong>action:</strong> ${action}</div>`;
        contentDiv.innerHTML += `<div class="tool-param"><strong>path:</strong> ${escapeHtml(path)}</div>`;
        if (data.arguments.file_text) {
            contentDiv.innerHTML += `<div class="tool-param"><strong>content:</strong> <pre><code>${escapeHtml(data.arguments.file_text.substring(0, 200))}${data.arguments.file_text.length > 200 ? '...' : ''}</code></pre></div>`;
        }
    } else {
        contentDiv.innerHTML = `<pre><code>${JSON.stringify(data.arguments, null, 2)}</code></pre>`;
    }
    
    toolDiv.appendChild(headerDiv);
    toolDiv.appendChild(contentDiv);
    
    // Add a "running" indicator
    const runningDiv = document.createElement('div');
    runningDiv.className = 'tool-running';
    runningDiv.innerHTML = '⏳ Running...';
    toolDiv.appendChild(runningDiv);
    
    messagesContainer.appendChild(toolDiv);
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Handle tool responses
function handleToolResponse(data) {
    // Remove the "running" indicator from the last tool message
    const toolMessages = messagesContainer.querySelectorAll('.tool-message');
    if (toolMessages.length > 0) {
        const lastToolMessage = toolMessages[toolMessages.length - 1];
        const runningIndicator = lastToolMessage.querySelector('.tool-running');
        if (runningIndicator) {
            runningIndicator.remove();
        }
    }
    
    if (data.is_error) {
        const errorDiv = document.createElement('div');
        errorDiv.className = 'message tool-error';
        errorDiv.innerHTML = `<strong>Tool Error:</strong> ${escapeHtml(data.result.error || 'Unknown error')}`;
        messagesContainer.appendChild(errorDiv);
    } else {
        // Handle successful tool response
        if (Array.isArray(data.result)) {
            data.result.forEach(content => {
                if (content.type === 'text' && content.text) {
                    const responseDiv = document.createElement('div');
                    responseDiv.className = 'message tool-result';
                    responseDiv.innerHTML = `<pre>${escapeHtml(content.text)}</pre>`;
                    messagesContainer.appendChild(responseDiv);
                }
            });
        }
    }
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
    
    // Reset streaming message so next assistant response creates a new message
    currentStreamingMessage = null;
    
    // Show thinking indicator because assistant will likely follow up with explanation
    // Only show if we're still processing (cancel button is active)
    if (isProcessing) {
        addThinkingIndicator();
    }
}

// Handle tool confirmations
function handleToolConfirmation(data) {
    const confirmDiv = document.createElement('div');
    confirmDiv.className = 'message tool-confirmation';
    confirmDiv.innerHTML = `
        <div class="tool-confirm-header">⚠️ Tool Confirmation Required</div>
        <div class="tool-confirm-content">
            <strong>${data.tool_name}</strong> wants to execute with:
            <pre><code>${JSON.stringify(data.arguments, null, 2)}</code></pre>
        </div>
        <div class="tool-confirm-note">Auto-approved in web mode (UI coming soon)</div>
    `;
    messagesContainer.appendChild(confirmDiv);
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Handle thinking messages
function handleThinking(data) {
    // For now, just log thinking messages
    console.log('Thinking:', data.message);
}

// Handle context exceeded
function handleContextExceeded(data) {
    const contextDiv = document.createElement('div');
    contextDiv.className = 'message context-warning';
    contextDiv.innerHTML = `
        <div class="context-header">⚠️ Context Length Exceeded</div>
        <div class="context-content">${escapeHtml(data.message)}</div>
        <div class="context-note">Auto-summarizing conversation...</div>
    `;
    messagesContainer.appendChild(contextDiv);
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Handle cancelled operation
function handleCancelled(data) {
    removeThinkingIndicator();
    resetSendButton();
    
    const cancelDiv = document.createElement('div');
    cancelDiv.className = 'message system-message cancelled';
    cancelDiv.innerHTML = `<em>${escapeHtml(data.message)}</em>`;
    messagesContainer.appendChild(cancelDiv);
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Handle completion of response
function handleComplete(data) {
    removeThinkingIndicator();
    resetSendButton();
    // Finalize any streaming message
    if (currentStreamingMessage) {
        currentStreamingMessage = null;
    }
}

// Reset send button to normal state
function resetSendButton() {
    isProcessing = false;
    sendButton.textContent = 'Send';
    sendButton.classList.remove('cancel-mode');
}

// Escape HTML to prevent XSS
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Send message or cancel
function sendMessage() {
    if (isProcessing) {
        // Cancel the current operation
        socket.send(JSON.stringify({
            type: 'cancel',
            session_id: sessionId
        }));
        return;
    }
    
    const message = messageInput.value.trim();
    if (!message || !isConnected) return;
    
    // Add user message to chat
    addMessage(message, 'user', Date.now());
    
    // Clear input
    messageInput.value = '';
    messageInput.style.height = 'auto';
    
    // Add thinking indicator
    addThinkingIndicator();
    
    // Update button to show cancel
    isProcessing = true;
    sendButton.textContent = 'Cancel';
    sendButton.classList.add('cancel-mode');
    
    // Send message through WebSocket
    socket.send(JSON.stringify({
        type: 'message',
        content: message,
        session_id: sessionId,
        timestamp: Date.now()
    }));
}

// Handle suggestion pill clicks
function sendSuggestion(text) {
    if (!isConnected || isProcessing) return;
    
    messageInput.value = text;
    sendMessage();
}

// Load session history if the session exists (like --resume in CLI)
async function loadSessionIfExists() {
    try {
        const response = await fetch(`/api/sessions/${sessionId}`);
        if (response.ok) {
            const sessionData = await response.json();
            if (sessionData.messages && sessionData.messages.length > 0) {
                // Remove welcome message since we're resuming
                const welcomeMessage = messagesContainer.querySelector('.welcome-message');
                if (welcomeMessage) {
                    welcomeMessage.remove();
                }
                
                // Display session resumed message
                const resumeDiv = document.createElement('div');
                resumeDiv.className = 'message system-message';
                resumeDiv.innerHTML = `<em>Session resumed: ${sessionData.messages.length} messages loaded</em>`;
                messagesContainer.appendChild(resumeDiv);
                
                
                // Update page title with session description if available
                if (sessionData.metadata && sessionData.metadata.description) {
                    document.title = `Goose Chat - ${sessionData.metadata.description}`;
                }
                
                messagesContainer.scrollTop = messagesContainer.scrollHeight;
            }
        }
    } catch (error) {
        console.log('No existing session found or error loading:', error);
        // This is fine - just means it's a new session
    }
}


// Event listeners
sendButton.addEventListener('click', sendMessage);

messageInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        sendMessage();
    }
});

// Auto-resize textarea
messageInput.addEventListener('input', () => {
    messageInput.style.height = 'auto';
    messageInput.style.height = messageInput.scrollHeight + 'px';
});

// Initialize WebSocket connection
connectWebSocket();

// Focus on input
messageInput.focus();

// Update session title
function updateSessionTitle() {
    const titleElement = document.getElementById('session-title');
    // Just show "Goose Chat" - no need to show session ID
    titleElement.textContent = 'Goose Chat';
}

// Update title on load
updateSessionTitle();