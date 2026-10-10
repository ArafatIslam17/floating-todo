# Floating Todo — UI/UX Design Specification

Requirements: [PRD](PRD.md) FR-01 – FR-11. Flows: [APP_FLOW](APP_FLOW.md).

## 1. Design principles

1. **Time is the content.** The interface exists to show what is next and how long remains. Every scheduled row carries its own *fuse* — the one bold element of the design (§4).
2. **Small and quiet until it matters.** Neutral, translucent, low-contrast chrome; colour is reserved for state.
3. **Never colour alone.** Every temporal state has colour **and** an icon **and** words.
4. **One gesture, one result.** Enter adds. Space completes. Esc backs out. No modal for ordinary work.
5. **Honest feedback.** Errors say what failed and what to do; nothing pretends to work.

Rejected defaults (and why): stacked SaaS cards with uniform shadows (wastes width in a 320 px panel), gradient washes (fight text legibility on a translucent surface), all-caps eyebrow labels and dotted meta strings (template chrome; replaced by plain sentence-case words), large headings and charts (not a dashboard).

## 2. Tokens

Defined once at the top of `src/styles/app.css` as CSS custom properties and exposed to Tailwind through `@theme`.

### Colour
| Token | Value | Use |
|---|---|---|
| `--ink` | `#0E1F3A` | Titles, primary text (deep navy rather than black, to sit in the blue family) |
| `--ink-muted` | `#55667F` | Metadata, times |
| `--ink-faint` | `#8696AD` | Placeholders, disabled |
| `--surface` | `rgb(243 247 252 / var(--surface-alpha))` | Panel; alpha = opacity setting (0.85–1.00, default 0.94) |
| `--surface-raised` | `rgb(255 255 255 / 0.72)` | Rows on hover/focus, editor, inputs |
| `--hairline` | `rgb(14 31 58 / 0.10)` | Borders, dividers |
| `--accent` | `#2456D6` | Interactive elements, focus ring, upcoming fuse |
| `--accent-soft` | `rgb(36 86 214 / 0.10)` | Selected/hover fills |
| `--soon` | `#94590A` | Due-soon text and fuse |
| `--overdue` | `#B83A26` | Overdue text, fuse, banner edge |
| `--overdue-soft` | `rgb(184 58 38 / 0.09)` | Overdue row tint |
| `--done` | `#1F7A52` | Completion check |

Contrast on `#F3F7FC`, computed with the WCAG 2 formula: ink 15.3:1, ink-muted 5.4:1, accent 5.8:1, soon 5.3:1, overdue 5.3:1, done 4.9:1 (all ≥ 4.5:1). `--ink-faint` (2.8:1) is used only for placeholder text and disabled controls, which WCAG exempts. Translucency trades against contrast: over a pure-black desktop, secondary text stays ≥ 4.6:1 at the default 0.94 opacity but falls to about 3.7–3.9:1 at the 0.85 minimum while titles stay near 10.8:1 (`--done` is used only for the check icon, which needs 3:1). That is why the range stops at 0.85 rather than going lower.

### Type
Manrope Variable, bundled. `font-variant-numeric: tabular-nums` for all times and countdowns.

| Token | Size / line | Weight | Use |
|---|---|---|---|
| `--text-title` | 13.5 / 18 px | 600 | Task titles |
| `--text-body` | 13 / 18 px | 500 | Inputs, banner text |
| `--text-meta` | 11.5 / 15 px | 500 | Time, countdown, counts |
| `--text-small` | 11 / 14 px | 600 | Buttons in banners, toast |
| `--text-head` | 12.5 / 16 px | 700 | Header name |

Respects OS text scaling: sizes are `rem`-based with `html { font-size: 100% }`; layout reflows rather than clips.

### Space, radius, depth, motion, layers
| Group | Tokens |
|---|---|
| Space (4-px grid) | 2, 4, 6, 8, 10, 12, 16, 20 |
| Radius (by hierarchy, not uniform) | panel 16, row 10, input 9, chip 6, check 5 |
| Border | 1 px `--hairline`; focus ring 2 px `--accent` + 2 px offset in `--surface` |
| Shadow | panel: `0 12px 32px -12px rgb(14 31 58 / .35), 0 1px 0 rgb(255 255 255 / .6) inset`; rows: none |
| Blur | `backdrop-filter: blur(18px) saturate(1.4)` on the panel (blurs content under overlays; desktop blur only with optional Acrylic) |
| Motion | `--dur-fast 120ms`, `--dur-base 180ms`, `--dur-slow 320ms`; easing `cubic-bezier(.2,.7,.2,1)` |
| Layers | base 0, row-actions 10, editor 20, banner 30, toast 40, settings sheet 50 |

## 3. Layout

Width target 300–380 px; works from 280 px.

```text
┌──────────────────────────────────────────┐
│ ◔ Floating Todo   4 open, 1 overdue  ⌄ ⚙ ×│  header (drag region) 40px
├──────────────────────────────────────────┤
│▌ Submit DSP lab report — overdue banner   │  alert banner (only when firing)
│   Complete   Snooze 10 min   Dismiss      │
├──────────────────────────────────────────┤
│ ☐ Tutor Rafi — chapter 5          ✎ 🗑   │  row: check · title · actions on hover/focus
│ ▌▌▌▌▌▌▌▌▌▌▌▌──── Today 17:30  in 25 min   │       fuse + time + countdown
│ ☐ Reply to supervisor                     │  unscheduled row: no fuse
│ ☐ Send attendance sheet                   │
│   Tomorrow 10:00  in 16 hr                │
├──────────────────────────────────────────┤
│ [ Add a task…                    ◷ ][ + ]│  quick-add 44px
│ ▸ Completed (3)                        ◢ │  footer + resize grip
└──────────────────────────────────────────┘
```

Compact mode = header + the most urgent row (alert banner still appears above it).

The header counts open and overdue tasks (a task in its *Due now* minute already counts as overdue). When the panel is narrower than 380 px and something is overdue, the product name is hidden so the counts never truncate. Text is left aligned throughout; numbers right-aligned in their slot.

Sorting: overdue (oldest first) → scheduled ascending → unscheduled (newest first). Completed section: most recently completed first, collapsed by default.

## 4. The fuse

A 2-px line under each scheduled row's title, spanning the row width.

| State | Fuse | Icon | Text |
|---|---|---|---|
| Upcoming (> 60 min) | full width in a neutral tone, small `--accent` head dot | `Clock` | "Today 17:30" and "in 3 hr" as two separate spans |
| Within 60 min | width = remaining/60 min, `--accent`; `--soon` within 30 min | `Clock` / `AlarmClock` | "in 25 min" |
| Due now (first minute) | zero width, pulsing head (static if reduced motion) | `BellRing` | "Due now" |
| Overdue | full width, `--overdue`, dashed | `TriangleAlert` | "Overdue 12 min" |
| Snoozed | as overdue + `BellOff` | `BellOff` | "Snoozed until 17:45" |
| Completed | none | filled check | struck title, muted |
| No time | none | none | nothing |

Width changes only on the minute refresh, with a CSS width transition — nothing animates continuously.

## 5. Components

```text
App
└─ AppProvider (backend connection, snapshot, events)
   └─ Widget (panel surface, widget-wide shortcuts, compact sizing)
      ├─ Header (drag region, counts, compact toggle, settings, hide)
      ├─ IssueBanner (audio / notification / shortcut / window problems)
      ├─ AlertBanner (active alerts; Complete · Snooze split button · Dismiss)
      ├─ task list
      │  └─ TaskRow (checkbox, title, time, countdown, fuse, row actions)
      │     └─ TaskEditor (inline, replaces the row; TaskFields)
      ├─ EmptyState
      ├─ completed section (toggle + TaskRows)
      ├─ QuickAdd (input, details toggle, TaskFields, Add)
      ├─ resize grip
      ├─ UndoToast
      └─ SettingsSheet
   DatabaseErrorScreen replaces the list when storage cannot open.
```

| Component | States |
|---|---|
| TaskRow | default, hover, focus-visible, editing, overdue, snoozed, completed, entering, leaving |
| Checkbox | unchecked, checked, focus, disabled while saving |
| QuickAdd | empty, typing, details open, invalid (message under field, `aria-invalid`), saving |
| AlertBanner | single alert, multiple ("3 tasks need attention", list of up to 3), snooze menu open |
| IssueBanner | info, warning, error; dismissible except database |
| SettingsSheet | default, shortcut capture, test-sound playing, error inline per control |

Buttons are icon-only only where the icon is unambiguous (settings, hide, edit, delete, add); each has `aria-label` and a native `title` tooltip.

## 6. States

| Situation | Content |
|---|---|
| No tasks | "Nothing on your list. Type a task below and press Enter." |
| Tasks but none scheduled | List as normal; header reads "3 open" |
| All completed | "All done. Completed tasks are below." |
| Loading (first snapshot) | Skeleton of 3 row outlines, no spinner; usually < 100 ms |
| Overdue | Row state per §4 + header count "1 overdue" in `--overdue` with icon |
| Invalid title | "Enter a task title." |
| Invalid time | "That time doesn't exist on this date (clock change). Choose another time." / "Pick a date for this time." |
| Database error | Full panel: "Can't open your task database." + message + path + Retry |
| Command failed | Inline under the control: "Couldn't save: <reason>." Input preserved |
| Notification failure | Banner: "Desktop notifications aren't available: <reason>. Alerts still appear here." |
| Audio failure | Banner: "Couldn't play the alert sound: <reason>. Check your output device." |
| Shortcut conflict | In Settings, under the field: "Ctrl+Alt+K couldn't be registered — another app may be using it (…). Choose a different shortcut." The previous shortcut stays active. |

## 7. Motion
- CSS: hover/focus colour (120 ms), fuse width (320 ms), sheet slide (180 ms).
- GSAP (`src/lib/motion.ts`): row enter (height 0→auto + fade, 180 ms), row leave on complete/delete (fade + collapse, 180 ms), alert banner arrival (one 2-step nudge). These need height-to-auto and sequencing, which is why GSAP is used rather than CSS.
- `prefers-reduced-motion: reduce` → GSAP durations 0, pulse disabled, transitions ≤ 1 ms.
- No idle animation anywhere.

## 8. Accessibility
- Full keyboard operation (PRD FR-10); roving focus in the task list (`↑`/`↓`). Row checkboxes and row buttons are taken out of the tab order and driven by row keys (Space, E, Delete) so Tab moves between regions instead of through every control.
- `role="list"`/`listitem`; checkbox is a native `<input type="checkbox">` with the task title as label.
- Alert banner is `role="alert"` (announced once); counts use `aria-live="polite"`.
- Focus ring always visible on keyboard focus (`:focus-visible`).
- Targets ≥ 28×28 px (row actions), ≥ 32 px for primary buttons — appropriate for mouse/trackpad desktop use.
- Urgency always has icon + text in addition to colour.

## 9. Design decisions
- **Light only in v1.** The brief asks for a light translucent look; a dark theme is a v2 item rather than a half-tuned variant.
- **Opacity affects the surface, not the text**, and its range is capped at 0.85 so titles remain high-contrast on any desktop.
- **Acrylic optional.** CSS blur only blurs the panel's own layers; true desktop blur needs Windows Acrylic, which lags on Windows 10, so it is a setting.
- **No category UI** per PRD; sorting by time does the organising.
