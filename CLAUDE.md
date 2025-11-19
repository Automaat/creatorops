# CreatorOps Design Guidelines

## Design Philosophy
- Clean, minimal desktop UI like FileExplorer
- Light/dark theme core requirement
- Glass morphism (backdrop-filter blur)
- 0.3s transitions standard
- Component-focused modular architecture

## CSS Structure
```
src/styles/
  variables.css    - tokens (colors, spacing, typography)
  global.css       - resets, base, utilities
  modern.css       - glass effects, animations
  theme.css        - theme switching
  components.css   - component styles
  layouts/         - layout styles
```

## Design Tokens

**Colors:** Semantic (bg-primary/secondary/tertiary, text-primary/secondary/tertiary, accent, hover, status). Separate light/dark palettes.

**Spacing:** 2, 4, 8, 12, 16, 24, 32px (xxs→xxl)

**Typography:** Inter font, 12-24px sizes, 400/500/600/700 weights

**Borders:** 4, 8, 12, 16px radius. 4 shadow levels.

**Z-Index:** 1, 10, 20, 50, 70, 100 (base→context-menu)

## Component Patterns

**Layout:** Flexbox-first. Grid for cards (auto-fill/minmax). 768px mobile breakpoint.

**Effects:** Glass (backdrop blur 8px), hover spotlights (radial gradient), neumorphic shadows, text gradients.

**Animations:** fadeIn, slideUp, slideDown, zoomIn (0.3s ease-out).

**A11y:** `.visually-hidden`, `:focus-visible`, accent color outlines.

## Tech Stack
- React (hooks, no UI frameworks)
- Vanilla CSS with CSS variables
- Tauri 2 (Rust backend)
- Vite + TypeScript

## Code Style

**React:** Functional components, custom hooks, provider pattern for state.

**Styling:** Vanilla CSS only. NO CSS-in-JS, NO Tailwind. Design tokens in variables.css.

**Naming:** Components PascalCase, hooks camelCase (use*), utils camelCase, styles kebab-case.

## UI Principles
1. Minimalism - remove unnecessary elements
2. Hierarchy - spacing/typography for organization
3. Consistency - reuse patterns
4. Performance - 60fps animations
5. Desktop-first, mobile-adapt
6. Light/dark from start
7. Font smoothing, focus states, micro-interactions

## Anti-Patterns
- ❌ Inline styles
- ❌ Magic numbers
- ❌ Tight coupling
- ❌ Inconsistent spacing
- ❌ Theme-unaware colors
- ❌ Heavy deps for simple UI
- ❌ Non-semantic HTML

## Example Usage
```css
/* Use tokens */
background: var(--color-bg-primary);
padding: var(--space-md);
gap: var(--space-sm);
```
