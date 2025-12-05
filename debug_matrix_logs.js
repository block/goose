// Matrix Debug Log Filter Script
// Copy and paste this into your browser's developer console

// Store original console methods
const originalLog = console.log;
const originalError = console.error;
const originalWarn = console.warn;

// Create a filtered log collector
const matrixLogs = [];

// Override console methods to capture Matrix-related logs
console.log = function(...args) {
  const message = args.join(' ');
  
  // Check for Matrix-related log patterns
  if (message.includes('🔧 useChatStream called with Matrix params:') ||
      message.includes('🔍 Matrix history loading effect triggered') ||
      message.includes('📚 Loading Matrix room history') ||
      message.includes('✅ Matrix history loaded successfully') ||
      message.includes('❌ Failed to load Matrix history') ||
      message.includes('💬 Opening room in chat:') ||
      message.includes('useChatStream render') ||
      message.includes('BaseChat2 render state:') ||
      message.includes('messagesLength:') ||
      message.includes('hasSession:') ||
      message.includes('Matrix') ||
      message.includes('matrix')) {
    
    const timestamp = new Date().toLocaleTimeString();
    const logEntry = `[${timestamp}] ${message}`;
    matrixLogs.push(logEntry);
    
    // Also display in a highlighted way
    originalLog('%c' + logEntry, 'background: #e3f2fd; color: #1976d2; padding: 2px 4px; border-radius: 3px;');
  } else {
    // Call original log for non-Matrix logs
    originalLog.apply(console, arguments);
  }
};

// Also capture errors and warnings
console.error = function(...args) {
  const message = args.join(' ');
  if (message.includes('Matrix') || message.includes('matrix')) {
    const timestamp = new Date().toLocaleTimeString();
    const logEntry = `[${timestamp}] ERROR: ${message}`;
    matrixLogs.push(logEntry);
    originalLog('%c' + logEntry, 'background: #ffebee; color: #d32f2f; padding: 2px 4px; border-radius: 3px;');
  }
  originalError.apply(console, arguments);
};

console.warn = function(...args) {
  const message = args.join(' ');
  if (message.includes('Matrix') || message.includes('matrix')) {
    const timestamp = new Date().toLocaleTimeString();
    const logEntry = `[${timestamp}] WARN: ${message}`;
    matrixLogs.push(logEntry);
    originalLog('%c' + logEntry, 'background: #fff3e0; color: #f57c00; padding: 2px 4px; border-radius: 3px;');
  }
  originalWarn.apply(console, arguments);
};

// Helper functions
window.showMatrixLogs = function() {
  console.group('📋 All Matrix Logs');
  matrixLogs.forEach(log => originalLog(log));
  console.groupEnd();
};

window.clearMatrixLogs = function() {
  matrixLogs.length = 0;
  originalLog('🧹 Matrix logs cleared');
};

window.restoreConsole = function() {
  console.log = originalLog;
  console.error = originalError;
  console.warn = originalWarn;
  originalLog('🔄 Console restored to original state');
};

// Initial setup message
originalLog('%c🔍 Matrix Debug Logger Active', 'background: #4caf50; color: white; padding: 4px 8px; border-radius: 4px; font-weight: bold;');
originalLog('Available commands:');
originalLog('  showMatrixLogs() - Show all captured Matrix logs');
originalLog('  clearMatrixLogs() - Clear the log buffer');
originalLog('  restoreConsole() - Restore original console behavior');
originalLog('');
originalLog('Now try opening a Matrix room and the relevant logs will be highlighted...');
