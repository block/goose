# Visual Testing Guide - Goose Navigation System

## Quick Start

### 1. Start the Development Server
```bash
cd /Users/spencermartin/goose-nav-lite/ui/desktop
npm run start-gui
```

### 2. Access the Application
The app will open automatically in Electron. If you see the splash screen, you may need to configure a provider first.

### 3. Enable Overlay Mode
1. Look for the toggle button in the top-right corner (or position based on your settings)
2. Click "Show launcher" to open the overlay
3. The navigation should appear as a centered grid overlay

## Testing Checklist

### ✅ Overlay Mode - Grid Layout

#### Visual Inspection
- [ ] **Grid appears centered** on screen (both horizontally and vertically)
- [ ] **All 12 items visible** without scrolling:
  - 7 navigation items (Home, Chat, History, Recipes, Scheduler, Extensions, Settings)
  - 2 placeholder tiles (empty, no icon/label)
  - 3 widgets (Clock, Activity Heatmap, Token Counter)
- [ ] **6 columns × 2 rows** layout on large displays (1920x1080+)
- [ ] **12px gap** between tiles (visible white space)
- [ ] **Tiles are square** (aspect-ratio 1:1)

#### Tile Sizing
- [ ] **Tile padding**: ~20px (px-5 py-5) - tiles should feel compact but not cramped
- [ ] **Icon size**: 24px (w-6 h-6) - icons should be clear and recognizable
- [ ] **Text size**: 20px (text-xl) - labels should be easily readable
- [ ] **Tag font**: 10px - small but legible stats/badges

#### Background & Blur
- [ ] **Backdrop blur** visible behind overlay
- [ ] **Semi-transparent background** (bg-black/20)
- [ ] **Tile backgrounds** have backdrop-blur-md effect
- [ ] **Active tile** has accent color background

### ✅ Responsive Behavior

Test at different viewport widths:

#### Mobile (< 640px)
- [ ] **2 columns** grid layout
- [ ] All tiles remain clickable
- [ ] Text remains readable
- [ ] Scrolling may be required (expected)

#### Small (640px - 768px)
- [ ] **3 columns** grid layout
- [ ] Better space utilization
- [ ] May require slight scrolling

#### Medium (768px - 1024px)
- [ ] **4 columns** grid layout
- [ ] Most items visible without scrolling

#### Large (1024px - 1280px)
- [ ] **5 columns** grid layout
- [ ] All items likely visible

#### XL/2XL (1280px+)
- [ ] **6 columns** grid layout
- [ ] All 12 items fit perfectly in 6×2 grid
- [ ] No scrolling required

### ✅ Interactive Elements

#### Hover Effects
- [ ] Tiles scale up slightly on hover (scale: 1.02)
- [ ] Background color changes on hover
- [ ] Drag handle (GripVertical icon) appears on hover
- [ ] Cursor changes to pointer

#### Click Actions
- [ ] Clicking tile navigates to correct page
- [ ] Overlay closes after navigation
- [ ] Active state persists on current page

#### Drag & Drop
- [ ] Can grab tile by drag handle
- [ ] Tile becomes semi-transparent while dragging (opacity: 0.5)
- [ ] Drop zones highlight with blue ring
- [ ] Tiles reorder correctly on drop
- [ ] Order persists after refresh

### ✅ Widgets

#### AnalogClock Widget
- [ ] Clock renders with no circular border
- [ ] **12 hour ticks** visible on lg+ screens (≥1024px)
- [ ] **Hour ticks hidden** on smaller screens (<1024px)
- [ ] **Hour hand** (thick, dark) moves correctly
- [ ] **Minute hand** (thinner) moves correctly
- [ ] **Second hand** (red, ultra-thin) moves smoothly
- [ ] **Center dot** visible
- [ ] Clock updates in real-time

#### Activity Heatmap Widget
- [ ] Displays 35 days of session data
- [ ] Grid layout: 7 columns × 5 rows
- [ ] Color intensity varies by activity:
  - No activity: bg-background-muted (gray)
  - Low: bg-green-200 (light green)
  - Medium: bg-green-300
  - High: bg-green-400
  - Very high: bg-green-500 (bright green)
- [ ] Hover shows date and session count
- [ ] "Last 35 days" label visible

#### Token Counter Widget
- [ ] Displays total tokens in millions (e.g., "2.45M")
- [ ] Large, readable font (text-3xl)
- [ ] "Total tokens" label visible
- [ ] Updates when new sessions created

### ✅ Overlay Controls

#### Opening Overlay
- [ ] Click "Show launcher" button
- [ ] Overlay fades in smoothly (opacity: 0 → 1)
- [ ] Grid scales in (scale: 0.95 → 1)
- [ ] Animation duration: 300ms

#### Closing Overlay
- [ ] Click backdrop (outside grid) to close
- [ ] Press ESC key to close
- [ ] Click "Hide launcher" button to close
- [ ] Overlay fades out smoothly
- [ ] Grid scales out (scale: 1 → 0.95)

### ✅ Live Stats & Animations

#### Stat Updates
- [ ] "Today's Chats" count updates when new chat created
- [ ] "Total Sessions" count accurate
- [ ] "Recipes" count matches saved recipes
- [ ] "Extensions" shows "X of Y enabled"
- [ ] Current time updates every second

#### Pulse Animations
- [ ] Blue dot appears when stat updates
- [ ] Dot pulses for 2 seconds
- [ ] Multiple stats can pulse simultaneously

### ✅ Placeholder Tiles

- [ ] **placeholder-1** tile is completely empty (no icon, no label)
- [ ] **placeholder-2** tile is completely empty (no icon, no label)
- [ ] Both tiles are clickable (navigate to '#')
- [ ] Both tiles can be dragged and reordered
- [ ] Both tiles can be hidden via settings

### ✅ Settings Integration

#### Navigation Mode
1. Navigate to Settings → App → Navigation Mode
2. [ ] Can switch between Push and Overlay modes
3. [ ] Mode persists after refresh
4. [ ] Overlay mode shows full-screen launcher
5. [ ] Push mode shows traditional navigation

#### Navigation Position
1. Navigate to Settings → App → Navigation Position
2. [ ] Can select Top, Bottom, Left, Right
3. [ ] Position persists after refresh
4. [ ] Toggle button moves to appropriate position

#### Navigation Style
1. Navigate to Settings → App → Navigation Style
2. [ ] Can switch between Expanded and Condensed
3. [ ] Style persists after refresh
4. [ ] Expanded shows tile grid
5. [ ] Condensed shows row layout

#### Navigation Items
1. Navigate to Settings → App → Navigation Items
2. [ ] Can drag items to reorder
3. [ ] Can toggle eye icon to hide/show items
4. [ ] Can click "Reset to Defaults" to restore
5. [ ] Changes reflect immediately in navigation
6. [ ] Order persists after refresh

### ✅ Performance

#### Animation Smoothness
- [ ] Overlay open/close is smooth (60fps)
- [ ] Tile hover effects are smooth
- [ ] Drag & drop is responsive
- [ ] No jank or stuttering

#### Loading Time
- [ ] Navigation data loads quickly (<500ms)
- [ ] Stats appear without delay
- [ ] Widgets render immediately

#### Memory Usage
- [ ] Open DevTools → Performance → Memory
- [ ] No memory leaks after multiple open/close cycles
- [ ] Memory usage stable

## Common Issues & Solutions

### Issue: Tiles are spread out too much
**Solution**: Check that overlay mode optimization is applied:
- Grid should have `xl:grid-cols-6` (not `xl:grid-cols-4`)
- Tiles should have `px-5 py-5` padding (not `px-8 py-8`)
- Icons should be `w-6 h-6` (not `w-8 h-8`)

### Issue: Scrolling required on 1920x1080
**Solution**: 
- Verify grid has `max-w-7xl mx-auto` for centering
- Check that gap is `gap-3` (not larger)
- Ensure container has `flex items-center justify-center`

### Issue: Placeholder tiles not visible
**Solution**: 
- Clear localStorage: `localStorage.removeItem('navigation_preferences')`
- Refresh the page
- Placeholders should appear at end of grid

### Issue: Clock ticks not visible
**Solution**: 
- Check viewport width is ≥1024px (lg breakpoint)
- Ticks are hidden on smaller screens by design (responsive behavior)

### Issue: Drag & drop not working
**Solution**: 
- Ensure you're grabbing the GripVertical icon (appears on hover)
- Check that tiles have `draggable` attribute
- Verify drop zones highlight with blue ring

### Issue: Stats not updating
**Solution**: 
- Check API calls in Network tab
- Verify `fetchNavigationData()` is called when overlay opens
- Check console for errors

## Screenshot Locations

Take screenshots at these key moments:

1. **Overlay Closed**: Main app view with toggle button visible
2. **Overlay Open**: Full grid with all 12 items visible
3. **Hover State**: Tile with hover effects and drag handle visible
4. **Dragging**: Tile being dragged with semi-transparent appearance
5. **Mobile View**: 2-column layout on small screen
6. **Tablet View**: 4-column layout on medium screen
7. **Desktop View**: 6-column layout on large screen
8. **Settings**: Navigation customization settings page

Save screenshots to: `/Users/spencermartin/goose-nav-lite/screenshots/`

## Testing Report Template

```markdown
# Overlay Mode Testing Report

**Date**: [Date]
**Tester**: [Name]
**Display**: [Resolution, e.g., 1920x1080]
**Browser/Electron Version**: [Version]

## Summary
- [ ] All 12 items visible without scrolling
- [ ] Grid layout correct (6 columns × 2 rows)
- [ ] Tiles compact and readable
- [ ] Animations smooth
- [ ] Drag & drop functional
- [ ] Widgets display correctly

## Issues Found
1. [Issue description]
   - **Severity**: [Low/Medium/High]
   - **Steps to reproduce**: [Steps]
   - **Expected**: [Expected behavior]
   - **Actual**: [Actual behavior]

## Screenshots
- [Attach screenshots]

## Recommendations
- [Any suggestions for improvements]

## Approval
- [ ] Ready for production
- [ ] Needs minor fixes
- [ ] Needs major fixes
```

## Next Steps After Testing

1. **Document Issues**: Create GitHub issues for any bugs found
2. **Performance Optimization**: Profile and optimize if needed
3. **User Feedback**: Share with team for feedback
4. **Accessibility Audit**: Test with screen readers and keyboard navigation
5. **Cross-Platform Testing**: Test on Windows and Linux
6. **Documentation**: Update user guide with screenshots
7. **Release Notes**: Document changes for release

## Contact

For questions or issues during testing:
- **Project**: goose-nav-lite
- **Branch**: spence/nav-lite
- **Commit**: bd787dbe6
- **Location**: /Users/spencermartin/goose-nav-lite
