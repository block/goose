# Goose Desktop UI Style Guide

## Introduction to the CSS Framework

This project uses [Tailwind CSS](https://tailwindcss.com/) — a utility-first CSS framework — to style the Goose desktop application's UI. Tailwind allows developers to apply small, reusable utility classes directly to HTML elements instead of writing custom CSS rules.

* Goose desktop UI favors consistent usage of Goose theme colors and tokens such as `text-textStandard`, `bg-background-default`, `border-borderSubtle`.
* Use the `CustomRadio` component for radio input styling, ensuring accessibility and dark mode support.
* Avoid hardcoded colors like `text-white` on labels or text that conflict with light mode readability.

## Table of Contents

1. Introduction to Tailwind CSS  
2. Design Principles and Theme Tokens  
3. Accessibility Best Practices  
4. Dark Mode Support  
5. Responsive Design  
6. Form Element Styling Rules  
7. Custom Components  
8. Code Maintenance and Best Practices  
9. Testing and Validation  
10. Detailed Style Examples  

---

## Accessibility Best Practices

- Always use semantic HTML elements like `<label>`, `<fieldset>`, `<legend>`, `<button>`.
- Link inputs with labels using `htmlFor` and `id`.
- Use `aria-describedby`, `aria-invalid`, and similar attributes for feedback.
- Ensure focus styles are visible and consistent, e.g., `focus:outline-none focus:ring-2`.
- Maintain color contrast ratios that meet WCAG standards.
- Use keyboard-navigable controls; avoid mouse-only interactions.

---

## Dark Mode Support

- Use the `dark:` prefix for all color utilities that require dark variants.
- Test all forms and components in both light and dark mode.
- Use Goose theme color tokens that adapt to dark mode rather than raw colors.
- Handle third party or SVG icons with theme-aware colors.
- Avoid hardcoded colors that don't adapt automatically.

---

## Responsive Design

- Use `sm:`, `md:`, and `lg:` prefixes to adapt padding, font size, and layout.
- Stack form fields vertically on small screens and use grid/flexbox at larger screens.
- Test mobile usability especially for touch targets and inputs.
- Example: `className="w-full sm:w-auto"`

---

## Design Tokens and Theme Variables

- Goose defines custom tokens like `text-textStandard` which map to specific hex colors.
- Use tokens consistently instead of raw hex or generic colors.
- Customize tokens in the Tailwind config if needed via design lead.
- Examples:

```css
/* Tailwind config snippet */
colors: {
  'textStandard': '#333333',
  'backgroundDefault': '#FFFFFF',
  'borderSubtle': '#E5E7EB',
  // ...
}
```

---

## Code Maintenance and Best Practices

- Avoid repeating large class strings; use component props or utility wrappers.
- Create and reuse components for common patterns to simplify updates.
- Add comments or naming conventions for custom components.
- Prefer utility classes over custom CSS for maintainability.

---

## Testing and Validation

- Use browser devtools to inspect elements for style correctness.
- Run accessibility linters or tools like Axe or Lighthouse.
- Perform user testing in different themes and devices.
- Automate style checking where possible via CI pipelines.

---

* Apply Tailwind utilities for layout such as `flex`, `gap-2`, `items-center` for consistent form element alignment.
* Use margin utilities like `mb-2` and `mt-1` for spacing form labels and error messages.

---



### Key Features of Tailwind CSS:
- Utility classes for controlling colors, typography, layout, spacing, etc.
- Responsive design with prefix utilities like `sm:`, `md:`, `lg:`.
- Supports custom theming via configuration files for colors, fonts, and other styles.
- Highly composable and avoids specificity conflicts due to inline classes.

### How Tailwind is used in Goose
- The UI components are styled with pre-defined Tailwind utility classes for consistency and speed.
- Custom colors and font sizes are defined as part of the project's theme.
- Conditional styling is often applied based on dark/light mode or state using Tailwind's variants.

---

## Best Practices for Tailwind CSS in Goose UI

- Use built-in utility classes for colors, text sizes, margins, padding, etc., to keep styling consistent.
- Leverage semantic component class names sparingly; prefer utility classes directly in JSX.
- Use dark mode variants (`dark:`) to ensure UI works well in both modes.
- Avoid hardcoding colors outside the theme definitions to maintain consistency and theming support.
- Use `peer` and `peer-checked` utilities for styling elements based on input states like checkboxes and radios.
- Ensure accessibility by managing focus styles and using appropriate semantic HTML elements (e.g., `label` with `htmlFor`).
- Organize spacing and flexbox utilities to create intuitive and accessible layouts.

---

## Style Guide for Form Elements

### Label Text
- Use `text-sm` or `text-md` for label font size.
- Use `font-medium` for label font weight.
- Use `text-textStandard` for label color in normal state.
- Use `text-textSubtle` for secondary or less prominent labels.
- Use `mb-2` or `mb-1` to provide bottom margin spacing below labels for separation from inputs.

### Inputs
- Use `border`, `rounded`, and `px-4 py-2` for inputs.
- Use `text-textStandard` or `text-textDefault` for normal text color.
- Use placeholder text color such as `placeholder:text-textSubtle`.
- Use dark mode variants like `dark:bg-background-default` and `dark:text-textStandard`.

### Checkboxes and Radios
- Use native `input[type="checkbox"]` or `input[type="radio"]` with classes `peer sr-only` to visually hide native input.
- Use an adjacent `div` or `span` with border and background utilities to display custom styled checkbox or radio.
- Use `peer-checked:border-black dark:peer-checked:border-white` and `peer-checked:bg-white dark:peer-checked:bg-black` for checked states.
- Use `rounded` or `rounded-full` for boxes and circles.
## Detailed Style Examples

### 1. Label and Input Group

```tsx
<div className="flex flex-col mb-4">
  <label htmlFor="username" className="text-sm font-medium text-textStandard mb-2">
    Username
  </label>
  <input
    id="username"
    type="text"
    className="border rounded px-4 py-2 text-textStandard placeholder:text-textSubtle dark:bg-background-default dark:text-textStandard"
    placeholder="Enter your username"
  />
</div>
```

Key points:
## 6. Custom Radio Component

The Goose UI uses a custom `CustomRadio` React component in place of standard HTML radio inputs. This component offers improved styling control, accessibility, and dark mode compatibility.

### Key Features:
- Native radio input is visually hidden with `peer sr-only`.
- Custom radio circle styled with borders and background changes on checked state using `peer-checked` variants.
- Supports primary and secondary labels with proper vertical stacking and color styles.
- Supports an optional right-aligned content area for additional info/icons.
- Proper handling of disabled state with opacity and cursor changes.
- Smooth animated transitions on check/uncheck.

### Usage Example:

```tsx
import CustomRadio from '../ui/CustomRadio';

<CustomRadio
  id="agreement"
  name="agreement"
  value="yes"
  checked={agreed}
  onChange={(e) => setAgreed(e.target.checked)}
  label="I agree to the terms and conditions"
  secondaryLabel="You must accept before continuing."
  rightContent={<SomeIcon />}
/>
```

### Styling Notes:
- The custom input circle is `h-4 w-4` with `rounded-full` border.
- Checked state uses a thicker border and background fill with colors adapting to dark mode.
- Labels use `text-sm` for primary and `text-xs` for secondary text.
- Colors adapt for light/dark mode using Tailwind color utilities.

Use `CustomRadio` throughout the Goose desktop UI for consistent radio selection controls.

---

This section can be referenced when building new forms or refactoring old ones to enhance UI consistency and accessibility.

- Use `flex flex-col` to stack label and input vertically.
- Label uses `text-sm font-medium` for readable text size and emphasis.
- Proper bottom margin (`mb-2`) to separate label from input.
- Input uses `border` and `rounded` for visible border and rounded corners.
- Use padding for comfortable click/touch target.
- Use `text-textStandard` for text color and `placeholder:text-textSubtle` for placeholder.
- Use dark mode classes for background and text.

---

### 2. Checkbox with Label

```tsx
<label className="flex items-center gap-2 cursor-pointer">
  <input
    type="checkbox"
    className="peer sr-only"
  />
  <div
    className="h-4 w-4 rounded border border-gray-500 peer-checked:border-black dark:border-gray-400 dark:peer-checked:border-white peer-checked:bg-black dark:peer-checked:bg-white transition-colors"
  />
  <span className="text-sm font-medium text-textStandard">
    Accept terms and conditions
  </span>
</label>
```

Key points:
- Wrap checkbox and label text in a `flex items-center gap-2` container.
- Hide native checkbox with `peer sr-only`.
- Use an adjacent `div` styled with borders and background to indicate checked state.
- Use `peer-checked` variants to change border and background colors on check.
- Use accessible `cursor-pointer` for the entire label.

---

### 3. Error Message Display

```tsx
{error && <p className="text-red-500 mt-1 text-sm">{error}</p>}
```

Key Points:
- Use `text-red-500` for error text color.
- Provide `mt-1` margin to separate from the above element.
- Use `text-sm` for smaller readable text.

---

### 4. Grouping Dependent Controls

```tsx
<div className="flex items-start gap-2">
  <label className="flex items-center gap-2 cursor-pointer">
    <input type="checkbox" className="peer sr-only" />
    <div className="h-4 w-4 rounded border border-gray-500 peer-checked:border-black dark:border-gray-400 dark:peer-checked:border-white peer-checked:bg-black dark:peer-checked:bg-white transition-colors" />
    <span className="text-sm font-medium text-textStandard">Use local model (no API key needed)</span>
  </label>
  <input
    type="text"
    placeholder="Enter API key"
    className="border rounded px-4 py-2 text-textStandard placeholder:text-textSubtle dark:bg-background-default dark:text-textStandard flex-grow"
  />
</div>
```

Key points:
- Use `flex items-start gap-2` to align checkbox and dependent input horizontally.
- Checkbox is wrapped in label for accessibility.
- Dependent input expands with `flex-grow`.

---

### 5. Button Styling

```tsx
<button className="px-6 py-3 bg-background-muted text-textStandard rounded-lg hover:bg-background-hover font-medium transition-colors">
  Submit
</button>
```

Key points:
- Appropriate padding for comfortable interaction.
- Background color from theme with hover variants.
- Font weight for emphasis.
- Smooth transitions for hover effects.

---

This extended style guide with examples can be used as reference to fix existing forms and to create new consistent UI screens in Goose desktop.

- Wrap the checkbox and label in `flex items-center gap-2` containers for alignment.

### Error and Validation States
- Use `text-red-500` for error messages.
- Position error messages below inputs or fields with `mt-1`.

### Layout
- Use `flex` and `items-center` to align checkboxes with labels.
- Use `gap-x-2` or `gap-2` between label and input elements.
- Use `block` or `w-full` on labels or inputs where full width is appropriate.

---

This style guide ensures that Goose desktop UI elements are consistent across different forms and components, improving both visual coherence and usability.

Feel free to ask for detailed examples or specific file updates based on this style guide.
