# Goose Website Revamp — TODO Tracker

## Legend
- ⬜ Not started
- 🔄 In progress
- ✅ Completed

---

## Phase 1: Reusable Components

| # | Task | Status | Notes |
|---|------|--------|-------|
| 1.1 | Create `ScrollingMarquee` component | ✅ | `src/components/ScrollingMarquee/` — CSS keyframes + React |

## Phase 2: Navigation Overhaul

| # | Task | Status | Notes |
|---|------|--------|-------|
| 2.1 | Create `FullScreenMenu` component | ✅ | `src/components/FullScreenMenu/` — overlay with links + social |
| 2.2 | Swizzle and customize Navbar | ✅ | `src/theme/Navbar/index.tsx` — homepage vs docs routing |
| 2.3 | Update `docusaurus.config.ts` navbar items | ✅ | Kept as-is — OriginalNavbar used on non-homepage pages |
| 2.4 | Navbar styles | ✅ | `src/theme/Navbar/styles.module.css` — fixed bar, logo, menu button |

## Phase 3: Homepage Sections

| # | Task | Status | Notes |
|---|------|--------|-------|
| 3.1 | Create `HeroSection` component | ✅ | `src/components/HeroSection/` — wordmark + tagline + 2 CTAs |
| 3.2 | Create `ProductDemo` component | ✅ | `src/components/ProductDemo/` — demo image + marquee overlay |
| 3.3 | Create `FeaturesGrid` component | ✅ | `src/components/FeaturesGrid/` — 2×2 cards with visuals |
| 3.4 | Create `ValueProps` component | ✅ | `src/components/ValueProps/` — 3-col: Open Source, Multi-Model, Agentic AI |
| 3.5 | Create `PersonaPrompts` component | ✅ | `src/components/PersonaPrompts/` — 4 personas with scrolling marquees |

## Phase 4: Homepage Assembly

| # | Task | Status | Notes |
|---|------|--------|-------|
| 4.1 | Rewrite `src/pages/index.tsx` | ✅ | All sections wired together |
| 4.2 | Rewrite `src/pages/index.module.css` | ✅ | Minimal — sections self-contained |
| 4.3 | Add demo assets (screenshot/video) | ✅ | `static/img/goose-demo.svg` placeholder created |

## Phase 5: Polish & Testing

| # | Task | Status | Notes |
|---|------|--------|-------|
| 5.1 | Dark mode testing for all new components | ⬜ | All use Arcade CSS variables — needs visual verification |
| 5.2 | Responsive testing (mobile/tablet/desktop) | ⬜ | All components have @media breakpoints — needs visual verification |
| 5.3 | Build verification (`npm run build`) | ⬜ | Blocked by npm install (corporate firewall) |
| 5.4 | Verify docs/blog pages still work | ⬜ | Navbar wrapper routes OriginalNavbar on non-homepage |
| 5.5 | Replace placeholder demo image | ⬜ | Replace `goose-demo.svg` with real screenshot |

## Phase 6: Stretch Goals

| # | Task | Status | Notes |
|---|------|--------|-------|
| 6.1 | Create `/features` page | ⬜ | Expanded features content |
| 6.2 | Create `/why-goose` page | ⬜ | Testimonials + value props |
| 6.3 | Visual refresh for `/extensions` page | ⬜ | Match new card style |

---

## Files Created/Modified

### New Files (14)
- `src/components/ScrollingMarquee/ScrollingMarquee.tsx`
- `src/components/ScrollingMarquee/styles.module.css`
- `src/components/FullScreenMenu/FullScreenMenu.tsx`
- `src/components/FullScreenMenu/styles.module.css`
- `src/components/HeroSection/HeroSection.tsx`
- `src/components/HeroSection/styles.module.css`
- `src/components/ProductDemo/ProductDemo.tsx`
- `src/components/ProductDemo/styles.module.css`
- `src/components/FeaturesGrid/FeaturesGrid.tsx`
- `src/components/FeaturesGrid/styles.module.css`
- `src/components/ValueProps/ValueProps.tsx`
- `src/components/ValueProps/styles.module.css`
- `src/components/PersonaPrompts/PersonaPrompts.tsx`
- `src/components/PersonaPrompts/styles.module.css`
- `src/theme/Navbar/index.tsx`
- `src/theme/Navbar/styles.module.css`
- `static/img/goose-demo.svg`

### Modified Files (2)
- `src/pages/index.tsx` (rewritten)
- `src/pages/index.module.css` (simplified)

### Unchanged
- `docusaurus.config.ts` (navbar config kept for non-homepage pages)
- `src/css/custom.css` (no changes needed — Arcade variables already defined)
- All docs, blog, and other pages
