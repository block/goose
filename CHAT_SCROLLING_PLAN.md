# Chat Scrolling Improvements

## Problem Statement
Fix disruptive auto-scroll behavior when users are reading previous messages while Goose is actively working.

## Proposed Solution
- Detect active user scrolling/reading
- Prevent auto-scroll during active reading
- Implement idle detection for graceful return to bottom
- No UI button approach - intelligent positioning only

## Implementation Plan
- [ ] Analyze current chat scrolling implementation
- [ ] Add scroll event detection and user activity tracking
- [ ] Implement intelligent auto-scroll logic
- [ ] Test with various chat scenarios
- [ ] Ensure smooth UX during active Goose sessions

