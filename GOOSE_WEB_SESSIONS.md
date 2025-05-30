# Goose Web Interface - CLI Session Semantics

## Session Management - Same as CLI!

The `goose web` interface now works **exactly like `goose session`** with identical semantics:

### 🎯 **URL Patterns (Like CLI Commands)**

| CLI Command | Web URL | Behavior |
|-------------|---------|----------|
| `goose session` | `http://localhost:8080/` | Auto-generate new session ID |
| `goose session --name my-project` | `http://localhost:8080/session/my-project` | Use named session |
| `goose session --name my-project --resume` | `http://localhost:8080/session/my-project` | Resume if exists, create if not |
| | `http://localhost:8080/?session=my-project` | Alternative URL parameter format |

### ✅ **Identical Session Behavior**

1. **Auto-generated Sessions**
   - Visit `/` → Creates new session with timestamp ID (`20250529_160430`)
   - Same format as CLI: `yyyymmdd_hhmmss`

2. **Named Sessions**
   - Visit `/session/my-project` → Uses session name "my-project"
   - Creates new session if doesn't exist
   - Resumes existing session if found

3. **Resume Functionality**
   - Automatically loads message history when session exists
   - Shows "Session resumed: X messages loaded" 
   - Updates page title with session description
   - No need for explicit `--resume` flag (always resumes if exists)

### 🔄 **Cross-Platform Session Management**

```bash
# CLI: Create named session
goose session --name web-test

# Web: Continue same session
open http://localhost:8080/session/web-test

# CLI: Resume web session
goose session --name 20250529_160430 --resume

# CLI: List all sessions (includes web sessions)
goose session list
```

### 📁 **Session Storage & Compatibility**

- **Same JSONL format**: Identical to CLI sessions
- **Same location**: `~/.local/share/goose/sessions/`
- **Same metadata**: Descriptions, token counts, working directory
- **Automatic descriptions**: Generated after 1st/3rd message
- **Full interoperability**: Sessions work seamlessly between CLI and web

### 🌐 **Web-Specific Features**

**Session Management UI:**
- Header shows current session name
- Page title updates with session description
- Visual "Session resumed" indicator

**API Endpoints:**
- `GET /api/sessions` - List all sessions
- `GET /api/sessions/{name}` - Get session details

### 📝 **Usage Examples**

```bash
# Start web server
goose web --port 8080

# Use cases:
# 1. Quick new session
open http://localhost:8080/

# 2. Named project session  
open http://localhost:8080/session/my-project

# 3. Resume specific session
open http://localhost:8080/session/20250529_160430

# 4. URL parameter format
open "http://localhost:8080/?session=my-project"
```

### 🎯 **Key Benefits**

- ✅ **Identical semantics**: Works exactly like `goose session`
- ✅ **No learning curve**: Same patterns as CLI users know
- ✅ **Full compatibility**: Sessions work in both interfaces
- ✅ **Automatic resuming**: No explicit resume flag needed
- ✅ **Named sessions**: Use meaningful names for projects
- ✅ **History preservation**: Complete conversation context maintained

The web interface now provides the **exact same session experience** as the CLI, making it a true drop-in replacement for interactive Goose usage!