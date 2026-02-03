---
name: design-reference
description: Transform a design reference HTML file into a Vibes app. Use when user provides a design.html, mockup, or static prototype to match exactly.
---

**Display this ASCII art immediately when starting:**

```
░▒▓███████▓▒░░▒▓████████▓▒░░▒▓███████▓▒░▒▓█▓▒░░▒▓██████▓▒░░▒▓███████▓▒░
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░      ░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░      ░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░
░▒▓█▓▒░░▒▓█▓▒░▒▓██████▓▒░  ░▒▓██████▓▒░░▒▓█▓▒░▒▓█▓▒▒▓███▓▒░▒▓█▓▒░░▒▓█▓▒░
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░             ░▒▓█▓▒░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░             ░▒▓█▓▒░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░
░▒▓███████▓▒░░▒▓████████▓▒░▒▓███████▓▒░░▒▓█▓▒░░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░
```

# Design Reference Transformer

Transform a complete design reference HTML file into a working Vibes app with Fireproof data persistence.

---

## Core Principle

> **Preserve and adapt, don't interpret and recreate.**

The design reference is **source code to transform**, not inspiration to interpret. When given a complete HTML file with styles, your job is to make **minimal surgical changes** to connect it to React/Fireproof—not to recreate it from your understanding of its aesthetic.

---

## When to Use This Skill

Use this skill when:
- User provides a `design.html`, mockup, or static prototype file
- User says "match this design exactly" or "use this as a reference"
- User wants their existing HTML converted to a Vibes app
- A previous implementation didn't match the design reference

---

## The Transformation is Mechanical

The conversion from design HTML to React/Fireproof is **deterministic, not creative**:

| Transformation | Rule | Example |
|----------------|------|---------|
| Attributes | `class` → `className` | `class="btn"` → `className="btn"` |
| Attributes | `for` → `htmlFor` | `for="email"` → `htmlFor="email"` |
| Attributes | kebab-case → camelCase | `stroke-width` → `strokeWidth` |
| Self-closing | Add explicit close | `<input>` → `<input />` |
| Comments | HTML → JSX | `<!-- x -->` → `{/* x */}` |
| Inline styles | String → Object | `style="color: red"` → `style={{ color: 'red' }}` |
| Event handlers | Lowercase → camelCase | `onclick` → `onClick` |

**CSS requires NO changes.** Copy the entire `<style>` block verbatim.

---

## Workflow

### Step 1: Read the Design Reference

```bash
# Read the design file completely
Read design.html
```

Note the structure:
- `<style>` block (copy verbatim)
- HTML structure (preserve exactly)
- Any vanilla JavaScript (will be replaced with React)

### Step 2: Identify Dynamic Content

Ask: "What content comes from the database?"

Typical dynamic elements:
- List items that repeat (`.map()`)
- Text that users enter (controlled inputs)
- Counts, totals, timestamps
- User-specific content

**Everything else stays static.**

### Step 3: Create the React Component

```jsx
import React from "react";
import { useFireproofClerk } from "use-fireproof";

export default function App() {
  const { database, useLiveQuery, useDocument } = useFireproofClerk("app-db");

  // Hooks for dynamic data
  const { doc, merge, submit } = useDocument({ /* initial shape */ });
  const { docs } = useLiveQuery("type", { key: "item" });

  return (
    <>
      {/* CSS copied VERBATIM from design.html */}
      <style>{`
        /* Paste entire <style> block here unchanged */
      `}</style>

      {/* HTML structure preserved, only syntax converted */}
      {/* Dynamic content replaced with {expressions} */}
    </>
  );
}
```

### Step 4: Handle Dark Mode Override (If Needed)

The Vibes template has dark mode support. If your design is light-only, add this CSS override:

```css
/* Force light theme regardless of system preference */
html, body, #container, #container > div {
    background-color: var(--your-bg-color) !important;
}

[style*="position: fixed"][style*="background"] {
    background-color: var(--your-bg-color) !important;
}
```

### Step 5: Assemble and Test

```bash
node "/path/to/vibes-skill/scripts/assemble.js" app.jsx index.html
```

Open in browser and **visually diff against the design reference**. They should be pixel-identical except for dynamic content.

---

## Anti-Patterns (DO NOT DO THESE)

| Anti-Pattern | Why It's Wrong | Correct Approach |
|--------------|----------------|------------------|
| Translate colors to OKLCH | Changes the design | Use exact hex values from reference |
| Restructure HTML "for React" | Breaks layout | Preserve structure, only change syntax |
| "Improve" the CSS | Not your job | Copy verbatim |
| Add your own classes | Introduces drift | Use exact classes from reference |
| Interpret the "vibe" | Creates divergence | Be literal, not interpretive |
| Skip vanilla JS analysis | Miss functionality | Understand what it does, then React-ify |

---

## Transformation Checklist

Before writing code, verify:

- [ ] Read the entire design.html file
- [ ] Identified all `<style>` blocks (will copy verbatim)
- [ ] Identified dynamic content (lists, inputs, user data)
- [ ] Identified vanilla JS functionality (will convert to React)
- [ ] Noted any custom fonts (add to imports if needed)
- [ ] Checked for dark/light theme assumptions

During transformation:

- [ ] CSS pasted unchanged (no "improvements")
- [ ] HTML structure preserved exactly
- [ ] Only syntax converted (class→className, etc.)
- [ ] Dynamic content uses `{expressions}` and `.map()`
- [ ] Vanilla JS replaced with React hooks and handlers
- [ ] Dark mode override added if design is light-only

After assembly:

- [ ] Visual comparison with design reference
- [ ] All interactive elements work
- [ ] Data persists on refresh
- [ ] No console errors

---

## Example: Static List → Dynamic List

**Design HTML:**
```html
<ul class="item-list">
  <li class="item">First item</li>
  <li class="item">Second item</li>
</ul>
```

**React with Fireproof:**
```jsx
const { docs } = useLiveQuery("type", { key: "item" });

<ul className="item-list">
  {docs.map(item => (
    <li key={item._id} className="item">{item.text}</li>
  ))}
</ul>
```

Note: Only the content changed. The classes, structure, and styling are identical.

---

## Example: Static Form → Controlled Form

**Design HTML:**
```html
<form>
  <input type="text" class="input" placeholder="Enter text...">
  <button class="btn">Submit</button>
</form>
```

**React with Fireproof:**
```jsx
const { doc, merge, submit } = useDocument({ text: "", type: "item" });

<form onSubmit={submit}>
  <input
    type="text"
    className="input"
    placeholder="Enter text..."
    value={doc.text}
    onChange={(e) => merge({ text: e.target.value })}
  />
  <button type="submit" className="btn">Submit</button>
</form>
```

Note: Same structure, same classes, same placeholder. Only added React bindings.

---

## Integration with Vibes Assembly

This skill produces an `app.jsx` that works with the standard Vibes assembly:

```bash
# In the working directory
node "/path/to/vibes-skill/scripts/assemble.js" app.jsx index.html
```

The assembly script:
- Inserts your JSX into the Vibes template
- Handles Clerk authentication wrapper
- Sets up import maps for React and Fireproof
- Configures Connect if `.env` is present

---

## What's Next?

After transforming a design reference, present these options using AskUserQuestion:

```
Question: "Design reference transformed! What's next?"
Header: "Next"
Options:
- Label: "Test locally"
  Description: "Open index.html in browser to verify it matches the design exactly"

- Label: "Deploy to exe.dev (/exe)"
  Description: "Push the app live at yourapp.exe.xyz"

- Label: "Make adjustments"
  Description: "Fine-tune specific elements while preserving the design"

- Label: "I'm done"
  Description: "Wrap up - files are saved locally"
```
