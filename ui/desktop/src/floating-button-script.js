// Handle window movement and docking detection
let isDragging = false;
let startX, startY;

document.addEventListener('mousedown', (e) => {
  isDragging = true;
  startX = e.screenX - window.screenX;
  startY = e.screenY - window.screenY;
});

document.addEventListener('mouseup', () => {
  if (isDragging) {
    isDragging = false;
    // Check for docking with parent window
    if (window.electronFloating) {
      window.electronFloating.checkDocking();
    }
  }
});

// Handle click to focus parent window
document.addEventListener('click', (e) => {
  if (!isDragging && window.electronFloating) {
    window.electronFloating.focusParent();
  }
});

console.log('Floating button script loaded');