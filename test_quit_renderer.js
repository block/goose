// Renderer process test interface for quit detection
// Add this to your preload.js or create a test component

// Test interface for renderer process
const quitTestInterface = {
  // Test individual scenarios
  testQuitScenario: (scenario) => ipcRenderer.invoke('test-quit-scenario', scenario),
  
  // Get test results
  getQuitTestResults: () => ipcRenderer.invoke('get-quit-test-results'),
  
  // Clear test results
  clearQuitTestResults: () => ipcRenderer.invoke('clear-quit-test-results'),
  
  // Simulate system events
  simulateSystemEvent: (eventType) => ipcRenderer.invoke('simulate-system-event', eventType),
  
  // Test the actual quit initiator detection
  testQuitInitiatorDetection: async () => {
    try {
      const quitInfo = await ipcRenderer.invoke('get-quit-initiator');
      console.log('🧪 Current quit initiator info:', quitInfo);
      return quitInfo;
    } catch (error) {
      console.error('🧪 Failed to get quit initiator:', error);
      return null;
    }
  }
};

// Make available globally in renderer
if (typeof window !== 'undefined') {
  window.quitTester = quitTestInterface;
}

// Example React component for testing
const QuitTestPanel = () => {
  const [testResults, setTestResults] = React.useState([]);
  const [currentQuitInfo, setCurrentQuitInfo] = React.useState(null);

  const runTest = async (scenario) => {
    try {
      const result = await window.quitTester.testQuitScenario(scenario);
      console.log(`🧪 Test result for ${scenario}:`, result);
      
      // Refresh results
      const allResults = await window.quitTester.getQuitTestResults();
      setTestResults(allResults);
    } catch (error) {
      console.error(`🧪 Test failed for ${scenario}:`, error);
    }
  };

  const checkCurrentState = async () => {
    const info = await window.quitTester.testQuitInitiatorDetection();
    setCurrentQuitInfo(info);
  };

  const simulateEvent = async (eventType) => {
    try {
      const result = await window.quitTester.simulateSystemEvent(eventType);
      console.log(`🧪 Simulated ${eventType}:`, result);
      
      // Check current state after simulation
      await checkCurrentState();
    } catch (error) {
      console.error(`🧪 Failed to simulate ${eventType}:`, error);
    }
  };

  return (
    <div style={{ padding: '20px', border: '1px solid #ccc', margin: '10px' }}>
      <h3>🧪 Quit Detection Testing Panel</h3>
      
      <div style={{ marginBottom: '20px' }}>
        <h4>Current State</h4>
        <button onClick={checkCurrentState}>Check Current Quit Info</button>
        {currentQuitInfo && (
          <pre style={{ background: '#f5f5f5', padding: '10px' }}>
            {JSON.stringify(currentQuitInfo, null, 2)}
          </pre>
        )}
      </div>

      <div style={{ marginBottom: '20px' }}>
        <h4>Test Scenarios</h4>
        <button onClick={() => runTest('user-quit')}>Test User Quit</button>
        <button onClick={() => runTest('system-shutdown')}>Test System Shutdown</button>
        <button onClick={() => runTest('user-logout')}>Test User Logout</button>
        <button onClick={() => runTest('app-quit')}>Test App Quit</button>
        <button onClick={() => runTest('all-tests')}>Run All Tests</button>
      </div>

      <div style={{ marginBottom: '20px' }}>
        <h4>Simulate System Events</h4>
        <button onClick={() => simulateEvent('shutdown')}>Simulate Shutdown</button>
        <button onClick={() => simulateEvent('user-resign')}>Simulate User Logout</button>
        <button onClick={() => simulateEvent('reset')}>Reset State</button>
      </div>

      <div>
        <h4>Test Results</h4>
        <button onClick={async () => {
          const results = await window.quitTester.getQuitTestResults();
          setTestResults(results);
        }}>Refresh Results</button>
        <button onClick={async () => {
          await window.quitTester.clearQuitTestResults();
          setTestResults([]);
        }}>Clear Results</button>
        
        {testResults.length > 0 && (
          <div style={{ maxHeight: '300px', overflow: 'auto', background: '#f5f5f5', padding: '10px' }}>
            {testResults.map((result, index) => (
              <div key={index} style={{ marginBottom: '10px', borderBottom: '1px solid #ddd', paddingBottom: '5px' }}>
                <strong>{result.reason}</strong> ({result.initiator})
                <br />
                <small>{result.timestamp}</small>
                <br />
                System Shutdown: {result.isSystemShutdown ? 'Yes' : 'No'}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

console.log('🧪 Quit test interface loaded. Use window.quitTester for testing.');
