const { app, powerMonitor, BrowserWindow } = require('electron');

// Mock testing utility for quit initiator scenarios
class QuitInitiatorTester {
  constructor() {
    this.originalQuitApp = null;
    this.testResults = [];
  }

  // Mock the quitApp function to capture calls instead of actually quitting
  mockQuitApp() {
    // Store reference to original function if it exists
    if (global.quitApp) {
      this.originalQuitApp = global.quitApp;
    }

    // Replace with mock that logs instead of quitting
    global.quitApp = (reason, initiator = 'user', callStack = new Error().stack) => {
      const testResult = {
        timestamp: new Date().toISOString(),
        reason,
        initiator,
        callStack: callStack.split('\n').slice(0, 5).join('\n'), // First 5 lines
        isSystemShutdown: global.isSystemShutdown || false
      };
      
      this.testResults.push(testResult);
      console.log('🧪 MOCK QUIT CALLED:', testResult);
      
      // Don't actually quit, just log
      return testResult;
    };
  }

  // Restore original quitApp function
  restoreQuitApp() {
    if (this.originalQuitApp) {
      global.quitApp = this.originalQuitApp;
    }
  }

  // Simulate system shutdown event
  testSystemShutdown() {
    console.log('🧪 Testing system shutdown scenario...');
    
    // Simulate the powerMonitor shutdown event
    global.isSystemShutdown = true;
    
    // Trigger the quit with system initiator
    if (global.quitApp) {
      global.quitApp('system shutdown/reboot detected', 'system');
    }
    
    return this.getLastResult();
  }

  // Simulate user logout
  testUserLogout() {
    console.log('🧪 Testing user logout scenario...');
    
    // Simulate the powerMonitor user-did-resign-active event
    global.isSystemShutdown = true; // This gets set in the actual handler
    
    if (global.quitApp) {
      global.quitApp('user logout/session switch detected', 'system');
    }
    
    return this.getLastResult();
  }

  // Simulate user-initiated quit
  testUserQuit() {
    console.log('🧪 Testing user-initiated quit...');
    
    global.isSystemShutdown = false;
    
    if (global.quitApp) {
      global.quitApp('user requested quit', 'user');
    }
    
    return this.getLastResult();
  }

  // Simulate app-initiated quit (like single instance check)
  testAppQuit() {
    console.log('🧪 Testing app-initiated quit...');
    
    global.isSystemShutdown = false;
    
    if (global.quitApp) {
      global.quitApp('single instance lock not acquired', 'system');
    }
    
    return this.getLastResult();
  }

  // Simulate squirrel startup quit
  testSquirrelQuit() {
    console.log('🧪 Testing squirrel startup quit...');
    
    global.isSystemShutdown = false;
    
    if (global.quitApp) {
      global.quitApp('electron-squirrel-startup detected', 'system');
    }
    
    return this.getLastResult();
  }

  // Test the before-quit event handler
  testBeforeQuitEvent() {
    console.log('🧪 Testing before-quit event scenarios...');
    
    const results = [];
    
    // Test with system shutdown
    global.isSystemShutdown = true;
    const systemEvent = { preventDefault: () => console.log('preventDefault called') };
    // You'd call your actual before-quit handler here
    results.push({ scenario: 'system-shutdown', shouldPreventDefault: false });
    
    // Test with user quit
    global.isSystemShutdown = false;
    const userEvent = { preventDefault: () => console.log('preventDefault called') };
    // You'd call your actual before-quit handler here
    results.push({ scenario: 'user-quit', shouldPreventDefault: true });
    
    return results;
  }

  // Get the last test result
  getLastResult() {
    return this.testResults[this.testResults.length - 1] || null;
  }

  // Get all test results
  getAllResults() {
    return this.testResults;
  }

  // Clear test results
  clearResults() {
    this.testResults = [];
  }

  // Run all tests
  runAllTests() {
    console.log('🧪 Running all quit initiator tests...\n');
    
    this.clearResults();
    this.mockQuitApp();
    
    const tests = [
      () => this.testUserQuit(),
      () => this.testSystemShutdown(),
      () => this.testUserLogout(),
      () => this.testAppQuit(),
      () => this.testSquirrelQuit()
    ];
    
    tests.forEach((test, index) => {
      console.log(`\n--- Test ${index + 1} ---`);
      test();
    });
    
    console.log('\n🧪 All tests completed!');
    console.log('📊 Test Summary:');
    this.testResults.forEach((result, index) => {
      console.log(`${index + 1}. ${result.reason} (${result.initiator})`);
    });
    
    this.restoreQuitApp();
    return this.testResults;
  }
}

// Export for use in main process
if (typeof module !== 'undefined' && module.exports) {
  module.exports = QuitInitiatorTester;
}

// Global instance for console testing
if (typeof global !== 'undefined') {
  global.QuitTester = new QuitInitiatorTester();
}

console.log('🧪 Quit Initiator Tester loaded. Use global.QuitTester to run tests.');

// Auto-run tests if GOOSE_TEST_QUIT is set
if (process.env.GOOSE_TEST_QUIT === 'true') {
  setTimeout(() => {
    console.log('🧪 Auto-running quit tests due to GOOSE_TEST_QUIT=true');
    global.QuitTester.runAllTests();
  }, 2000);
}
