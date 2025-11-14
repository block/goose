// Quick test script for Matrix connection
const testMatrixConnection = async () => {
  try {
    // Test if matrix.org is reachable
    const response = await fetch('https://matrix.org/_matrix/client/versions');
    const data = await response.json();
    console.log('✅ Matrix.org is reachable');
    console.log('Supported versions:', data.versions);
    
    // Test registration endpoint
    const registerTest = await fetch('https://matrix.org/_matrix/client/r0/register', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: 'test', password: 'test' })
    });
    
    console.log('Registration endpoint status:', registerTest.status);
    if (registerTest.status === 400) {
      console.log('✅ Registration endpoint is working (400 = missing auth, which is expected)');
    }
    
  } catch (error) {
    console.error('❌ Matrix connection failed:', error);
  }
};

// Run the test
testMatrixConnection();
