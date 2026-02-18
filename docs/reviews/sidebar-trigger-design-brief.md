# Sidebar Trigger — UX/UI Design Brief

**Date:** 2025-02-18
**Author:** Engineering (via design-system audit)
**Status:** 🟡 Awaiting UX/UI team decision
**Priority:** P2 — Usability improvement (not blocking)

---

## Problem Statement

The current sidebar toggle button sits in an **absolute-positioned header bar** (`absolute top-3 z-100`) that floats above the main content area. On macOS, it has `pl-21` left padding to clear the traffic light buttons, creating a ~84px dead zone at the top-left corner.

### Current Layout

```
┌──────────────────────────────────────────────┐
│  ● ● ●   [☰] [⊞]              (titlebar)    │  ← Drag region
├──────────────────────────────────────────────┤
│ ┌──────────┐ ┌──────────────────────────────┐│
│ │ pt-12    │ │                              ││
│ │          │ │                              ││
│ │ Sidebar  │ │     Main Content             ││
│ │ Content  │ │     (SidebarInset)           ││
│ │          │ │                              ││
│ │          │ │                              ││
│ └──────────┘ └──────────────────────────────┘│
└──────────────────────────────────────────────┘
```

**Issues identified:**
1. **Wasted vertical space** — The floating trigger + new-window button consume ~48px of vertical space above the sidebar content (`pt-12` = 48px padding-top to avoid overlap)
2. **Disconnected affordance** — The trigger visually belongs to the header/titlebar, not the sidebar itself
3. **Mobile inconsistency** — On mobile, buttons hide when the sidebar sheet opens (`shouldHideButtons = isMobile && openMobile`)
4. **macOS-specific padding** — `pl-21` (84px) is needed only to clear traffic lights, wasted on non-macOS

---

## Current Implementation

```
File: src/components/Layout/AppLayout.tsx

<div className="absolute top-3 z-100 flex items-center">
  <SidebarTrigger />      ← Burger menu icon
  <Button>                ← New window button
    <AppWindowMac />
  </Button>
</div>
<Sidebar variant="inset" collapsible="offcanvas">
  <AppSidebar />          ← pt-12 padding to avoid trigger overlap
</Sidebar>
```

```
File: src/components/ui/sidebar.tsx

SidebarTrigger → ghost Button → onClick: toggleSidebar()
  Currently renders: <Menu className="h-4 w-4" />
```

---

## Proposed Options

### Option A: Integrated Sidebar Header ⭐ Recommended

Move the trigger and new-window button **inside** the sidebar's own header area, eliminating the floating overlay entirely.

```
┌──────────────────────────────────────────────┐
│  ● ● ●                        (titlebar)     │
├──────────┬───────────────────────────────────┤
│ [☰][⊞]  │                                   │
│──────────│                                   │
│ 🏠 Home  │     Main Content                  │
│ ⚙ Settings│                                  │
│ 📦 Extensions                                │
│          │                                   │
│──────────│                                   │
│ Projects │                                   │
└──────────┴───────────────────────────────────┘
```

**When sidebar is closed:**
```
┌──────────────────────────────────────────────┐
│  ● ● ●   [☰]                  (titlebar)     │
├──────────────────────────────────────────────┤
│                                              │
│           Main Content (full width)          │
│                                              │
└──────────────────────────────────────────────┘
```

| Aspect | Detail |
|---|---|
| Trigger location | Inside `<SidebarHeader>` — first element in sidebar |
| Closed state | Small floating trigger in titlebar (same as current but no new-window button) |
| Gained space | ~48px vertical (remove `pt-12` from `SidebarContent`) |
| Complexity | Low — move existing components |
| Accessibility | ✅ Clear toggle affordance in both states |

### Option B: Icon Rail Collapse (VS Code pattern)

Sidebar never fully hides — it collapses to a narrow icon strip (~48px) showing only icons for each navigation section.

```
Expanded:                    Collapsed:
┌──────────┬────────────┐   ┌────┬────────────────┐
│ 🏠 Home  │            │   │ 🏠 │                │
│ ⚙ Settings│   Content │   │ ⚙  │    Content     │
│ 📦 Exts  │            │   │ 📦 │                │
│ 📊 Monitor│           │   │ 📊 │                │
│──────────│            │   │────│                │
│ Projects │            │   │ 📂 │                │
└──────────┴────────────┘   └────┴────────────────┘
```

| Aspect | Detail |
|---|---|
| Trigger | Click any icon to expand; hover or dedicated toggle |
| Closed state | 48px icon rail always visible |
| Gained space | ~200px horizontal when collapsed (vs 280px full) |
| Complexity | Medium — needs icon-only variants for all menu items |
| Accessibility | ✅ All sections always reachable |
| Infrastructure | `collapsible="icon"` already supported by sidebar component |

**Note:** The `<Sidebar>` component already supports `collapsible="icon"` mode. The main work is adding tooltip labels to icon-only items.

### Option C: Edge-Peek Hover Zone

No visible trigger. Hovering the left edge (0-8px) for 200ms reveals the sidebar with a slide animation.

```
Normal:                      Hover left edge:
┌──────────────────────┐    ┌──────────┬───────────┐
│                      │    │          │           │
│   Full-width Content │    │ Sidebar  │  Content  │
│                      │    │ (overlay)│           │
└──────────────────────┘    └──────────┴───────────┘
```

| Aspect | Detail |
|---|---|
| Trigger | Invisible 8px hover zone on left edge |
| Closed state | No chrome at all — maximum content space |
| Gained space | ~48px vertical + ~280px horizontal |
| Complexity | Low — CSS hover + transition |
| Accessibility | ⚠️ Poor discoverability; needs keyboard shortcut (`Ctrl+B`) |
| Risk | Users may not discover the sidebar exists |

### Option D: Minimal Floating Dot

Replace the full burger menu with a minimal floating indicator (small dot or thin line) in the top-left corner.

```
┌──────────────────────────────────────────────┐
│  ● ● ●  •                     (titlebar)     │
├──────────────────────────────────────────────┤
│                                              │
│           Full-width Content                 │
│                                              │
└──────────────────────────────────────────────┘
        ↑ tiny dot expands to sidebar on click
```

| Aspect | Detail |
|---|---|
| Trigger | 8x8px dot or 2x24px line |
| Closed state | Nearly invisible — maximum content space |
| Gained space | ~44px vertical |
| Complexity | Low |
| Accessibility | ⚠️ Small target (WCAG requires ≥24x24px touch targets) |

### Option E: Keyboard-Only + App Menu

Remove the visible trigger entirely. Sidebar is toggled via:
- `Ctrl+B` / `Cmd+B` keyboard shortcut
- Application menu: `View → Toggle Sidebar`

| Aspect | Detail |
|---|---|
| Gained space | Maximum — no chrome at all |
| Complexity | Low |
| Accessibility | ❌ Terrible for mouse-first users; violates discoverability |
| Recommended only | As a complement to Options A–D, never standalone |

---

## Recommendation

**Option A (Integrated Sidebar Header)** is the recommended approach:

1. **Lowest risk** — familiar pattern (Slack, Discord, Notion)
2. **Recovers 48px** of vertical space from `pt-12`
3. **Simple implementation** — move existing components, remove floating div
4. **No discoverability issues** — trigger is visible in closed state
5. **Pairs well with Option B** as a future enhancement (A now, B later)

### Implementation Estimate

| Task | Effort |
|---|---|
| Move trigger into `<SidebarHeader>` | 30min |
| Handle closed-state trigger position | 30min |
| Remove `pt-12` padding from `SidebarContent` | 5min |
| Adjust macOS traffic light clearance | 15min |
| Test both open/closed states | 20min |
| **Total** | **~1.5hr** |

---

## Technical Context

### Files to Modify

| File | Change |
|---|---|
| `src/components/Layout/AppLayout.tsx` | Remove floating trigger div; move new-window button |
| `src/components/GooseSidebar/AppSidebar.tsx` | Add `<SidebarHeader>` with trigger + new-window button |
| `src/components/ui/sidebar.tsx` | No changes needed (trigger component is reusable) |

### Existing Infrastructure

- `<Sidebar>` supports `collapsible="offcanvas"` (current), `"icon"`, and `"none"`
- `SidebarTrigger` component already exists and is reusable
- `useSidebar()` hook provides `state`, `open`, `toggleSidebar`, `isMobile`
- `pt-12` padding in `SidebarContent` was added solely to clear the floating trigger

### Constraints

- macOS: Traffic light buttons occupy top-left (~70px)
- Windows/Linux: No traffic lights, but frameless titlebar needs drag region
- Mobile: Sidebar uses sheet/overlay pattern (different layout)
- The titlebar drag region must remain functional

---

## Decision Matrix

| Criteria | Weight | A: Header | B: Rail | C: Peek | D: Dot | E: Keyboard |
|---|---|---|---|---|---|---|
| Space efficiency | 25% | ★★★★☆ | ★★★☆☆ | ★★★★★ | ★★★★★ | ★★★★★ |
| Discoverability | 25% | ★★★★★ | ★★★★★ | ★★☆☆☆ | ★★★☆☆ | ★☆☆☆☆ |
| Implementation cost | 20% | ★★★★★ | ★★★☆☆ | ★★★★☆ | ★★★★☆ | ★★★★★ |
| Familiarity | 15% | ★★★★★ | ★★★★★ | ★★★☆☆ | ★★☆☆☆ | ★★★☆☆ |
| Accessibility | 15% | ★★★★★ | ★★★★★ | ★★★☆☆ | ★★☆☆☆ | ★★☆☆☆ |
| **Weighted Score** | | **4.5** | **4.0** | **3.4** | **3.2** | **2.9** |

---

## Next Steps

1. **UX/UI team reviews** this brief and selects an option
2. **Create Figma mockups** for the selected option (both themes, both platforms)
3. **Engineering implements** (~1.5hr for Option A, ~4hr for Option B)
4. **QA validates** on macOS + Windows + Linux + mobile breakpoint

**Please tag @ux-team for review.**
