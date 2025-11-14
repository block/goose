# Matrix Setup Guide for Goose P2P

## 🚀 Quick Start

### 1. **Choose a Homeserver**

Since Matrix.org has disabled registration, use one of these alternatives:

#### **Recommended: Element.io Homeserver**
- URL: `https://matrix-client.matrix.org`
- ✅ Open registration
- ✅ Reliable and fast
- ✅ Official Element homeserver

#### **Alternative: Tchncs.de**
- URL: `https://matrix.tchncs.de`
- ✅ Privacy-focused
- ✅ European-based
- ✅ Good community

### 2. **Update Your Configuration**

The homeserver is already updated in your code to use Element.io:

```typescript
// In MatrixService.ts
export const matrixService = new MatrixService({
  homeserverUrl: 'https://matrix-client.matrix.org',
});
```

### 3. **Test Your Setup**

Run your Goose app and try to:
1. Register a new Matrix account
2. Create a collaborative session
3. Invite friends to the session

## 🛠 **Development Setup**

### **Option A: Use Public Homeserver (Easiest)**

1. Your app is already configured for Element.io
2. Users register through your app's UI
3. No additional setup needed!

### **Option B: Local Development Server**

For development, you can run a local Matrix server:

```bash
# Using Docker
docker run -it --rm \
  -p 8008:8008 \
  -v $(pwd)/synapse-data:/data \
  matrixdotorg/synapse:latest generate

# Edit the config to enable registration
# In synapse-data/homeserver.yaml:
# enable_registration: true

# Start the server
docker run -it --rm \
  -p 8008:8008 \
  -v $(pwd)/synapse-data:/data \
  matrixdotorg/synapse:latest
```

Then update your MatrixService:
```typescript
export const matrixService = new MatrixService({
  homeserverUrl: 'http://localhost:8008',
});
```

## 🔧 **Production Setup**

For production deployment:

### **Option 1: Use Managed Hosting**
- **Element Matrix Services**: https://element.io/matrix-services
- **Modular.im**: https://www.modular.im/
- **EMS**: Professional Matrix hosting

### **Option 2: Self-Host with Docker**

```yaml
# docker-compose.yml
version: '3.8'
services:
  synapse:
    image: matrixdotorg/synapse:latest
    ports:
      - "8008:8008"
    volumes:
      - ./synapse:/data
    environment:
      - SYNAPSE_SERVER_NAME=your-domain.com
      - SYNAPSE_REPORT_STATS=no

  postgres:
    image: postgres:13
    environment:
      POSTGRES_DB: synapse
      POSTGRES_USER: synapse
      POSTGRES_PASSWORD: your-secure-password
    volumes:
      - ./postgres:/var/lib/postgresql/data
```

### **Option 3: Use Matrix.org with Application Service**

If you want to use matrix.org, you'd need to register as an Application Service, but this is overkill for most use cases.

## 🧪 **Testing Your Setup**

### **Manual Test**
1. Open your Goose app
2. Go to Settings > Peers
3. Try to register a new account
4. Create a collaborative session
5. Invite another user (you can test with a second browser/account)

### **Automated Test**
```javascript
// Test homeserver connectivity
const testHomeserver = async (url) => {
  try {
    const response = await fetch(`${url}/_matrix/client/versions`);
    const data = await response.json();
    console.log('✅ Homeserver reachable:', data.versions);
    
    // Test registration
    const regResponse = await fetch(`${url}/_matrix/client/v3/register`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: 'test', password: 'test' })
    });
    
    if (regResponse.status !== 403) {
      console.log('✅ Registration is enabled');
    } else {
      console.log('❌ Registration is disabled');
    }
  } catch (error) {
    console.error('❌ Connection failed:', error);
  }
};

// Test Element.io homeserver
testHomeserver('https://matrix-client.matrix.org');
```

## 🔐 **Security Considerations**

### **For Development**
- Use test accounts only
- Don't share sensitive information
- Local homeservers are fine for testing

### **For Production**
- Use HTTPS only
- Enable end-to-end encryption
- Regular backups
- Monitor server resources
- Consider rate limiting

## 🆘 **Troubleshooting**

### **Registration Issues**
```
Error: [403] Registration has been disabled
```
**Solution**: Switch to a different homeserver that allows registration.

### **Connection Issues**
```
Error: Failed to fetch
```
**Solutions**:
- Check internet connection
- Verify homeserver URL
- Check for CORS issues (in development)
- Try a different homeserver

### **CORS Issues (Development)**
If you get CORS errors in development:
1. Use a homeserver that supports CORS
2. Or run your own local server
3. Or use a CORS proxy (not recommended for production)

## 📚 **Resources**

- **Matrix Specification**: https://spec.matrix.org/
- **Synapse Documentation**: https://matrix-org.github.io/synapse/
- **Matrix.js SDK**: https://github.com/matrix-org/matrix-js-sdk
- **Public Homeserver List**: https://joinmatrix.org/servers/
- **Element Web**: https://app.element.io/ (for testing accounts)

## 🎯 **Next Steps**

1. ✅ Update homeserver URL (already done)
2. ✅ Test registration in your app
3. ✅ Create collaborative sessions
4. ✅ Test with multiple users
5. 🔄 Add error handling for network issues
6. 🔄 Implement session persistence
7. 🔄 Add end-to-end encryption support
