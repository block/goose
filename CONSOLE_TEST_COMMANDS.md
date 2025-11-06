# Console Test Commands

Copy and paste these into your browser console to test the collaborative editor!

## 🎯 Quick Tests

### 1. Get Your Editor

```javascript
// Get the first available editor
const docId = Object.keys(window.gooseEditors)[0];
const editor = window.gooseEditors[docId];
console.log('✅ Editor ready:', docId);
```

### 2. Test Insert Text

```javascript
editor.insertText('Hello from Goose! 👋\n\n');
```

### 3. Test Append Text

```javascript
editor.appendText('This text was added to the end!\n\n');
```

### 4. Test Formatting

```javascript
// Make the first line bold
editor.formatText(0, 20, 'bold');
```

### 5. Test Replace Text

```javascript
// Replace "Hello" with "Hi"
editor.replaceText(0, 5, 'Hi');
```

### 6. Get Content

```javascript
// See what's in the document
console.log('Plain text:', editor.getText());
console.log('HTML:', editor.getContent());
```

---

## 🎬 Full Demo

Paste this entire block to see a complete demo:

```javascript
(async function demo() {
  // Get editor
  const docId = Object.keys(window.gooseEditors)[0];
  const editor = window.gooseEditors[docId];
  
  console.log('🎬 Starting demo for document:', docId);
  
  // Clear document
  editor.clear();
  await new Promise(r => setTimeout(r, 500));
  
  // Add title
  console.log('📝 Adding title...');
  editor.insertText('Collaborative Editing Demo\n\n');
  editor.formatText(0, 27, 'heading1');
  await new Promise(r => setTimeout(r, 1000));
  
  // Add introduction
  console.log('📝 Adding introduction...');
  editor.appendText('This document was created by Goose using the collaborative editing API. ');
  editor.appendText('Watch as text appears in real-time!\n\n');
  await new Promise(r => setTimeout(r, 1000));
  
  // Add features section
  console.log('📝 Adding features...');
  editor.appendText('Key Features:\n');
  editor.appendText('Real-time editing\n');
  editor.appendText('Visual feedback\n');
  editor.appendText('Full undo support\n');
  editor.appendText('Multiple format options\n\n');
  
  // Format as list
  const text = editor.getText();
  const listStart = text.indexOf('Key Features:');
  const listEnd = text.indexOf('Multiple format options') + 23;
  editor.formatText(listStart, listEnd, 'bulletList');
  await new Promise(r => setTimeout(r, 1000));
  
  // Add code example
  console.log('📝 Adding code example...');
  editor.appendText('\nExample API call:\n');
  editor.appendText('editor.insertText("Hello!")');
  const codeStart = text.length + 20;
  const codeEnd = codeStart + 28;
  editor.formatText(codeStart, codeEnd, 'code');
  await new Promise(r => setTimeout(r, 1000));
  
  // Add conclusion
  console.log('📝 Adding conclusion...');
  editor.appendText('\n\nConclusion\n\n');
  editor.appendText('This demonstrates the power of collaborative AI editing. ');
  editor.appendText('Goose can edit documents in real-time while you watch!');
  
  const conclusionStart = editor.getText().indexOf('Conclusion');
  editor.formatText(conclusionStart, conclusionStart + 10, 'heading2');
  
  console.log('✅ Demo complete!');
  console.log('📄 Final document:', editor.getText());
})();
```

---

## 🎨 Typing Animation

Simulate Goose typing word by word:

```javascript
(async function typeAnimation() {
  const docId = Object.keys(window.gooseEditors)[0];
  const editor = window.gooseEditors[docId];
  
  editor.clear();
  
  const text = "The quick brown fox jumps over the lazy dog. This is a test of the collaborative editing system.";
  const words = text.split(' ');
  
  console.log('⌨️ Starting typing animation...');
  
  for (let i = 0; i < words.length; i++) {
    editor.appendText(words[i] + ' ');
    await new Promise(r => setTimeout(r, 200)); // 200ms between words
  }
  
  console.log('✅ Typing complete!');
})();
```

---

## 🔄 Format Testing

Test all available formats:

```javascript
(function testFormats() {
  const docId = Object.keys(window.gooseEditors)[0];
  const editor = window.gooseEditors[docId];
  
  editor.clear();
  
  // Add sample text for each format
  const formats = [
    { text: 'Bold Text\n', format: 'bold' },
    { text: 'Italic Text\n', format: 'italic' },
    { text: 'Heading 1\n', format: 'heading1' },
    { text: 'Heading 2\n', format: 'heading2' },
    { text: 'Heading 3\n', format: 'heading3' },
    { text: 'Bullet item 1\nBullet item 2\n', format: 'bulletList' },
    { text: 'Numbered item 1\nNumbered item 2\n', format: 'orderedList' },
    { text: 'inline code\n', format: 'code' },
    { text: 'Code block content\n', format: 'codeBlock' },
    { text: 'This is a quote\n', format: 'blockquote' },
  ];
  
  let position = 0;
  formats.forEach(({ text, format }) => {
    editor.appendText(text);
    const start = position;
    const end = position + text.length;
    editor.formatText(start, end, format);
    position = end;
    console.log(`✅ Applied ${format} to "${text.trim()}"`);
  });
  
  console.log('✅ All formats tested!');
})();
```

---

## 🧪 Advanced Tests

### Test 1: Replace Multiple Words

```javascript
const editor = window.gooseEditors[Object.keys(window.gooseEditors)[0]];

// Add some text
editor.clear();
editor.insertText('The cat sat on the mat.');

// Replace words
setTimeout(() => editor.replaceText(4, 7, 'dog'), 500);
setTimeout(() => editor.replaceText(16, 19, 'a'), 1000);
setTimeout(() => editor.replaceText(20, 23, 'log'), 1500);

// Result: "The dog sat on a log."
```

### Test 2: Progressive Formatting

```javascript
(async function progressiveFormat() {
  const editor = window.gooseEditors[Object.keys(window.gooseEditors)[0]];
  
  editor.clear();
  editor.insertText('This is important text that needs emphasis.');
  
  await new Promise(r => setTimeout(r, 1000));
  editor.formatText(8, 17, 'bold'); // "important"
  
  await new Promise(r => setTimeout(r, 1000));
  editor.formatText(30, 38, 'italic'); // "emphasis"
  
  console.log('✅ Progressive formatting complete!');
})();
```

### Test 3: Build a Document

```javascript
(async function buildDocument() {
  const editor = window.gooseEditors[Object.keys(window.gooseEditors)[0]];
  
  editor.clear();
  
  // Title
  editor.insertText('My Document\n\n');
  editor.formatText(0, 11, 'heading1');
  await new Promise(r => setTimeout(r, 500));
  
  // Section 1
  editor.appendText('Introduction\n\n');
  editor.formatText(13, 25, 'heading2');
  await new Promise(r => setTimeout(r, 500));
  
  editor.appendText('This is the introduction paragraph. ');
  editor.appendText('It provides context for the document.\n\n');
  await new Promise(r => setTimeout(r, 500));
  
  // Section 2
  editor.appendText('Key Points\n\n');
  const keyPointsStart = editor.getText().length - 12;
  editor.formatText(keyPointsStart, keyPointsStart + 10, 'heading2');
  await new Promise(r => setTimeout(r, 500));
  
  editor.appendText('First point\n');
  editor.appendText('Second point\n');
  editor.appendText('Third point\n\n');
  
  const listStart = editor.getText().indexOf('First point');
  const listEnd = editor.getText().indexOf('Third point') + 11;
  editor.formatText(listStart, listEnd, 'bulletList');
  
  console.log('✅ Document built!');
})();
```

---

## 🎯 One-Liner Tests

Quick tests you can run one at a time:

```javascript
// Test 1: Insert emoji
window.gooseEditors[Object.keys(window.gooseEditors)[0]].insertText('🚀 ');

// Test 2: Append timestamp
window.gooseEditors[Object.keys(window.gooseEditors)[0]].appendText(`\n\nEdited at: ${new Date().toLocaleTimeString()}`);

// Test 3: Get word count
window.gooseEditors[Object.keys(window.gooseEditors)[0]].getText().split(' ').length;

// Test 4: Clear everything
window.gooseEditors[Object.keys(window.gooseEditors)[0]].clear();

// Test 5: Get selection
window.gooseEditors[Object.keys(window.gooseEditors)[0]].getSelection();
```

---

## 🔍 Debugging Commands

```javascript
// Check if editor exists
console.log('Available editors:', Object.keys(window.gooseEditors));

// Get editor details
const docId = Object.keys(window.gooseEditors)[0];
const editor = window.gooseEditors[docId];
console.log('Editor methods:', Object.keys(editor));

// Check content
console.log('Current content:', editor.getText());
console.log('Content length:', editor.getText().length);

// Test each method
console.log('insertText:', typeof editor.insertText);
console.log('replaceText:', typeof editor.replaceText);
console.log('appendText:', typeof editor.appendText);
console.log('formatText:', typeof editor.formatText);
console.log('getContent:', typeof editor.getContent);
console.log('getText:', typeof editor.getText);
console.log('getSelection:', typeof editor.getSelection);
console.log('clear:', typeof editor.clear);
```

---

## 🎊 Fun Demos

### Demo 1: Countdown

```javascript
(async function countdown() {
  const editor = window.gooseEditors[Object.keys(window.gooseEditors)[0]];
  editor.clear();
  
  for (let i = 5; i >= 0; i--) {
    editor.clear();
    editor.insertText(`Countdown: ${i}`);
    editor.formatText(0, 20, 'heading1');
    await new Promise(r => setTimeout(r, 1000));
  }
  
  editor.clear();
  editor.insertText('🎉 DONE! 🎉');
  editor.formatText(0, 13, 'heading1');
})();
```

### Demo 2: Progress Bar

```javascript
(async function progressBar() {
  const editor = window.gooseEditors[Object.keys(window.gooseEditors)[0]];
  editor.clear();
  
  for (let i = 0; i <= 10; i++) {
    const bar = '█'.repeat(i) + '░'.repeat(10 - i);
    editor.clear();
    editor.insertText(`Progress: ${bar} ${i * 10}%`);
    await new Promise(r => setTimeout(r, 500));
  }
  
  editor.appendText('\n\n✅ Complete!');
})();
```

---

## 📋 Copy-Paste Ready

**Quick Start:**
```javascript
const editor = window.gooseEditors[Object.keys(window.gooseEditors)[0]];
editor.insertText('Hello from Goose! 👋');
```

**Full Demo:**
```javascript
// Paste the "Full Demo" block from above
```

**Typing Animation:**
```javascript
// Paste the "Typing Animation" block from above
```

---

**Tip**: Open DevTools (Cmd+Option+I), go to Console tab, and paste any of these commands!

**Status**: ✅ Ready to test  
**Branch**: `spence/doceditor`
