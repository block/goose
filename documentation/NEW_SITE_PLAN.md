# Goose Website Revamp Plan

## Overview

Transform the goose documentation site from a standard Docusaurus docs site into a product-forward marketing + docs hybrid, based on the new design in `newsite.mov`.

The site remains on **Docusaurus 3.9** with React 19, Tailwind CSS 3, and Framer Motion. No framework migration needed. Docs and blog sections stay largely untouched — the changes focus on the homepage, navigation, and new marketing-oriented components.

---

## Phase 1: Navigation Overhaul

### Current State
- Horizontal navbar with 8+ items (Quickstart, Docs, Tutorials, MCPs, Blog, Resources dropdown, Discord, GitHub)
- Standard Docusaurus navbar component

### New Design
- Minimal top bar: goose logo (left) + `Menu +` button (right)
- Clicking `Menu +` opens a **full-screen overlay** with large stacked links:
  - `features`
  - `extensions`
  - `docs`
  - `github`
  - `why goose`
- Social links at bottom of overlay: Twitter, GitHub, Discord
- `Close X` button in top-right of overlay

### Implementation

1. **Swizzle Navbar** — `npx docusaurus swizzle @docusaurus/theme-classic Navbar --wrap`
2. **Create `src/components/FullScreenMenu/`**:
   - `FullScreenMenu.tsx` — overlay component with menu items and social links
   - `styles.module.css` — full-screen overlay styles, transitions
3. **Create `src/theme/Navbar/index.tsx`** — custom navbar wrapper:
   - Renders logo on left
   - Renders `Menu +` / `Close X` toggle on right
   - Controls FullScreenMenu visibility
4. **Update `docusaurus.config.ts`**:
   - Strip most navbar items (they move into the overlay)
   - Keep logo config
5. **Preserve mobile sidebar** — Docusaurus mobile sidebar still works for docs pages; the overlay menu is for the marketing pages

### Files
- `src/theme/Navbar/index.tsx` (new — swizzled wrapper)
- `src/components/FullScreenMenu/FullScreenMenu.tsx` (new)
- `src/components/FullScreenMenu/styles.module.css` (new)
- `docusaurus.config.ts` (modify navbar items)
- `src/css/custom.css` (overlay z-index, transitions)

---

## Phase 2: Homepage Redesign

### Current State
- Hero: GooseLogo + tagline + "install goose" CTA + YouTube embed (2-column)
- Features: `HomepageFeatures` component with icon cards + testimonials + video

### New Design — Sections (top to bottom)

#### 2.1 Hero Section
- Centered layout, no columns
- Large "goose" wordmark (existing `GooseLogo` component)
- Tagline: "Goose is a general-purpose AI agent created by Block. It helps you code, automate tasks, and solve problems with powerful extensions."
- Two CTAs: `Get Started` (primary/filled) + `View on GitHub` (secondary/outline)

#### 2.2 Product Demo Section
- Full-width embedded Goose desktop app screenshot/video
- Scrolling marquee overlay across the middle: `SOFTWARE BUILT BY THE PEOPLE • MAKE YOUR DREAMS COME TRUE • ALWAYS CUSTOM • ALWAYS FREE •`
- The demo shows the actual Goose UI (screenshot or looping video, NOT an iframe)

#### 2.3 Features Grid Section
- 2×2 card grid:
  - **Developer Tools** — "Code editing and shell commands" with terminal preview showing commands
  - **Task Management** — "Break down complex problems"
  - **Extensions** — "Dynamic plugin system" with visual extension picker (Developer, Analytics, Browser, Memory)
  - **Smart Memory** — "Context-aware conversations"

#### 2.4 Value Props Section
- Three cards in a row:
  - **Open Source** — "Built by Block, for everyone"
  - **Multi-Model** — "Works with various LLMs"
  - **Agentic AI Foundation** — with scrolling model ticker: `DeepSeek • GPT-4 • Claude • Llama • Gemini •`

#### 2.5 Persona Prompts Section
- Four audience labels stacked vertically: `EVERYDAY`, `DEVELOPERS`, `DESIGNERS`, `BUILDERS`
- Each has a horizontally scrolling marquee of example prompts in uppercase:
  - DEVELOPERS: `REFACTOR MY REPO TO USE REACT`, `DEBUG MY CI PIPELINE`, etc.
  - DESIGNERS: `CREATE A TOKEN LIBRARY FOR DARK MODE`, `UNIFY MY LIBRARY STYLING USING TAILWIND`
  - BUILDERS: `BUILD A NEW GARDENING APP`, `SCAFFOLD A REST API`
  - EVERYDAY: `START RESEARCH ON CAKE RECIPES`, `RESPOND TO CUSTOMER EMAILS`

#### 2.6 Footer
- Keep existing footer structure (Quick Links, Community, More, Copyright)

### Implementation

1. **Rewrite `src/pages/index.tsx`** — new homepage with all sections
2. **Rewrite `src/pages/index.module.css`** — new styles
3. **Create new components**:
   - `src/components/HeroSection/HeroSection.tsx`
   - `src/components/ProductDemo/ProductDemo.tsx`
   - `src/components/FeaturesGrid/FeaturesGrid.tsx`
   - `src/components/ValueProps/ValueProps.tsx`
   - `src/components/PersonaPrompts/PersonaPrompts.tsx`
   - `src/components/ScrollingMarquee/ScrollingMarquee.tsx` (reusable)
4. **Add assets**:
   - Goose app demo screenshot or video (`static/img/goose-demo.png` or `static/videos/goose-demo.mp4`)

### Files
- `src/pages/index.tsx` (rewrite)
- `src/pages/index.module.css` (rewrite)
- `src/components/HeroSection/HeroSection.tsx` (new)
- `src/components/HeroSection/styles.module.css` (new)
- `src/components/ProductDemo/ProductDemo.tsx` (new)
- `src/components/ProductDemo/styles.module.css` (new)
- `src/components/FeaturesGrid/FeaturesGrid.tsx` (new)
- `src/components/FeaturesGrid/styles.module.css` (new)
- `src/components/ValueProps/ValueProps.tsx` (new)
- `src/components/ValueProps/styles.module.css` (new)
- `src/components/PersonaPrompts/PersonaPrompts.tsx` (new)
- `src/components/PersonaPrompts/styles.module.css` (new)
- `src/components/ScrollingMarquee/ScrollingMarquee.tsx` (new)
- `src/components/ScrollingMarquee/styles.module.css` (new)

---

## Phase 3: Visual & Interaction Design

### Typography
- Larger, bolder headings throughout
- Hero tagline: 18–20px
- Section labels: ALL CAPS, letter-spacing
- Keep Cash Sans font family

### Color
- Keep Arcade design tokens (light/dark mode)
- More contrast: black/white dominant with minimal accent color
- Primary CTA: solid black (light) / solid white (dark)
- Secondary CTA: outline style

### Animations
- **Scrolling marquees**: CSS `@keyframes` with `translateX` for infinite horizontal scroll (GPU-accelerated)
- **Scroll-triggered reveals**: Framer Motion `whileInView` for section fade-ins
- **Menu overlay**: CSS transitions for open/close (opacity + transform)
- **Hover effects**: Subtle scale/shadow on cards

### Spacing
- More generous whitespace between sections
- Full-bleed sections (edge-to-edge backgrounds)
- Cards with consistent padding and border-radius

### Files
- `src/css/custom.css` (update global styles, marquee keyframes)
- `tailwind.config.js` (extend if needed)
- All new component CSS modules

---

## Phase 4: Content Pages

### New Pages
- **`/features`** — Expanded features page (can be Phase 2 stretch goal)
- **`/why-goose`** — Testimonials + value proposition (move testimonials from current homepage)

### Existing Pages — Keep As-Is
- `/docs/*` — All documentation pages unchanged
- `/blog` — Blog unchanged
- `/extensions` — Minor visual refresh to match new card style
- `/skills` — Keep
- `/recipes` — Keep
- `/prompt-library` — Keep
- `/community` — Keep
- `/deeplink-generator` — Keep
- `/recipe-generator` — Keep

### Files
- `src/pages/features.tsx` (new — stretch)
- `src/pages/why-goose.tsx` (new — stretch)

---

## Phase 5: Technical Notes

### Docusaurus Compatibility
- Homepage and navigation are fully custom React — this is standard Docusaurus practice
- Docs sidebar, blog, and MDX rendering remain untouched
- Swizzling Navbar is the only theme override needed

### Performance
- Marquee animations use CSS `transform: translateX()` (GPU-accelerated, no layout thrashing)
- Product demo: use optimized image or compressed video (not iframe)
- Lazy-load below-fold sections with Framer Motion `whileInView`

### Responsive Design
- Full-screen overlay menu works naturally on all screen sizes
- Marquee sections: `overflow: hidden` on container
- Features grid: 2×2 on desktop → stacked on mobile
- Value props: 3-column → stacked on mobile

### Dark Mode
- All components use Arcade CSS variables (`var(--text-prominent)`, etc.)
- Marquee text uses `currentColor`
- Demo screenshot: provide both light and dark variants, or use a neutral dark screenshot

---

## Execution Order

1. **ScrollingMarquee component** (reusable, needed by multiple sections)
2. **FullScreenMenu + Navbar overhaul** (navigation)
3. **HeroSection** (top of homepage)
4. **ProductDemo** (below hero)
5. **FeaturesGrid** (below demo)
6. **ValueProps** (below features)
7. **PersonaPrompts** (below value props)
8. **Homepage assembly** (wire all sections into index.tsx)
9. **Polish** (animations, responsive, dark mode)
10. **Build & test** (ensure no broken links, docs still work)
