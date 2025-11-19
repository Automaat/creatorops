# CreatorOps Design Guidelines

## Design Philosophy

- Clean, minimal UI inspired by Todoist + FileExplorer
- Prioritize content hierarchy and whitespace
- Light/dark theme core requirement
- Subtle glass morphism (backdrop-filter blur)
- 0.3s transitions standard
- Component-focused modular architecture
- Functional simplicity with visual clarity

## CSS Structure

```text
src/styles/
  variables.css    - tokens (colors, spacing, typography)
  global.css       - resets, base, utilities
  modern.css       - glass effects, animations
  theme.css        - theme switching
  components.css   - component styles
  layouts/         - layout styles
```

## Design Tokens

**Color Palette:** <https://colorhunt.co/palette/fcf9eabadfdbffa4a4ffbdbd>

- `#FCF9EA` - Cream/off-white (bg-primary light mode)
- `#BADFDB` - Soft teal (secondary accent)
- `#FFA4A4` - Muted pink (primary accent)
- `#FFBDBD` - Light pink (tertiary accent/hover)

**Colors:** Semantic naming (bg-primary/secondary/tertiary, text-primary/secondary/tertiary, accent-primary/secondary, hover, status). Separate light/dark palettes. High contrast for readability.

**Spacing:** 2, 4, 8, 12, 16, 24, 32, 48px (xxs→xxl). Generous whitespace for breathing room.

**Typography:** System font stack (system-ui, Segoe UI, Roboto, Arial) fallback to Inter. 12-24px sizes, 400/500/600/700 weights. Optimize for readability.

**Borders:** 4, 6, 8, 12px radius (subtle rounding). 4 shadow levels (minimal, avoid heavy shadows).

**Z-Index:** 1, 10, 20, 50, 70, 100 (base→context-menu)

**Priority System:** Color-coded levels (P1-P4) with visual indicators (text color, icons, or badges).

## Component Patterns

**Layout:** Flexbox-first. Multi-column layouts for content areas. Sidebar + main content structure. 768px mobile breakpoint.

**Lists/Cards:** Nested lists with visual hierarchy. Task/item cards with hover states. Drag-and-drop support UI patterns.

**Views:** Support multiple perspectives (list, board, calendar). Quick-add input patterns with natural entry.

**Effects:** Subtle glass (backdrop blur 8px, use sparingly). Minimal hover states (bg color shift). Avoid heavy shadows—prefer flat design with subtle borders.

**Animations:** fadeIn, slideUp, slideDown (0.3s ease-out). Smooth state transitions. Micro-interactions on actions.

**A11y:** `.visually-hidden`, `:focus-visible`, accent color outlines, keyboard shortcuts support.

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

1. **Content-first** - maximize space for actual content
2. **Visual hierarchy** - size, spacing, color for organization
3. **Whitespace-driven** - generous breathing room, avoid cramped layouts
4. **Minimalism** - remove unnecessary chrome and decoration
5. **Clarity** - high contrast text, readable typography
6. **Consistency** - reuse patterns across components
7. **Performance** - 60fps animations, optimize renders
8. **Desktop-first** - mobile-adapt where needed
9. **Theme-aware** - light/dark from start, respect system preferences
10. **Accessible** - keyboard nav, focus states, semantic HTML

## Anti-Patterns

- ❌ Inline styles
- ❌ Magic numbers
- ❌ Tight coupling
- ❌ Inconsistent spacing
- ❌ Theme-unaware colors
- ❌ Heavy deps for simple UI
- ❌ Non-semantic HTML
- ❌ Heavy shadows/gradients (prefer flat, minimal depth)
- ❌ Cramped layouts (always prioritize whitespace)
- ❌ Over-decorated UI (avoid unnecessary visual noise)

## Example Usage

```css
/* Use tokens */
background: var(--color-bg-primary);
padding: var(--space-md);
gap: var(--space-sm);
```
