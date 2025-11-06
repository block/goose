# Testing the Document Editor

## Quick Start

### 1. Start the Application

```bash
# From the goose root directory
source bin/activate-hermit
cd ui/desktop
npm run start-gui
```

### 2. Open a Document

1. Once the app loads, hover your mouse over the **right edge** of the window
2. A **plus button** will appear
3. Click the plus button
4. Select **"New Document"** from the menu
5. The document editor will open in a sidecar panel

### 3. Test Features

#### Basic Text Editing
- Type some text
- Select text and try formatting:
  - **Bold** (Cmd+B)
  - *Italic* (Cmd+I)
  - <u>Underline</u> (Cmd+U)

#### Headings
- Click the H1, H2, or H3 buttons in the toolbar
- Type a heading

#### Lists
- Click the bullet list icon
- Type list items, press Enter for new items
- Try the numbered list
- Try the task list (with checkboxes)

#### Links
- Select some text
- Click the link icon
- Enter a URL (e.g., https://example.com)
- Press Enter or click "Add Link"

#### Images
- Click the image icon
- Enter an image URL (e.g., https://picsum.photos/400/300)
- Press Enter or click "Add Image"

#### Code
- Click the code icon for inline code
- Click it again (or use the dropdown) for a code block

#### Alignment
- Try the alignment buttons (left, center, right, justify)

#### Other Features
- Try the blockquote button
- Try the horizontal rule button
- Try the highlight button
- Use Undo (Cmd+Z) and Redo (Cmd+Shift+Z)

### 4. Save Your Work

- Click the **"Save"** button in the header
- Or wait 30 seconds for auto-save
- Check the console to see the saved content

### 5. Multiple Documents

- Hover over the right edge again
- Click the plus button
- Select "New Document" again
- You now have two documents side-by-side!
- Drag the divider between them to resize

### 6. Close Documents

- Click the **red X button** in the top-right of any document to close it
- Click the **small X button** in the top-left of the bento box to close all documents

## What to Test

### ✅ Functionality
- [ ] All toolbar buttons work
- [ ] Keyboard shortcuts work (Cmd+B, Cmd+I, etc.)
- [ ] Links can be added and removed
- [ ] Images display correctly
- [ ] Lists work (bullet, numbered, task)
- [ ] Task list checkboxes are clickable
- [ ] Undo/Redo works
- [ ] Auto-save triggers every 30 seconds
- [ ] Manual save button works
- [ ] Last saved time updates

### ✅ UI/UX
- [ ] Toolbar is visible and accessible
- [ ] Active buttons are highlighted
- [ ] Disabled buttons (undo/redo) look disabled
- [ ] Link/image input appears below toolbar
- [ ] Placeholder text shows when empty
- [ ] Editor is scrollable for long documents
- [ ] Styling looks good (headings, lists, etc.)

### ✅ Multiple Documents
- [ ] Can open multiple documents
- [ ] Each document has its own content
- [ ] Can resize documents by dragging
- [ ] Can close individual documents
- [ ] Can close all documents at once

### ✅ Integration
- [ ] Plus button appears on hover
- [ ] Document option appears in menu
- [ ] Document opens in bento box
- [ ] Works alongside other sidecar types (localhost, file viewer)

## Known Issues

1. **No Persistence**: Documents are not saved to disk yet. Content is lost on refresh.
2. **Console Logging**: Save operations log to console instead of persisting.
3. **No Document List**: Can't browse or reopen previous documents.

## Troubleshooting

### Editor doesn't appear
- Make sure you're on the chat page (main page)
- Try refreshing the app
- Check the console for errors

### Toolbar buttons don't work
- Make sure the editor has focus (click in the editor area)
- Try refreshing the app

### Can't type in the editor
- Click in the editor area to focus it
- Make sure read-only mode is not enabled

### Auto-save not working
- Check the console for save logs
- Wait the full 30 seconds
- Try manual save instead

## Reporting Issues

If you find bugs or have suggestions:

1. Note the exact steps to reproduce
2. Check the browser console for errors
3. Take a screenshot if relevant
4. Document the issue in the branch PR

## Next Steps

After testing, consider:

1. **Persistence**: Add localStorage or backend save
2. **Document Management**: Add a document list/browser
3. **Export**: Add export to Markdown/PDF
4. **Templates**: Add document templates
5. **Collaboration**: Add real-time editing

---

**Branch**: `spence/doceditor`  
**Status**: Ready for testing  
**Date**: November 5, 2025
