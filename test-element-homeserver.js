// Test Element.io homeserver
const testElementHomeserver = async () => {
  try {
    console.log('🧪 Testing Element.io homeserver...');
    
    // Test if homeserver is reachable
    const versionsResponse = await fetch('https://matrix-client.matrix.org/_matrix/client/versions');
    const versionsData = await versionsResponse.json();
    console.log('✅ Element.io homeserver is reachable');
    console.log('Supported versions:', versionsData.versions);
    
    // Test registration endpoint
    const registerResponse = await fetch('https://matrix-client.matrix.org/_matrix/client/v3/register', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ 
        username: 'testuser_' + Date.now(), 
        password: 'testpassword123',
        auth: { type: 'm.login.dummy' }
      })
    });
    
    console.log('Registration response status:', registerResponse.status);
    
    if (registerResponse.status === 401) {
      const errorData = await registerResponse.json();
      console.log('✅ Registration requires additional auth steps (this is normal)');
      console.log('Available auth flows:', errorData.flows);
    } else if (registerResponse.status === 400) {
      console.log('✅ Registration endpoint is working (400 = validation error, which is expected)');
    } else if (registerResponse.status === 403) {
      console.log('❌ Registration is disabled on this homeserver');
    } else {
      console.log('Registration response:', await registerResponse.json());
    }
    
  } catch (error) {
    console.error('❌ Element.io homeserver test failed:', error);
  }
};

// Alternative homeservers to try
const testAlternativeHomeservers = async () => {
  const homeservers = [
    'https://matrix.tchncs.de',
    'https://matrix.envs.net',
    'https://matrix.allmende.io'
  ];
  
  for (const homeserver of homeservers) {
    try {
      console.log(`\n🧪 Testing ${homeserver}...`);
      
      const response = await fetch(`${homeserver}/_matrix/client/versions`);
      if (response.ok) {
        console.log(`✅ ${homeserver} is reachable`);
        
        // Test registration
        const regResponse = await fetch(`${homeserver}/_matrix/client/v3/register`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ username: 'test', password: 'test' })
        });
        
        if (regResponse.status !== 403) {
          console.log(`✅ ${homeserver} allows registration (status: ${regResponse.status})`);
        } else {
          console.log(`❌ ${homeserver} has registration disabled`);
        }
      }
    } catch (error) {
      console.log(`❌ ${homeserver} failed:`, error.message);
    }
  }
};

// Run tests
testElementHomeserver().then(() => {
  console.log('\n--- Testing Alternative Homeservers ---');
  return testAlternativeHomeservers();
});
