# cclog — Visual Design System v0.1

Status: Active (2026-05-31)
Purpose: Single source of truth for the cclog static HTML viewer visual system. Every template, component, and CSS token references this document.

---

## 1. Palette

### Light Theme

| Token | Hex | Usage |
|---|---|---|
| `bg-surface` | `#FAF7F2` | Page background — warm off-white, not pure white |
| `bg-surface-elevated` | `#F3EFE8` | Cards, elevated containers |
| `bg-surface-hover` | `#EDE8E0` | Hover state for cards/rows |
| `bg-surface-pressed` | `#E5DFD5` | Active/pressed state |
| `text-primary` | `#1E1B18` | Primary text on surface — near-black warm |
| `text-secondary` | `#5C5853` | Secondary/muted text |
| `text-tertiary` | `#8B8680` | Tertiary/placeholder text |
| `text-accent` | `#C2592E` | Accent text — warm terracotta |
| `bg-accent` | `#D4683A` | Accent background — warm terracotta |
| `bg-accent-hover` | `#C2592E` | Accent hover — slightly darker |
| `ring-accent` | `#D4683A` | Focus rings, active borders |
| `border-subtle` | `#E8E3DB` | Subtle borders |
| `border-default` | `#D5CFC6` | Default borders |
| `success` | `#3B8C5A` | Positive / active status |
| `error` | `#C5443B` | Error / destructive |
| `warning` | `#D4852B` | Warning |

### Dark Theme

| Token | Hex | Usage |
|---|---|---|
| `bg-surface` | `#161410` | Page background — warm dark |
| `bg-surface-elevated` | `#1F1C18` | Cards, elevated containers |
| `bg-surface-hover` | `#28241F` | Hover state for cards/rows |
| `bg-surface-pressed` | `#322D27` | Active/pressed state |
| `text-primary` | `#EBE5DD` | Primary text — warm off-white |
| `text-secondary` | `#A0998F` | Secondary/muted text |
| `text-tertiary` | `#6B6660` | Tertiary/placeholder text |
| `text-accent` | `#E8926C` | Accent text — lighter terracotta for dark bg |
| `bg-accent` | `#D4683A` | Accent background |
| `bg-accent-hover` | `#E07B50` | Accent hover — slightly lighter |
| `ring-accent` | `#E8926C` | Focus rings on dark surfaces |
| `border-subtle` | `#2A2621` | Subtle borders |
| `border-default` | `#3D3832` | Default borders |
| `success` | `#5BAD75` | Positive / active status |
| `error` | `#D9665E` | Error / destructive |
| `warning` | `#E2A04A` | Warning |

### State Colors (shared across themes)

Used for role dots, message-type indicators, and semantic labels.

| Token | Hex | Usage |
|---|---|---|
| `state-user` | `#D4852B` | User message indicator |
| `state-assistant` | `#D4683A` | Assistant message indicator (uses accent) |
| `state-thinking` | `#8B7EC8` | Thinking block indicator (muted purple — decorative only, not primary UI) |
| `state-tool` | `#4A9E8E` | Tool call/result indicator |
| `state-system` | `#8B8680` | System message indicator |

### Explicitly Removed

- `#7C4DFF` (purple primary) — replaced by terracotta/amber
- `#6200EE` (purple primary light) — replaced
- `#3700B3` (purple container) — replaced
- `#03DAC6` (teal secondary) — replaced by muted warm tones
- No purple (`#7C4DFF`, `#6200EE`, `#3700B3`, `#D4BFF9`, `#E8DEF8`, `#21005D`) anywhere in rendered output

---

## 2. Type Scale

| Token | Size / Weight / Family | Usage |
|---|---|---|
| `text-display` | 28px / 700 / Space Grotesk | Page titles (h1) |
| `text-heading` | 18px / 600 / Geist | Card titles, section headings |
| `text-subheading` | 14px / 600 / Geist | Session card titles |
| `text-body` | 14px / 400 / Geist | Body text, stats labels |
| `text-body-sm` | 13px / 400 / Geist | Secondary body |
| `text-mono` | 12px / 400 / JetBrains Mono | Encoded paths, IDs, timestamps |
| `text-mono-sm` | 11px / 400 / JetBrains Mono | Meta labels, chip text |
| `text-mono-xs` | 10px / 600 / JetBrains Mono | Column headers, uppercase labels |
| `text-caption` | 11px / 500 / Geist | Stat labels in cards |

### Type Treatment

- **Project names** (table/card): `text-heading` (18px/600) in card view, `text-body` weight (13px/600) in list view
- **Encoded paths**: `text-mono-sm` (11px) with `text-secondary` color, truncated with ellipsis
- **Stat values**: `text-mono` (12px/600) in `text-primary`
- **Stat labels**: `text-caption` (11px/500) uppercase tracked at 0.05em in `text-tertiary`

---

## 3. Spacing Scale

| Token | Value | Usage |
|---|---|---|
| `space-xs` | 4px | Tight internal gaps (icon+label) |
| `space-sm` | 8px | Element gaps within a component |
| `space-md` | 12px | Related component gaps |
| `space-lg` | 16px | Section gaps (card grid gap, toolbar gap) |
| `space-xl` | 24px | Major section gaps, page padding |
| `space-2xl` | 32px | Header-to-content gap |
| `space-3xl` | 48px | Page top/bottom padding |

### Layout Rhythm

```
[Page top padding: 48px]
[Header band: title + stats strip + date range]   gap: 20px
[Toolbar band: search · date chips · view-switcher]   gap: 16px
[Content: table or grid]   gap: 16px (grid), 2px (list)
[Footer]   gap: 24px above, 24px below
```

---

## 4. Component States

### Buttons (view-switcher, filter chips)

| State | Light Theme | Dark Theme |
|---|---|---|
| **Default** | `bg-surface-elevated` bg, `border-subtle` border, `text-secondary` text | `bg-surface-elevated` bg, `border-subtle` border, `text-secondary` text |
| **Hover** | `bg-surface-hover` bg | `bg-surface-hover` bg |
| **Active/Pressed** | `bg-accent` bg, `#FFFFFF` text, `bg-accent` border | `bg-accent` bg, `#FFFFFF` text, `bg-accent` border |
| **Focus** | `ring-accent` outline (2px, offset 2px) | `ring-accent` outline (2px, offset 2px) |

### Cards (project cards, session cards)

| State | Light Theme | Dark Theme |
|---|---|---|
| **Default** | `bg-surface-elevated` bg, `border-subtle` border, 4px radius | `bg-surface-elevated` bg, `border-subtle` border, 4px radius |
| **Hover** | `bg-surface-hover` bg, `ring-accent` border, translateY(-2px) | `bg-surface-hover` bg, `ring-accent` border, translateY(-2px) |
| **Focus** | `ring-accent` outline (2px, offset 2px) | `ring-accent` outline (2px, offset 2px) |

### Table Rows (list view)

| State | Light Theme | Dark Theme |
|---|---|---|
| **Default** | `bg-surface-elevated` bg, `border-subtle` border | `bg-surface-elevated` bg, `border-subtle` border |
| **Hover** | `bg-surface-hover` bg, no transform | `bg-surface-hover` bg, no transform |
| **Focus** | `ring-accent` outline (2px, offset 2px) | `ring-accent` outline (2px, offset 2px) |

### Inputs (search)

| State | Light Theme | Dark Theme |
|---|---|---|
| **Default** | `bg-surface` bg, `border-subtle` border, pill shape | `bg-surface` bg, `border-subtle` border, pill shape |
| **Focus** | `ring-accent` border | `ring-accent` border |
| **Placeholder** | `text-tertiary` | `text-tertiary` |

### Date Chips

Same state pattern as buttons. Active chip uses `bg-accent` fill with white text.

---

## 5. View Switcher Pattern

Segmented two-button control replacing the single mystery toggle.

```
┌─────────────────────────┐
│ [icon:grid] [icon:list] │   ← shared rounded container
└─────────────────────────┘
```

- **Container**: `bg-surface-elevated` background, `border-subtle` border, 8px border-radius, 4px internal padding, inline-flex
- **Icons**: Inline SVG (no icon font/CDN). Grid = 3×3 dots pattern, List = 3 horizontal lines pattern
- **Active button**: `bg-accent` fill, white icon, 6px border-radius
- **Inactive button**: transparent background, `text-secondary` icon
- **Accessibility**: `aria-pressed` on each button, `aria-label` on each, focusable via keyboard
- **State persistence**: `localStorage` key `cclog:index:viewMode` (preserve existing behavior)

---

## 6. Toolbar Pattern

Single-row toolbar at desktop (≥1024px):

```
[Search input (left)]  [Date chips (center)]  [View switcher (right)]
```

- **Container**: flex row, `justify-content: space-between`, `align-items: center`, 16px gap
- **Wrap behavior**: At <1024px, wraps to two rows: search + view-switcher on top, date chips below
- **Spacing**: 12px gap between date chips, 8px gap between search and adjacent element

---

## 7. Card Pattern (Grid View)

```
┌──────────────────────────────┐
│ Project Name (bold, 18px)    │  ← display_name
│ -encoded-path (11px, muted)  │  ← truncated, title=tooltip
│                              │
│ Last activity: 2h ago        │  ← labeled (was unlabeled)
│                              │
│ Sessions   Messages   Tokens │  ← labeled stats row
│   12        340       3.4k   │
└──────────────────────────────┘
```

- Uniform height via flex column with `height: 100%` on cards in grid
- Path truncation: `overflow: hidden; text-overflow: ellipsis; white-space: nowrap`
- Full path available via `title` attribute on path element

---

## 8. Table Pattern (List View)

Project column shows two-line cell:

```
Project Name          (semibold, 13px, text-primary)
/encoded/path/here... (muted, 11px, mono, truncated, title=tooltip)
```

Other columns (Last activity, Sessions, Messages, Tokens) remain single-line with sortable headers.

---

## 9. Dark Theme

Dark theme uses the same token structure as light theme, with values mapped per the palette table above. Theme toggle persists via `localStorage` key `cclog-theme` (existing behavior preserved). System preference (`prefers-color-scheme`) used as fallback when no stored preference.

---

## 10. Self-Containment

- No external font CDNs — fonts are bundled or fall back to system fonts
- No external icon libraries — icons are inline SVG
- No external CSS frameworks beyond the compiled Tailwind output
- No external JS dependencies
