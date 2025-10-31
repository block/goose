# 🎨 Live IDE Input - Usage Guide

## ✨ What is it?

A **live code editor** built directly into the chat input! Type `#python` (or any supported language) and start coding with real-time syntax highlighting.

## 🚀 How to Use

### Step 1: Activate Code Mode
Type a language trigger at the start of your message:
- `#python`
- `#javascript`
- `#typescript`
- `#java`
- `#cpp`
- `#go`
- `#rust`
- ...and 25+ more!

### Step 2: Start Coding
Once you type the trigger and a space, you'll see:
- 🏷️ **Language badge** appears
- 🎨 **Syntax highlighting** activates
- 💻 **IDE-style background**

### Step 3: Write Your Code
- **Type naturally** - syntax highlighting updates in real-time
- **Press Enter** - Adds a newline (doesn't send the message)
- **Press Cmd+Enter** (Mac) or **Ctrl+Enter** (Windows/Linux) - Sends the message

### Step 4: Exit Code Mode
- Delete the `#language` trigger to exit code mode
- Or just send your message with Cmd/Ctrl+Enter

## 📝 Examples

### Example 1: Python
```
Type: #python 
def greet(name):
    print(f"Hello, {name}!")

greet("World")
```

### Example 2: JavaScript
```
Type: #javascript 
const fetchData = async (url) => {
  const response = await fetch(url);
  return await response.json();
};
```

### Example 3: TypeScript
```
Type: #typescript 
interface User {
  name: string;
  age: number;
}

const greet = (user: User): string => {
  return `Hello, ${user.name}!`;
};
```

## 🎯 Supported Languages

### Web Development
- `#javascript` / `#js`
- `#typescript` / `#ts`
- `#jsx`
- `#tsx`
- `#html`
- `#css`

### Backend & Systems
- `#python`
- `#java`
- `#cpp` / `#c`
- `#go`
- `#rust`
- `#php`
- `#ruby`

### Mobile
- `#swift`
- `#kotlin`
- `#dart`

### Functional
- `#haskell`
- `#elixir`
- `#clojure`
- `#scala`

### Data & Scripting
- `#sql`
- `#bash` / `#shell`
- `#powershell`
- `#r`
- `#matlab`
- `#lua`
- `#perl`

### Data Formats
- `#json`
- `#yaml`

## ⌨️ Keyboard Shortcuts

| Key Combination | Action |
|----------------|---------|
| `#language` + Space | Activate code mode |
| `Enter` | Insert newline (in code mode) |
| `Cmd+Enter` (Mac) | Send message |
| `Ctrl+Enter` (Win/Linux) | Send message |
| Delete `#language` | Exit code mode |

## 💡 Tips & Tricks

### 1. Quick Code Snippets
Perfect for sharing quick code examples in chat:
```
#python print("Hello, World!")
```

### 2. Multi-line Code
Use Enter freely to format your code:
```
#javascript
function fibonacci(n) {
  if (n <= 1) return n;
  return fibonacci(n-1) + fibonacci(n-2);
}
```

### 3. Language Detection
The language badge shows you're in code mode:
- Look for the badge in the top-left
- It displays the active language

### 4. Syntax Highlighting
Colors update as you type:
- **Keywords** in purple
- **Strings** in green
- **Comments** in gray
- **Functions** in yellow
- And more!

## 🆚 Comparison: Triple Backticks vs #language

| Feature | Triple Backticks | #language Trigger |
|---------|------------------|-------------------|
| Activation | ` ```python ` | `#python ` |
| Live highlighting | ❌ After complete | ✅ As you type |
| Enter behavior | Normal | Inserts newline |
| Visual feedback | After closing ` ``` ` | Immediate |
| Best for | Complete code blocks | Quick snippets |

## 🎨 Visual Features

### Language Badge
- Appears in top-left when code mode is active
- Shows current language
- Icon + language name

### IDE-Style Background
- Subtle dark background (`#1E1E1E/30`)
- Rounded corners
- Border for definition

### Syntax Colors
- Based on VS Code Dark Plus theme
- Professional, easy-to-read colors
- Consistent with popular IDEs

## 🐛 Troubleshooting

### Code mode not activating?
- Make sure `#language` is at the **very start** of your input
- Add a space after the language name
- Check spelling (e.g., `#python` not `#pyton`)

### Enter key sending message instead of newline?
- Make sure you're in code mode (look for the language badge)
- The `#language` trigger must be present

### Syntax highlighting not working?
- Check if the language is supported (see list above)
- Try restarting the app
- Check console for errors

### Want to exit code mode?
- Delete the `#language` trigger
- Or send your message and start fresh

## 🚀 Pro Tips

1. **Start with the language** - Always begin with `#language`
2. **Use Cmd/Ctrl+Enter** - Get used to this for sending
3. **Format as you go** - Use Enter to keep code readable
4. **Mix with text** - Can't mix code mode with regular text (by design)
5. **Quick switch** - Delete trigger to switch back to normal mode

## 🎉 Enjoy Coding!

The live IDE input makes sharing code in chat feel natural and professional. No more awkward triple backticks - just type `#python` and start coding!

---

**Questions or Issues?**  
Check the console logs (they start with `💻 CODE MODE:`) for debugging info.

## 🆕 Update: Inline Code Mode!

You can now use `#language` **anywhere** in your message, not just at the start!

### Examples:

**Before (only worked at start):**
```
#python print("hello")
```

**Now (works inline too!):**
```
Here's the code: #python print("hello")
```

```
Let me show you this function #javascript
const greet = () => console.log("Hi!");
```

```
Check this out #typescript
interface User {
  name: string;
}
```

The text before `#language` is preserved and displayed normally, then everything after becomes highlighted code!

