# Matrix Homeservers with Open Registration

## Recommended Public Homeservers

### 1. **Element.io** (Most Popular)
- URL: `https://matrix-client.matrix.org`
- Registration: Open
- Reliable and well-maintained
- Good for production use

### 2. **Tchncs.de**
- URL: `https://matrix.tchncs.de`
- Registration: Open
- European-based, privacy-focused
- Good community

### 3. **Envs.net**
- URL: `https://matrix.envs.net`
- Registration: Open
- Smaller community server

### 4. **Matrix.allmende.io**
- URL: `https://matrix.allmende.io`
- Registration: Open
- German-based server

## How to Test a Homeserver

```bash
# Test if homeserver supports registration
curl -X POST "https://matrix-client.matrix.org/_matrix/client/v3/register" \
  -H "Content-Type: application/json" \
  -d '{"username":"test","password":"test"}'

# Should return 400 (missing auth) not 403 (disabled)
```

## Update Your MatrixService

```typescript
// In MatrixService.ts, change the default homeserver:
export const matrixService = new MatrixService({
  homeserverUrl: 'https://matrix-client.matrix.org', // Element.io
});
```
