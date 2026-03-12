---
name: Square Design Guidelines
description: Square brand-faithful visuals. Always review this skill prior to producing sites (HTML / CSS / etc), Slidev slides, SVG, PDF, email templates, image generation prompts, or any other visual mediums.
metadata:
  version: "1.2"
  category: design
  tags:
    - html
    - UI
    - UX
    - design
    - images
    - slideshows
    - presentations
---


# Square Design Guidelines

Use these design tokens to produce Square brand-faithful visuals in any medium: HTML/CSS, Slidev slides, SVG, PDF, email templates, image generation prompts, and more.

**STOP!** Before you proceed, open and carefully review at least 1 most relevant example under `/examples`.

---

## Design Principles

Your design should be:

### Authentic

We capture the real—real people, real moments, real impact. We show the grit and heart of our sellers with honesty and empathy. Nothing is staged or exaggerated. Every choice we make puts our sellers first and brings their stories to life in ways that feel true, relatable, and genuine.

- Candid, in-the-moment photography that’s not over lit or appears staged
- Custom type inspired by the city it represents
- Product renderings and photos that are realistically in-use
- Textures and ephemera from our seller’s worlds

### Refined

We design with intention — every detail has a purpose. We remove what’s unnecessary so the essential can stand out. Our work is rooted in simplicity but is never boring. It allows the story to lead and the design to lift it, never compete with it. Every decision is shaped by context, ensuring it fits the moment, the medium, and the message.

- Multiple, well balanced type styles show craft and elevate the seller’s story
- Refined sizing and contrast of elements aid in legibility and focus
- Intriguing photo crops with graphic detail
- Uncluttered grids and cohesive art direction

### Eclectic

We reflect the energy and character of the businesses and neighborhoods we serve. Our role is to amplify their world — not overshadow it. We draw from visual cues that speak to their vibe, their voice, their culture — shaping design that feels true to them, but unmistakably Square.

- Custom type inspired by seller swag
- Illustrations from the music-store genre
- Interior wallpaper pattern from the seller’s store, and custom illustration of their storefront
- Glyph animation in product

### Unexpected

From layout to interaction to installations, we explore new formats and original ideas to create work that feels fresh, engaging, and elevated. Storytelling is always at the core—but how we bring it to life is where we challenge convention and raise the bar.

- A physical storefront in a local neighborhood is new for Square, and the category
- Small details, like a tiny comb glyph at the end of a paragraph about a beauty seller
- A newsprint zine instead of a glossy direct mailer
- Showing hardware in a related but creative context

## Font

### Font Families

| Purpose | CSS font-family |
|---------|-----------------|
| Body/UI text | `'Cash Sans', 'Helvetica Neue', Helvetica, Arial, sans-serif` |
| Display/headings | `'Cash Sans', 'Helvetica Neue', Helvetica, Arial, sans-serif` |
| Serif display/editorial | `'Exact Block', Georgia, 'Times New Roman', serif` |
| Code/mono | `'Cash Sans Mono', 'Roboto Mono', monospace` |

**Exact Block** is Square's serif display typeface. Use it for editorial, campaign, and marketing pages where a serif headline creates contrast against Cash Sans body text. Do not use it for UI chrome, forms, or data-dense contexts.

### CDN Font Files

**Cash Sans Base URL**: `https://square-fonts-production-f.squarecdn.com/v1.2.0/`

| Style | Path |
|-------|------|
| Regular | `cash-sans/CashSans-Regular.woff2` |
| Regular Italic | `cash-sans/CashSans-RegularItalic.woff2` |
| Medium | `cash-sans/CashSans-Medium.woff2` |
| Medium Italic | `cash-sans/CashSans-MediumItalic.woff2` |
| Semibold | `cash-sans/CashSans-Semibold.woff2` |
| Semibold Italic | `cash-sans/CashSans-SemiboldItalic.woff2` |
| Bold | `cash-sans/CashSans-Bold.woff2` |
| Bold Italic | `cash-sans/CashSans-BoldItalic.woff2` |
| Mono Regular | `cash-sans-mono/CashSansMono-Regular.woff2` |
| Mono Medium | `cash-sans-mono/CashSansMono-Medium.woff2` |
| Mono Semibold | `cash-sans-mono/CashSansMono-Semibold.woff2` |
| Mono Bold | `cash-sans-mono/CashSansMono-Bold.woff2` |

**Exact Block Base URL**: `https://campaign-hub-production-f.squarecdn.com/static/fonts/exact/`

| Style | Path |
|-------|------|
| Regular | `ExactBlock-Regular.woff2` |
| Italic | `ExactBlock-Italic.woff2` |

```css
@font-face {
  font-family: 'Cash Sans';
  src: url('https://square-fonts-production-f.squarecdn.com/v1.2.0/cash-sans/CashSans-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Cash Sans';
  src: url('https://square-fonts-production-f.squarecdn.com/v1.2.0/cash-sans/CashSans-RegularItalic.woff2') format('woff2');
  font-weight: 400;
  font-style: italic;
  font-display: swap;
}

@font-face {
  font-family: 'Cash Sans';
  src: url('https://square-fonts-production-f.squarecdn.com/v1.2.0/cash-sans/CashSans-Medium.woff2') format('woff2');
  font-weight: 500;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Cash Sans';
  src: url('https://square-fonts-production-f.squarecdn.com/v1.2.0/cash-sans/CashSans-MediumItalic.woff2') format('woff2');
  font-weight: 500;
  font-style: italic;
  font-display: swap;
}

@font-face {
  font-family: 'Cash Sans';
  src: url('https://square-fonts-production-f.squarecdn.com/v1.2.0/cash-sans/CashSans-Semibold.woff2') format('woff2');
  font-weight: 600;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Cash Sans';
  src: url('https://square-fonts-production-f.squarecdn.com/v1.2.0/cash-sans/CashSans-SemiboldItalic.woff2') format('woff2');
  font-weight: 600;
  font-style: italic;
  font-display: swap;
}

@font-face {
  font-family: 'Cash Sans';
  src: url('https://square-fonts-production-f.squarecdn.com/v1.2.0/cash-sans/CashSans-Bold.woff2') format('woff2');
  font-weight: 700;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Cash Sans';
  src: url('https://square-fonts-production-f.squarecdn.com/v1.2.0/cash-sans/CashSans-BoldItalic.woff2') format('woff2');
  font-weight: 700;
  font-style: italic;
  font-display: swap;
}

@font-face {
  font-family: 'Cash Sans Mono';
  src: url('https://square-fonts-production-f.squarecdn.com/v1.2.0/cash-sans-mono/CashSansMono-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Cash Sans Mono';
  src: url('https://square-fonts-production-f.squarecdn.com/v1.2.0/cash-sans-mono/CashSansMono-Medium.woff2') format('woff2');
  font-weight: 500;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Exact Block';
  src: url('https://campaign-hub-production-f.squarecdn.com/static/fonts/exact/ExactBlock-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Exact Block';
  src: url('https://campaign-hub-production-f.squarecdn.com/static/fonts/exact/ExactBlock-Italic.woff2') format('woff2');
  font-weight: 400;
  font-style: italic;
  font-display: swap;
}
```

---

## Color Palette

Square uses a monochrome color palette. Mix this judiciously with the extended color palette.

### Primary color palette

| Token | Hex | Use |
|-------|-----|-----|
| `text-10` | `#101010` | Primary text, headings |
| `text-20` | `#666666` | Secondary text, descriptions |
| `text-30` | `#959595` | Tertiary/placeholder text |
| `text-inverse` | `#FFFFFF` | Text on dark fills |
| `text-black` | `#101010` | Always-black text |
| `text-white` | `#FFFFFF` | Always-white text |
| `fill-10` | `#101010` | Primary fill (buttons, icons) |
| `fill-20` | `#959595` | Secondary fill |
| `fill-30` | `#dadada` | Tertiary fill, borders |
| `fill-40` | `#f0f0f0` | Subtle backgrounds, hover states |
| `fill-50` | `#F7F7F7` | Lightest fill, card backgrounds |
| `fill-inverse` | `#FFFFFF` | Inverse fill |
| `fill-black` | `#000000` | Constant black |
| `fill-white` | `#FFFFFF` | Constant white |
| `divider-10` | `#959595` | Strong dividers |
| `divider-20` | `#f0f0f0` | Subtle dividers |
| `surface-5` | `#FFFFFF` | Page background |
| `surface-10` | `#FFFFFF` | Card/section background |
| `surface-20` | `#FFFFFF` | Elevated surface |
| `surface-30` | `#FFFFFF` | Highest elevation |
| `surface-inverse` | `#232323` | Inverse surface (dark cards) |
| `emphasis-fill` | `#101010` | Primary action fill (was blue, now black) |
| `emphasis-text` | `#101010` | Primary action text (was blue, now black) |
| `emphasis-10` | `#666666` | Emphasis shade 1 |
| `emphasis-20` | `#333333` | Emphasis shade 2 |
| `emphasis-30` | `#CCCCCC` | Emphasis tint 1 |
| `emphasis-40` | `#E8E8E8` | Emphasis tint 2 |
| `focus` | `black` | Focus ring color |

### Semantic colors

| Role | Fill | Text (light mode) | When to use |
|------|------|-------------------|-------------|
| `emphasis` | `#006AFF` | `#005AD9` | Interactive elements, links, focus rings |
| `success` | `#00B23B` | `#007D2A` | Success states, positive values |
| `warning` | `#FF9F40` | `#945C25` | Warnings, caution states |
| `critical` | `#CC0023` | `#BF0020` | Errors, destructive actions |

### Extended palette

| Color | Fill | Text (light mode) |
|-------|------|-------------------|
| `green` | `#00B23B` | `#007D2A` |
| `forest` | `#19802A` | `#007D2A` |
| `teal` | `#12BF94` | `#0C785D` |
| `blue` | `#006AFF` | `#005AD9` |
| `sky` | `#2693FF` | `#0F65BA` |
| `purple` | `#8716D9` | `#8716D9` |
| `pink` | `#D936B0` | `#A82A88` |
| `burgundy` | `#990838` | `#990838` |
| `red` | `#CC0023` | `#BF0020` |
| `orange` | `#F25B3D` | `#A83F2A` |
| `gold` | `#FF9F40` | `#945C25` |
| `yellow` | `#FFBF00` | `#876500` |
| `taupe` | `#A67C53` | `#826141` |
| `brown` | `#664A2E` | `#664A2E` |

---

## Typography

### Type Scale Reference

| Semantic Name | Size | Line Height | Weight | Letter Spacing | Font | Use |
|--------------|------|-------------|--------|----------------|------|-----|
| `keypad-total` | 96px | 96px | 500 (medium) | 0 | Display | Large keypad display numbers |
| `hero` | 56px | 56px | 400 (regular) | -2.24px | Display | Primary hero text |
| `numeral-large` | 56px | 56px | 500 (medium) | 0 | Display | Large numerical displays |
| `headline-large` | 44px | 44px | 400 (regular) | -1.43px | Display | Major page headlines |
| `headline-small` | 32px | 32px | 400 (regular) | -0.8px | Display | Section headlines |
| `header` | 28px | 32px | 500 (medium) | 0 | Display | General headers |
| `page-title` | 32px | 32px | 500 (medium) | -0.48px | Display | Page titles |
| `section-title` | 24px | 24px | 500 (medium) | -0.18px | Text | Section headers |
| `numeral-small` | 32px | 32px | 500 (medium) | 0 | Display | Smaller numerical displays |
| `label-medium` | 16px | 24px | 500 (medium) | 0 | Text | Primary labels, form fields, buttons |
| `body-medium` | 16px | 24px | 400 (regular) | -0.08px | Text | Primary body text |
| `link-medium` | 16px | 24px | 500 (medium) | 0 | Text | Primary links (underlined) |
| `label-small` | 14px | 20px | 500 (medium) | 0.035px | Text | Secondary labels |
| `body-small` | 14px | 20px | 400 (regular) | -0.035px | Text | Secondary body text |
| `link-small` | 14px | 20px | 500 (medium) | 0.035px | Text | Secondary links (underlined) |
| `button` | 16px | 24px | 500 (medium) | 0 | Text | Standard button text |
| `button-compact` | 14px | 16px | 500 (medium) | 0.035px | Text | Compact button text |
| `label-x-small` | 10px | 16px | 500 (medium) | 0.6px | Mono | Micro labels, tags (UPPERCASE) |
| `body-x-small` | 10px | 16px | 400 (regular) | 0.3px | Mono | Fine print, metadata |
| `link-x-small` | 10px | 16px | 500 (medium) | 0.4px | Mono | Micro links (underlined) |

---

## Spacing

```
2px  4px  8px  12px  16px  20px  24px  32px  40px  48px  64px  80px  120px  160px
```

Use **8px** as the base unit. Common patterns:

| Context | Spacing |
|---------|---------|
| Inline spacing (icon + label) | 8px |
| Button padding (vertical) | 12px |
| Button padding (horizontal) | 16–24px |
| Card padding | 16–24px |
| Section gaps | 32–48px |
| Page margins | 24–40px |

---

## Border Radius

| Token | Value | Use |
|-------|-------|-----|
| `none` | 0px | Sharp corners (tables, dividers) |
| `33` | 2px | Subtle rounding |
| `66` | 4px | Small elements |
| `100` / `forms` | 6px | Form inputs, text fields, textareas |
| `200` / `modals` | 12px | Cards, modals, dialogs |
| `266` | 16px | Large cards |
| `400` | 24px | Prominent containers |
| `533` | 32px | Large rounded elements |
| `circle` | 1000px | Pills, avatars, circular buttons |

### Monochrome Button Radius Override

Buttons use **pill shape** (border-radius = height/2):

| Button Size | Radius |
|-------------|--------|
| Small | 20px |
| Medium | 24px |
| Large | 32px |

---

## Borders

| Type | Value |
|------|-------|
| Thin | 1px solid |
| Medium | 2px solid |
| Thick | 8px solid |
| Default color | `divider-20` (`#f0f0f0` light / `#333333` dark) |
| Strong color | `divider-10` (`#959595` both modes) |

---

## Breakpoints

| Name | Min | Max | Use |
|------|-----|-----|-----|
| `narrow` | 0px | 599px | Mobile |
| `medium` | 600px | 839px | Tablet |
| `wide` | 800px | 1023px | Small desktop |
| `extraWide` | 1024px+ | — | Desktop |

---

## Animation

| Transition | Easing | Fast | Moderate | Slow |
|-----------|--------|------|----------|------|
| Enter | `cubic-bezier(0.26, 0.10, 0.48, 1.0)` | 100ms | 240ms | 400ms |
| Exit | `cubic-bezier(0.52, 0.0, 0.74, 0.0)` | 100ms | 160ms | 300ms |
| Move | `cubic-bezier(0.76, 0.0, 0.24, 1.0)` | 100ms | 240ms | 400ms |

---

## Opacity

| State | Value |
|-------|-------|
| Disabled | 0.4 |

---

## Component Recipes

### Primary Button (Monochrome)

**Visual Spec**:
- Solid black fill in light mode, solid white fill in dark mode
- Pill shape (full height radius)
- 16px/24px medium weight text, inverse color
- 12px vertical padding, 24px horizontal padding

**When to Use**: Primary actions, form submissions, CTAs

**HTML/CSS Implementation**:

```html
<button class="btn-primary">Get Started</button>
```

```css
.btn-primary {
  font-family: 'Cash Sans', 'Helvetica Neue', Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 500;
  letter-spacing: 0;
  background-color: #101010;
  color: #FFFFFF;
  border: none;
  border-radius: 24px;
  padding: 12px 24px;
  cursor: pointer;
  transition: background-color 100ms cubic-bezier(0.26, 0.10, 0.48, 1.0);
}

.btn-primary:hover {
  background-color: #333333;
}

.btn-primary:focus {
  outline: 2px solid black;
  outline-offset: 2px;
}

.btn-primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
```

---

### Secondary Button

**Visual Spec**:
- Transparent background with 1px border
- Pill shape (full height radius)
- 16px/24px medium weight text
- Border and text use `#959595` light / varies dark

**When to Use**: Secondary actions, cancel buttons, alternative options

**HTML/CSS Implementation**:

```html
<button class="btn-secondary">Cancel</button>
```

```css
.btn-secondary {
  font-family: 'Cash Sans', 'Helvetica Neue', Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 500;
  letter-spacing: 0;
  background-color: transparent;
  color: #101010;
  border: 1px solid #959595;
  border-radius: 24px;
  padding: 12px 24px;
  cursor: pointer;
  transition: all 100ms cubic-bezier(0.26, 0.10, 0.48, 1.0);
}

.btn-secondary:hover {
  background-color: #f0f0f0;
}

.btn-secondary:focus {
  outline: 2px solid black;
  outline-offset: 2px;
}

.btn-secondary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
```

---

### Tertiary Button (Text Button)

**Visual Spec**:
- No background, no border
- Underline text (1px) in monochrome theme
- 16px/24px medium weight text

**When to Use**: Low-emphasis actions, inline links, "Learn more"

**HTML/CSS Implementation**:

```html
<button class="btn-tertiary">Learn more</button>
```

```css
.btn-tertiary {
  font-family: 'Cash Sans', 'Helvetica Neue', Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 500;
  letter-spacing: 0;
  background-color: transparent;
  color: #101010;
  border: none;
  text-decoration: underline;
  text-underline-offset: 2px;
  padding: 12px 0;
  cursor: pointer;
}

.btn-tertiary:hover {
  color: #666666;
}

.btn-tertiary:focus {
  outline: 2px solid black;
  outline-offset: 2px;
}
```

---

### Text Input / Field

**Visual Spec**:
- 6px border-radius (forms token)
- 1px border using `divider-20`
- 16px/24px regular weight input text
- Placeholder uses `text-30`
- Focus: border changes to `emphasis-fill`

**When to Use**: Form inputs, search fields, text entry

**HTML/CSS Implementation**:

```html
<input type="text" class="input-field" placeholder="Enter your name" />
```

```css
.input-field {
  font-family: 'Cash Sans', 'Helvetica Neue', Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;
  letter-spacing: -0.08px;
  background-color: #FFFFFF;
  color: #101010;
  border: 1px solid #f0f0f0;
  border-radius: 6px;
  padding: 12px 16px;
  width: 100%;
  transition: border-color 100ms cubic-bezier(0.26, 0.10, 0.48, 1.0);
}

.input-field::placeholder {
  color: #959595;
}

.input-field:focus {
  outline: none;
  border-color: #101010;
}

.input-field:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
```

---

## Common Mistakes
- Designing a boring grid of cards. Take inspiration from `./examples`.
- Not being receptive to feedback. If the merchant wants to take the design elsewhere, oblige. These guidelines are defaults, not religion.
- Not reviewing your designs after implementation. Review, and refine.