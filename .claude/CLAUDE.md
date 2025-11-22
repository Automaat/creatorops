# CreatorOps Design Guidelines

## Core Philosophy

**Spacious Minimalism** - Prioritize whitespace and clarity over density. Every element should breathe.

- Clean, minimal UI inspired by Todoist + FileExplorer
- Component-focused modular architecture
- Functional simplicity with visual clarity
- Desktop-first with mobile adaptation

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

## Spacing System

**8px Grid Base** - All spacing, padding, margins must be multiples of 8px (8, 16, 24, 32, 40, 48)

**Generous Padding:**

- Cards: 24px internal padding
- Sections: 40px vertical separation
- Page margins: 32px from edges
- List items: 16px vertical padding

**Line Height:**

- Body text: 1.6
- Headings: 1.3
- Compact UI: 1.4

## Color Palette

**Neutrals:**

- Background: `#FEFEFE`
- Surface: `#FFFFFF`
- Border: `#E5E5E5`
- Text primary: `#202020`
- Text secondary: `#666666`
- Text tertiary: `#999999`

**Accent:**

- Primary action: `#D68406` (orange)
- Hover: `#C07605`
- Success: `#4CAF50`
- Sidebar highlight: `#FFF9E6` (warm cream)

**Principle:** Use one accent color consistently. Avoid multiple competing colors.

**Semantic naming:** bg-primary/secondary/tertiary, text-primary/secondary/tertiary, accent-primary/secondary, hover, status. High contrast for readability.

## Typography

**Semantic Type Scale:**

- **Title**: 32px / 700 bold / 1.3 line-height — Page/project titles
- **Section Heading**: 18px / 500 medium / 1.4 line-height — Major sections (e.g., "Actions")
- **Subheading**: 13px / 600 semibold / 1.4 line-height — Group labels (e.g., "Photos", "Videos")
- **Body**: 16px / 600 semibold / 1.5 line-height — Primary content, action names
- **Caption**: 14px / 400 normal / 1.6 line-height — Secondary descriptive text
- **Meta**: 12px / 500 medium / 1.4 line-height — Field labels, small UI text

**Font Family:** System font stack (-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif)

**Usage:** Use semantic type tokens from `variables.css`:

- `var(--type-title-size)`, `var(--type-title-weight)`, `var(--type-title-height)`
- `var(--type-section-size)`, `var(--type-section-weight)`, `var(--type-section-height)`
- `var(--type-subheading-size)`, `var(--type-subheading-weight)`, `var(--type-subheading-height)`
- `var(--type-body-size)`, `var(--type-body-weight)`, `var(--type-body-height)`
- `var(--type-caption-size)`, `var(--type-caption-weight)`, `var(--type-caption-height)`
- `var(--type-meta-size)`, `var(--type-meta-weight)`, `var(--type-meta-height)`

**Principle:** Clear hierarchy through size and weight, not color variations. Apply consistently across all components.

## Visual Weight

**Borders:**

- Prefer subtle shadows over borders
- If borders needed: 1px solid `#E5E5E5`
- Never use thick borders (>1px)

**Shadows:**

- Default card: `0 1px 3px rgba(0,0,0,0.08)`
- Hover state: `0 4px 12px rgba(0,0,0,0.12)`
- Modal/dropdown: `0 8px 24px rgba(0,0,0,0.15)`

**Principle:** Depth through shadows, not borders or background changes.

## Components

**Border Radius:**

- Cards: 8px
- Buttons: 6px
- Badges: 4px
- Inputs: 6px

**Buttons:**

- Height: 40px minimum
- Padding: 12px 24px
- Primary: Orange background, white text
- Secondary: White background, gray border
- Ghost: No background, gray text

**Cards:**

- White background
- Subtle shadow
- No visible border
- Hover: Lift with increased shadow

**Layout:**

- Flexbox-first
- Multi-column layouts for content areas
- Sidebar + main content structure

**Lists/Cards:**

- Nested lists with visual hierarchy
- Task/item cards with hover states
- Drag-and-drop support UI patterns

**Views:**

- Support multiple perspectives (list, board, calendar)
- Quick-add input patterns with natural entry

**Effects:**

- Subtle glass (backdrop blur 8px, use sparingly)
- Minimal hover states (bg color shift)
- Avoid heavy shadows—prefer flat design with subtle borders

## Interactions

**Transitions:**

- Duration: 200ms
- Easing: ease-in-out
- Properties: transform, box-shadow, background-color

**Hover States:**

- Cards: `translateY(-2px)` + shadow increase
- Buttons: Darken background 10%
- Links: Opacity 0.7

**Animations:**

- fadeIn, slideUp, slideDown (0.3s ease-out)
- Smooth state transitions
- Micro-interactions on actions

**Principle:** All interactions should feel instant and responsive.

## Layout

**Sidebar:**
- Width: 240px fixed
- Background: `#FAFAFA`
- No borders, use shadow for separation
- Selected item: Warm cream background (`#FFF9E6`)

**Content Area:**
- Max width: 1200px for readability
- Padding: 32px minimum
- No edge-to-edge content

**Principle:** Content should never feel cramped or touch edges.

## Data Display

**Dates:**
- Format: "Nov 20, 2025" or "20 Nov 2025"
- Never: "2025-11-20" in UI
- Relative: "Today", "Tomorrow", "2 days ago"

**Status Badges:**
- Small: 12px text
- Light backgrounds: Pastel tints
- Minimal visual weight

**Counts:**
- Position: Right-aligned
- Style: 13px, medium weight, secondary color

## Iconography

**Style:**
- Outline/stroke icons only
- 20px default size
- 1.5-2px stroke width
- Consistent visual weight

**Usage:**
- Always pair with labels in navigation
- Use sparingly in content
- Maintain 8px spacing from text

## Empty States

- Never leave blank space
- Add illustrations or placeholder content
- Use muted colors
- Provide clear next action

## Mobile Considerations

**Responsive Breakpoints:**
- Desktop: 1024px+
- Tablet: 768px - 1023px
- Mobile: < 768px

**Touch Targets:**
- Minimum: 44px height
- Spacing: 8px minimum between targets

## Accessibility

- `.visually-hidden` for screen readers
- `:focus-visible` for keyboard navigation
- Accent color outlines
- Keyboard shortcuts support
- Semantic HTML

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

1. **Spacious Minimalism** - every element breathes with whitespace
2. **Content-first** - maximize space for actual content
3. **Visual hierarchy** - size, spacing, color for organization
4. **Whitespace-driven** - generous breathing room, avoid cramped layouts
5. **Clarity** - high contrast text, readable typography
6. **Consistency** - reuse patterns across components
7. **Performance** - 60fps animations, optimize renders
8. **Desktop-first** - mobile-adapt where needed
9. **Accessible** - keyboard nav, focus states, semantic HTML

## Anti-Patterns

- ❌ Inline styles
- ❌ Magic numbers (use 8px grid multiples)
- ❌ Tight coupling
- ❌ Inconsistent spacing
- ❌ Multiple competing accent colors
- ❌ Heavy deps for simple UI
- ❌ Non-semantic HTML
- ❌ Thick borders (>1px)
- ❌ Cramped layouts (always prioritize whitespace)
- ❌ Over-decorated UI (avoid unnecessary visual noise)
- ❌ Edge-to-edge content

## Verification

After adding functionality, verify using CI commands:

**Frontend:**
```bash
npm run format:check
npm run lint
npm run test -- --run
npm run build
```

**Rust:**
```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml --check
mise run lint:rust
mise run test:rust
cd src-tauri && cargo build --release
```

**All (via mise):**
```bash
mise run fmt
mise run lint
mise run test
```

## Example Usage

```css
/* Use 8px grid spacing tokens */
padding: 24px;
margin-bottom: 40px;
gap: 16px;

/* Use semantic color tokens */
background: var(--color-bg-primary);
color: var(--color-text-primary);
border: 1px solid var(--color-border);

/* Use shadow tokens for depth */
box-shadow: var(--shadow-card);

/* Use typography scale */
font-size: 14px;
line-height: 1.6;
font-weight: 400;
```
