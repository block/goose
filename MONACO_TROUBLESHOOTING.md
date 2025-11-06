# 🔧 Monaco Integration Troubleshooting

## Issue: Spinner Stuck on Loading

### What I Fixed:

1. **Added CDN Loader Configuration** ✅
   - File: `MonacoCodeInput.tsx`
   - Added: `loader.config()` to load Monaco from CDN
   - This ensures Monaco workers load correctly in Electron/Vite

2. **Updated Vite Configuration** ✅
   - File: `vite.config.mts`
   - Added: `optimizeDeps` and `worker` configuration
   - This helps Vite bundle Monaco correctly

3. **Added Debug Logging** ✅
   - Added `beforeMount` callback with console.log
   - Added `onValidate` callback to catch errors
   - Check browser DevTools console for messages

---

## 🔍 How to Debug

### Step 1: Restart the App
```bash
# Stop the current app (Cmd+Q or close window)
# Then restart:
cd /Users/spencermartin/Desktop/goose/ui/desktop
npm run start-gui
```

### Step 2: Open DevTools
- In the app, press: **Cmd+Option+I** (or Cmd+Shift+I)
- Go to the **Console** tab
- Look for these messages:
  - `🎯 Monaco beforeMount called` - Monaco is loading
  - Any error messages in red

### Step 3: Test Code Mode
1. Type `#python ` in chat
2. Watch the console for messages
3. If you see errors, share them with me!

---

## 🐛 Common Issues & Fixes

### Issue 1: "Failed to fetch Monaco"
**Cause**: Network issue or CDN blocked  
**Fix**: Check internet connection, or try local Monaco

### Issue 2: "Worker failed to load"
**Cause**: Vite worker configuration  
**Fix**: Already fixed in vite.config.mts

### Issue 3: Spinner never stops
**Cause**: Monaco not loading from CDN  
**Fix**: Check console for errors, may need to use local Monaco

### Issue 4: "Cannot find module"
**Cause**: Monaco not installed  
**Fix**: Run `npm install` in ui/desktop

---

## 🔄 Alternative: Use Local Monaco (If CDN Fails)

If the CDN approach doesn't work, we can switch to local Monaco:

### Option A: Install Monaco Editor Directly
```bash
cd /Users/spencermartin/Desktop/goose/ui/desktop
npm install monaco-editor
```

Then update `MonacoCodeInput.tsx`:
```typescript
// Remove the loader.config() line
// Monaco will use local files instead
```

### Option B: Use Vite Plugin
```bash
npm install vite-plugin-monaco-editor --save-dev
```

Then update `vite.config.mts`:
```typescript
import monacoEditorPlugin from 'vite-plugin-monaco-editor';

export default defineConfig({
  plugins: [
    react(),
    monacoEditorPlugin({
      languageWorkers: ['editorWorkerService', 'typescript', 'json', 'html', 'css']
    })
  ],
  // ... rest of config
});
```

---

## 📊 What to Check

### In Browser Console:
- [ ] Look for `🎯 Monaco beforeMount called`
- [ ] Check for any red error messages
- [ ] Look for network errors (Failed to fetch)
- [ ] Check if Monaco files are loading (Network tab)

### In Network Tab:
- [ ] Look for requests to `cdn.jsdelivr.net`
- [ ] Check if Monaco files (*.js) are loading
- [ ] Look for 404 or CORS errors

### In Elements Tab:
- [ ] Find the `.monaco-code-input-wrapper` div
- [ ] Check if Monaco editor HTML is inside
- [ ] Look for any error overlays

---

## 🚀 Next Steps

1. **Restart the app** with the fixes
2. **Open DevTools** (Cmd+Option+I)
3. **Try code mode** (#python )
4. **Check console** for messages
5. **Share any errors** you see

---

## 💡 Quick Test

Try this in the console after typing `#python `:

```javascript
// Check if Monaco loaded
console.log('Monaco loaded?', window.monaco);

// Check if loader is configured
console.log('Loader config:', window.require);
```

---

## 📝 What Changed

### Files Modified:
1. `MonacoCodeInput.tsx` - Added CDN loader config
2. `vite.config.mts` - Added optimizeDeps and worker config

### Why These Changes:
- Monaco needs workers to run (for syntax highlighting, autocomplete, etc.)
- Vite/Electron environment needs special configuration
- CDN approach is most reliable for Electron apps

---

## 🆘 If Still Stuck

Share these with me:
1. Console errors (screenshot or copy/paste)
2. Network tab errors
3. Any error overlays in the app

I'll help you get it working! 🎯
